use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Deserialize;

use crate::error::Error;

use super::SessionService;

// Client ID of the official Web Spotify front-end.
const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

// All scopes we could possibly require.
const ACCESS_SCOPES: &str = "streaming,user-read-email,user-read-private,playlist-read-private,playlist-read-collaborative,playlist-modify-public,playlist-modify-private,user-follow-modify,user-follow-read,user-library-read,user-library-modify,user-top-read,user-read-recently-played";

// Consider token expired even before the official expiration time.  Spotify
// seems to be reporting excessive token TTLs so let's cut it down by 30
// minutes.
const EXPIRATION_TIME_THRESHOLD: Duration = Duration::from_secs(60 * 30);

#[derive(Clone)]
pub struct AccessToken {
    pub token: String,
    pub expires: Instant,
}

impl AccessToken {
    fn expired() -> Self {
        Self {
            token: String::new(),
            expires: Instant::now(),
        }
    }

    pub fn request(session: &SessionService) -> Result<Self, Error> {
        #[derive(Deserialize)]
        struct MercuryAccessToken {
            #[serde(rename = "expiresIn")]
            expires_in: u64,
            #[serde(rename = "accessToken")]
            access_token: String,
        }

        let token: MercuryAccessToken = session.connected()?.get_mercury_json(format!(
            "hm://keymaster/token/authenticated?client_id={}&scope={}",
            CLIENT_ID, ACCESS_SCOPES
        ))?;

        Ok(Self {
            token: token.access_token,
            expires: Instant::now() + Duration::from_secs(token.expires_in),
        })
    }

    fn is_expired(&self) -> bool {
        self.expires.saturating_duration_since(Instant::now()) < EXPIRATION_TIME_THRESHOLD
    }
}

pub struct TokenProvider {
    token: Mutex<AccessToken>,
}

impl TokenProvider {
    pub fn new() -> Self {
        Self {
            token: Mutex::new(AccessToken::expired()),
        }
    }

    pub fn get(&self, session: &SessionService) -> Result<AccessToken, Error> {
        let mut token = self.token.lock();
        if token.is_expired() {
            log::info!("access token expired, requesting");
            *token = AccessToken::request(session)?;
        }
        Ok(token.clone())
    }
}
