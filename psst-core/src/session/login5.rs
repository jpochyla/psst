// Ported from librespot

use crate::error::Error;
use crate::session::client_token::ClientTokenProvider;
use crate::session::token::Token;
use crate::session::token::TokenType::AuthToken;
use crate::session::SessionService;
use crate::util::{default_ureq_agent_builder, solve_hash_cash};
use librespot_protocol::login5::login_response::Response;
use librespot_protocol::{
    client_info::ClientInfo,
    credentials::{StoredCredential},
    hashcash::HashcashSolution,
    login5::{
        login_request::Login_method, ChallengeSolution, LoginError, LoginRequest,
        LoginResponse,
    },
};
use parking_lot::Mutex;
use protobuf::well_known_types::duration::Duration as ProtoDuration;
use protobuf::{Message, MessageField};
use std::fmt::Formatter;
use std::time::{Duration, Instant};
use std::{error, fmt, thread};
use crate::system_info::{CLIENT_ID, DEVICE_ID};

const MAX_LOGIN_TRIES: u8 = 3;
const LOGIN_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug)]
enum Login5Error {
    FaultyRequest(LoginError),
    CodeChallenge,
    NoStoredCredentials,
    RetriesFailed(u8),
}

impl error::Error for Login5Error {}

impl fmt::Display for Login5Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Login5Error::FaultyRequest(e) => {
                write!(f, "Login request was denied {:?}", e)
            }
            Login5Error::CodeChallenge => {
                write!(f, "Login5 code challenge is not supported")
            }
            Login5Error::NoStoredCredentials => {
                write!(f, "Tried to acquire access token without stored credentials")
            }
            Login5Error::RetriesFailed(u8) => {
                write!(f, "Couldn't successfully authenticate after {:?} times", u8)
            }
        }
    }
}

impl From<Login5Error> for Error {
    fn from(err: Login5Error) -> Self {
        match err {
            Login5Error::NoStoredCredentials | Login5Error::RetriesFailed(_) | Login5Error::FaultyRequest(_) => {
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

pub struct Login5 {
    auth_token: Mutex<Option<Token>>,
    client_token_provider: ClientTokenProvider,
    agent: ureq::Agent,
}

impl Login5 {
    /// Login5 instances can be used to cache and retrieve access tokens from stored credentials.
    ///
    /// # Arguments
    ///
    /// * `client_token_provider`: Can be optionally injected to control which client-id is
    ///   used for it.
    ///
    /// returns: Login5
    pub fn new(client_token_provider: Option<ClientTokenProvider>, proxy_url: Option<&str>) -> Self {
        Self {
            auth_token: Mutex::new(None),
            client_token_provider: client_token_provider.unwrap_or_else(|| ClientTokenProvider::new(proxy_url)),
            agent: default_ureq_agent_builder(proxy_url).build().into(),
        }
    }

    fn request(&self, message: &LoginRequest) -> Result<Vec<u8>, Error> {
        let client_token: String = self.client_token_provider.get()?;
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

    fn request_new_access_token(&self, login: Login_method) -> Result<Token, Error> {
        let mut login_request = LoginRequest {
            client_info: MessageField::some(ClientInfo {
                client_id: String::from(CLIENT_ID),
                device_id: String::from(DEVICE_ID),
                special_fields: Default::default(),
            }),
            login_method: Some(login),
            ..Default::default()
        };

        let mut response = self.request(&login_request)?;
        let mut count = 0;

        loop {
            count += 1;

            let message = LoginResponse::parse_from_bytes(&response)?;
            log::debug!("Login5 attempt responded with {message:?}");

            if let Some(Response::Ok(ok)) = message.response {
                return Ok(Token {
                    access_token: ok.access_token,
                    expires_in: Duration::from_secs(ok.access_token_expires_in.try_into().unwrap_or(3600)),
                    token_type: AuthToken,
                    token_type_s: "Bearer".to_string(),
                    scopes: vec![],
                    timestamp: Instant::now(),
                });
            }

            if message.has_error() {
                match message.error() {
                    LoginError::TIMEOUT | LoginError::TOO_MANY_ATTEMPTS => {
                        log::debug!("Too many login5 requests... timeout!");
                        thread::sleep(LOGIN_TIMEOUT)
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
                response = self.request(&login_request)?;
            } else {
                return Err(Login5Error::RetriesFailed(MAX_LOGIN_TRIES).into());
            }
        }
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
            let duration = solve_hash_cash(
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

    /// Retrieve an `access_token` via Login5. The token is either requested first (slow), or
    /// retrieved from local cache (fast).
    ///
    /// This request will only work if the session already has valid credentials available.
    /// The client-id of the credentials have to match the client-id used to retrieve
    /// the client token (see also `Login5::new(...)`). For example, if you previously generated
    /// stored credentials with an android client-id, they won't work within login5 using a desktop
    /// client-id.
    pub fn get_access_token(
        &self,
        session: &SessionService,
    ) -> Result<Token, Error> {
        let mut cur_token = self.auth_token.lock();

        let login_creds = session.config.lock().as_ref().unwrap().login_creds.clone();
        let auth_data = login_creds.auth_data.clone();
        if auth_data.is_empty() {
            return Err(Login5Error::NoStoredCredentials.into());
        }

        if let Some(auth_token) = &*cur_token {
            if !auth_token.is_expired() {
                return Ok(auth_token.clone());
            }

            *cur_token = None;
            log::debug!("Auth token expired");
        }


        log::debug!("Requesting new auth token");

        // Conversion from psst protocol structs to librespot protocol structs
        let method = Login_method::StoredCredential(StoredCredential {
            username: login_creds.username.clone().unwrap(),
            data: auth_data,
            ..Default::default()
        });

        let new_token = self.request_new_access_token(method)?;

        log::debug!("Successfully requested new auth token");

        *cur_token = Some(new_token.clone());
        Ok(new_token)
    }
}
