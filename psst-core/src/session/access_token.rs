// Keymaster token acquisition is deprecated in Psst.
// OAuth/PKCE is the primary auth path; Keymaster remains only for legacy compatibility.
// See librespot discussion for context (403/code=4, scope restrictions):
// https://github.com/librespot-org/librespot/issues/1532#issuecomment-3188123661
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use serde::Deserialize;

use crate::error::Error;

use super::SessionService;

// Client ID of the official Web Spotify front-end.
pub const CLIENT_ID: &str = "65b708073fc0480ea92a077233ca87bd";

// All scopes we could possibly require.
pub const ACCESS_SCOPES: &str = "user-read-email,user-read-private,playlist-read-private,playlist-read-collaborative,playlist-modify-public,playlist-modify-private,user-follow-modify,user-follow-read,user-library-read,user-library-modify,user-top-read,user-read-recently-played";

// Consider token expired even before the official expiration time.  Spotify
// seems to be reporting excessive token TTLs so let's cut it down by 30
// minutes.
const EXPIRATION_TIME_THRESHOLD: Duration = Duration::from_secs(60 * 30);
// Avoid repeatedly hammering keymaster when errors occur.

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
            #[serde(alias = "accessToken", alias = "access_token")]
            access_token: String,
            #[serde(alias = "expiresIn", alias = "expires_in")]
            expires_in: u64,
        }

        let token: MercuryAccessToken = session.connected()?.get_mercury_json(format!(
            "hm://keymaster/token/authenticated?client_id={CLIENT_ID}&scope={ACCESS_SCOPES}",
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

    pub fn invalidate(&self) {
        let mut token = self.token.lock();
        *token = AccessToken::expired();
    }

    pub fn get(&self, session: &SessionService) -> Result<AccessToken, Error> {
        // Prefer an OAuth bearer if the session provides one.
        if let Some(tok) = session.oauth_bearer() {
            return Ok(AccessToken {
                token: tok,
                // Give the bearer a reasonable lifetime; it will be replaced when refreshed.
                expires: Instant::now() + Duration::from_secs(3600),
            });
        }

        let mut token = self.token.lock();
        if token.is_expired() {
            log::debug!("access token expired, requesting");
            *token = AccessToken::request(session)?;
        }
        Ok(token.clone())
    }
}
