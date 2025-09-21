// Ported from librespot

use crate::error::Error;
use crate::session::spclient::SpClient;
use crate::session::token::Token;
use crate::session::token::TokenType::AuthToken;
use crate::session::SessionService;
use crate::util::default_ureq_agent_builder;
use byteorder::{BigEndian, ByteOrder};
use librespot_protocol::login5::login_response::Response;
use librespot_protocol::{
    client_info::ClientInfo,
    credentials::{Password, StoredCredential},
    hashcash::HashcashSolution,
    login5::{
        login_request::Login_method, ChallengeSolution, LoginError, LoginOk, LoginRequest,
        LoginResponse,
    },
};
use parking_lot::Mutex;
use protobuf::well_known_types::duration::Duration as ProtoDuration;
use protobuf::{Message, MessageField};
use sha1::{Digest, Sha1};
use std::fmt::Formatter;
use std::ops::Deref;
use std::time::{Duration, Instant};
use std::{error, fmt};
use ureq::Body;

const MAX_LOGIN_TRIES: u8 = 3;
const LOGIN_TIMEOUT: Duration = Duration::from_secs(3);

/// Client ID for desktop keymaster client
pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

// Device ID used for authentication message.
const DEVICE_ID: &str = "Psst";

#[derive(Debug)]
enum Login5Error {
    FaultyRequest(LoginError),
    CodeChallenge,
    NoStoredCredentials,
    RetriesFailed(u8),
    OnlyForMobile,
}

impl error::Error for Login5Error {}

impl fmt::Display for Login5Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Login5Error::FaultyRequest(e) => {
                write!(f, "Login request was denied {:?}", e)
            }
            Login5Error::CodeChallenge => {
                write!(f, "Code challenge is not supported")
            }
            Login5Error::NoStoredCredentials => {
                write!(f, "Tried to acquire token without stored credentials")
            }
            Login5Error::RetriesFailed(u8) => {
                write!(f, "Couldn't successfully authenticate after {:?} times", u8)
            }
            Login5Error::OnlyForMobile => {
                write!(f, "Login via login5 is only allowed for android or ios")
            }
        }
    }
}

impl From<Login5Error> for Error {
    fn from(err: Login5Error) -> Self {
        match err {
            Login5Error::NoStoredCredentials => Error::InvalidStateError(err.into()),
            Login5Error::OnlyForMobile => Error::UnimplementedError(err.into()),
            Login5Error::RetriesFailed(_) | Login5Error::FaultyRequest(_) => {
                Error::InvalidStateError(err.into())
            }
            Login5Error::CodeChallenge => Error::UnimplementedError(err.into()),
        }
    }
}

#[derive(Debug)]
pub struct Login5Request {
    pub uri: String,
    pub method: String,
    pub payload: Vec<Vec<u8>>,
}

pub struct Login5Manager {
    auth_token: Mutex<Option<Token>>,
    agent: ureq::Agent,
}

impl Login5Manager {
    pub fn new(proxy_url: Option<&str>) -> Self {
        Self {
            auth_token: Mutex::new(None),
            agent: default_ureq_agent_builder(proxy_url).build().into(),
        }
    }

    fn request(&self, spclient: &SpClient, message: &LoginRequest) -> Result<Vec<u8>, Error> {
        //self.session().spclient().client_token().await?;
        let client_token: String = spclient.client_token()?;
        let body = message.write_to_bytes()?;

        let mut response = self
            .agent
            .post("https://login5.spotify.com/v3/login")
            .header("Accept", "application/x-protobuf")
            .header("client-token", &client_token)
            .send(body)?;

        let vec = response.body_mut().read_to_vec()?;
        Ok(vec)
    }

    fn login5_request(&self, spclient: &SpClient, login: Login_method) -> Result<LoginOk, Error> {
        let mut login_request = LoginRequest {
            client_info: MessageField::some(ClientInfo {
                client_id: String::from(CLIENT_ID),
                device_id: String::from(DEVICE_ID),
                special_fields: Default::default(),
            }),
            login_method: Some(login),
            ..Default::default()
        };

        let mut response = self.request(spclient, &login_request)?;
        let mut count = 0;

        loop {
            count += 1;

            let message = LoginResponse::parse_from_bytes(&response)?;
            log::debug!("Login5 attempt responded with {message:?}");

            if let Some(Response::Ok(ok)) = message.response {
                break Ok(ok);
            }

            if message.has_error() {
                match message.error() {
                    LoginError::TIMEOUT | LoginError::TOO_MANY_ATTEMPTS => {
                        log::debug!("Too many login5 requests... timeout!");

                        // TODO: timeout of 3 seconds
                    }
                    others => {
                        log::debug!("Login5 request failed!");

                        return Err(Login5Error::FaultyRequest(others).into());
                    }
                }
            }

            if message.has_challenges() {
                // handles the challenges, and updates the login context with the response
                Self::handle_challenges(&mut login_request, message)?;
            }

            if count < MAX_LOGIN_TRIES {
                response = self.request(spclient, &login_request)?;
            } else {
                return Err(Login5Error::RetriesFailed(MAX_LOGIN_TRIES).into());
            }
        }
    }

    /// Login for android and ios
    ///
    /// This request doesn't require a connected session as it is the entrypoint for android or ios
    ///
    /// This request will only work when:
    /// - client_id => android or ios | can be easily adjusted in [SessionConfig::default_for_os]
    /// - user-agent => android or ios | has to be adjusted in [HttpClient::new](crate::http_client::HttpClient::new)
    pub fn login(
        &self,
        id: impl Into<String> + fmt::Debug,
        password: impl Into<String> + fmt::Debug,
    ) -> Result<(Token, Vec<u8>), Error> {
        log::debug!("Wanting to log in with {id:?} and {password:?}");
        Err(Login5Error::OnlyForMobile.into())
        /*
        if !matches!(OS, "android" | "ios") {
            // by manipulating the user-agent and client-id it can be also used/tested on desktop
            return Err(Login5Error::OnlyForMobile.into());
        }

        let method = Login_method::Password(Password {
            id: id.into(),
            password: password.into(),
            ..Default::default()
        });

        let token_response = self.login5_request(method)?;
        let auth_token = Self::token_from_login(
            token_response.access_token,
            token_response.access_token_expires_in,
        );

        Ok((auth_token, token_response.stored_credential))
         */
    }

    /// Retrieve the access_token via login5
    ///
    /// This request will only work when the store credentials match the client-id. Meaning that
    /// stored credentials generated with the keymaster client-id will not work, for example, with
    /// the android client-id.
    pub fn auth_token(
        &self,
        session: &SessionService,
        spclient: &SpClient,
    ) -> Result<Token, Error> {
        let mut cur_token = self.auth_token.lock();

        let login_creds = session.config.lock().as_ref().unwrap().login_creds.clone();
        let auth_data = login_creds.auth_data.clone();
        if auth_data.is_empty() {
            return Err(Login5Error::NoStoredCredentials.into());
        }

        if let Some(auth_token) = &*cur_token {
            // auth token expired check
            return Ok(auth_token.clone());
        }

        let method = Login_method::StoredCredential(StoredCredential {
            username: login_creds.username.clone().unwrap(),
            data: auth_data,
            ..Default::default()
        });

        let token_response = self.login5_request(spclient, method)?;
        let auth_token = Self::token_from_login(
            token_response.access_token,
            token_response.access_token_expires_in,
        );

        *cur_token = Some(auth_token);

        log::trace!("Got auth token: {:?}", self.auth_token);

        (*cur_token)
            .clone()
            .ok_or(Login5Error::NoStoredCredentials.into())
    }

    fn handle_challenges(
        login_request: &mut LoginRequest,
        message: LoginResponse,
    ) -> Result<(), Error> {
        let challenges = message.challenges();
        log::debug!(
            "Received {} challenges, solving...",
            challenges.challenges.len()
        );

        for challenge in &challenges.challenges {
            if challenge.has_code() {
                return Err(Login5Error::CodeChallenge.into());
            } else if !challenge.has_hashcash() {
                log::debug!("Challenge was empty, skipping...");
                continue;
            }

            let hash_cash_challenge = challenge.hashcash();

            let mut suffix = [0u8; 0x10];
            let duration = Login5Manager::solve_hash_cash(
                &message.login_context,
                &hash_cash_challenge.prefix,
                hash_cash_challenge.length,
                &mut suffix,
            )?;

            let (seconds, nanos) = (duration.as_secs() as i64, duration.subsec_nanos() as i32);
            log::debug!("Solving hashcash took {seconds}s {nanos}ns");

            let mut solution = ChallengeSolution::new();
            solution.set_hashcash(HashcashSolution {
                suffix: Vec::from(suffix),
                duration: MessageField::some(ProtoDuration {
                    seconds,
                    nanos,
                    ..Default::default()
                }),
                ..Default::default()
            });

            login_request
                .challenge_solutions
                .mut_or_insert_default()
                .solutions
                .push(solution);
        }

        login_request.login_context = message.login_context;

        Ok(())
    }

    fn token_from_login(token: String, expires_in: i32) -> Token {
        Token {
            access_token: token,
            expires_in: Duration::from_secs(expires_in.try_into().unwrap_or(3600)),
            token_type: AuthToken,
            token_type_s: "Bearer".to_string(),
            scopes: vec![],
            timestamp: Instant::now(),
        }
    }

    // TODO: move solve_hash_cash to a better place
    pub fn solve_hash_cash(
        ctx: &[u8],
        prefix: &[u8],
        length: i32,
        dst: &mut [u8],
    ) -> Result<Duration, Error> {
        // after a certain number of seconds, the challenge expires
        const TIMEOUT: u64 = 5; // seconds
        let now = Instant::now();

        let md = Sha1::digest(ctx);

        let mut counter: i64 = 0;
        let target: i64 = BigEndian::read_i64(&md[12..20]);

        let suffix = loop {
            if now.elapsed().as_secs() >= TIMEOUT {
                return Err(Error::InvalidStateError(
                    format!("{TIMEOUT} seconds expired").into(),
                ));
            }

            let suffix = [(target + counter).to_be_bytes(), counter.to_be_bytes()].concat();

            let mut hasher = Sha1::new();
            hasher.update(prefix);
            hasher.update(&suffix);
            let md = hasher.finalize();

            if BigEndian::read_i64(&md[12..20]).trailing_zeros() >= (length as u32) {
                break suffix;
            }

            counter += 1;
        };

        dst.copy_from_slice(&suffix);

        Ok(now.elapsed())
    }
}
