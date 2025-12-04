// Ported from librespot

use crate::error::Error;
use crate::session::token::Token;
use crate::system_info::{CLIENT_ID, DEVICE_ID, OS, SPOTIFY_SEMANTIC_VERSION};
use crate::util::{default_ureq_agent_builder, solve_hash_cash};
use data_encoding::HEXUPPER_PERMISSIVE;
use librespot_protocol::clienttoken_http::{
    ChallengeAnswer, ChallengeType, ClientTokenRequest, ClientTokenRequestType,
    ClientTokenResponse, ClientTokenResponseType,
};
use parking_lot::Mutex;
use protobuf::{Enum, Message};
use std::time::{Duration, Instant};

pub struct ClientTokenProvider {
    token: Mutex<Option<Token>>,
    agent: ureq::Agent,
}

impl ClientTokenProvider {
    pub fn new(proxy_url: Option<&str>) -> Self {
        Self {
            token: Mutex::new(None),
            agent: default_ureq_agent_builder(proxy_url).build().into(),
        }
    }

    fn request<M: Message>(&self, message: &M) -> Result<Vec<u8>, Error> {
        let body = message.write_to_bytes()?;

        let mut response = self
            .agent
            .post("https://clienttoken.spotify.com/v1/clienttoken")
            .header("Accept", "application/x-protobuf")
            .send(body)?;

        let vec = response.body_mut().read_to_vec();
        Ok(vec?)
    }

    fn request_new_token(&self) -> Result<Token, Error> {
        log::debug!("Requesting new token...");

        let mut request = ClientTokenRequest::new();
        request.request_type = ClientTokenRequestType::REQUEST_CLIENT_DATA_REQUEST.into();

        let client_data = request.mut_client_data();

        client_data.client_version = SPOTIFY_SEMANTIC_VERSION.into();
        client_data.client_id = CLIENT_ID.into();

        let connectivity_data = client_data.mut_connectivity_sdk_data();
        connectivity_data.device_id = DEVICE_ID.to_string();

        let platform_data = connectivity_data
            .platform_specific_data
            .mut_or_insert_default();

        let os_version = sysinfo::System::os_version().unwrap_or("0".into());
        let kernel_version = sysinfo::System::kernel_version().unwrap_or_else(|| String::from("0"));

        match OS {
            "windows" => {
                let os_version = os_version.parse::<f32>().unwrap_or(10.) as i32;
                let kernel_version = kernel_version.parse::<i32>().unwrap_or(21370);

                let (pe, image_file) = match std::env::consts::ARCH {
                    "arm" => (448, 452),
                    "aarch64" => (43620, 452),
                    "x86_64" => (34404, 34404),
                    _ => (332, 332), // x86
                };

                let windows_data = platform_data.mut_desktop_windows();
                windows_data.os_version = os_version;
                windows_data.os_build = kernel_version;
                windows_data.platform_id = 2;
                windows_data.unknown_value_6 = 9;
                windows_data.image_file_machine = image_file;
                windows_data.pe_machine = pe;
                windows_data.unknown_value_10 = true;
            }
            "ios" => {
                let ios_data = platform_data.mut_ios();
                ios_data.user_interface_idiom = 0;
                ios_data.target_iphone_simulator = false;
                ios_data.hw_machine = "iPhone14,5".to_string();
                ios_data.system_version = os_version;
            }
            "android" => {
                let android_data = platform_data.mut_android();
                android_data.android_version = os_version;
                android_data.api_version = 31;
                "Pixel".clone_into(&mut android_data.device_name);
                "GF5KQ".clone_into(&mut android_data.model_str);
                "Google".clone_into(&mut android_data.vendor);
            }
            "macos" => {
                let macos_data = platform_data.mut_desktop_macos();
                macos_data.system_version = os_version;
                macos_data.hw_model = "iMac21,1".to_string();
                macos_data.compiled_cpu_type = std::env::consts::ARCH.to_string();
            }
            _ => {
                let linux_data = platform_data.mut_desktop_linux();
                linux_data.system_name = "Linux".to_string();
                linux_data.system_release = kernel_version;
                linux_data.system_version = os_version;
                linux_data.hardware = std::env::consts::ARCH.to_string();
            }
        }

        let mut response = self.request(&request)?;
        let mut count = 0;
        const MAX_TRIES: u8 = 3;

        let token_response = loop {
            count += 1;

            let message = ClientTokenResponse::parse_from_bytes(&response)?;

            match ClientTokenResponseType::from_i32(message.response_type.value()) {
                // depending on the platform, you're either given a token immediately
                // or are presented a hash cash challenge to solve first
                Some(ClientTokenResponseType::RESPONSE_GRANTED_TOKEN_RESPONSE) => {
                    log::debug!("Received a granted token");
                    break message;
                }
                Some(ClientTokenResponseType::RESPONSE_CHALLENGES_RESPONSE) => {
                    log::debug!("Received a hash cash challenge, solving...");

                    let challenges = message.challenges().clone();
                    let state = challenges.state;
                    if let Some(challenge) = challenges.challenges.first() {
                        let hash_cash_challenge = challenge.evaluate_hashcash_parameters();

                        let ctx = vec![];
                        let prefix = HEXUPPER_PERMISSIVE
                            .decode(hash_cash_challenge.prefix.as_bytes())
                            .map_err(|e| {
                                Error::InvalidStateError(
                                    format!("Unable to decode hash cash challenge: {e}").into(),
                                )
                            })?;
                        let length = hash_cash_challenge.length;

                        let mut suffix = [0u8; 0x10];
                        let answer = solve_hash_cash(&ctx, &prefix, length, &mut suffix);

                        match answer {
                            Ok(_) => {
                                // the suffix must be in uppercase
                                let suffix = HEXUPPER_PERMISSIVE.encode(&suffix);

                                let mut answer_message = ClientTokenRequest::new();
                                answer_message.request_type =
                                    ClientTokenRequestType::REQUEST_CHALLENGE_ANSWERS_REQUEST
                                        .into();

                                let challenge_answers = answer_message.mut_challenge_answers();

                                let mut challenge_answer = ChallengeAnswer::new();
                                challenge_answer.mut_hash_cash().suffix = suffix;
                                challenge_answer.ChallengeType =
                                    ChallengeType::CHALLENGE_HASH_CASH.into();

                                challenge_answers.state = state.to_string();
                                challenge_answers.answers.push(challenge_answer);

                                log::trace!("Answering hash cash challenge");
                                match self.request(&answer_message) {
                                    Ok(token) => {
                                        response = token;
                                        continue;
                                    }
                                    Err(e) => {
                                        log::trace!("Answer not accepted {count}/{MAX_TRIES}: {e}");
                                    }
                                }
                            }
                            Err(e) => log::trace!(
                                "Unable to solve hash cash challenge {count}/{MAX_TRIES}: {e}"
                            ),
                        }

                        if count < MAX_TRIES {
                            response = self.request(&request)?;
                        } else {
                            return Err(Error::InvalidStateError(
                                format!("Unable to solve any of {MAX_TRIES} hash cash challenges")
                                    .into(),
                            ));
                        }
                    } else {
                        return Err(Error::InvalidStateError("No challenges found".into()));
                    }
                }

                Some(unknown) => {
                    return Err(Error::UnimplementedError(
                        format!("Unknown client token response type: {unknown:?}").into(),
                    ));
                }
                None => {
                    return Err(Error::InvalidStateError(
                        "No client token response type".into(),
                    ))
                }
            }
        };

        let granted_token = token_response.granted_token();
        let access_token = granted_token.token.to_owned();

        Ok(Token {
            access_token: access_token.clone(),
            expires_in: Duration::from_secs(
                granted_token
                    .refresh_after_seconds
                    .try_into()
                    .unwrap_or(7200),
            ),
            token_type: "client-token".to_string(),
            scopes: granted_token
                .domains
                .iter()
                .map(|d| d.domain.clone())
                .collect(),
            timestamp: Instant::now(),
        })
    }

    pub fn get(&self) -> Result<String, Error> {
        // Check for cached token availability, else retrieve fresh token
        let mut cur_token = self.token.lock();

        if let Some(token) = &*cur_token {
            if !token.is_expired() {
                return Ok(token.access_token.clone());
            }

            *cur_token = None;
            log::debug!("Client token expired");
        }

        let new_token = self.request_new_token()?;

        *cur_token = Some(new_token.clone());
        Ok(new_token.access_token)
    }
}
