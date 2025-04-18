use crate::error::Error;
use crate::oauth::listen_for_callback_parameter;
use rustfm_scrobble::{responses::SessionResponse, Scrobble, Scrobbler, ScrobblerError};
use std::{net::SocketAddr, time::Duration};
use url::Url;

pub struct LastFmClient;

impl LastFmClient {
    /// Report a track as "now playing" to Last.fm using an existing Scrobbler instance.
    pub fn now_playing_song(
        scrobbler: &Scrobbler, // Requires an authenticated Scrobbler
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        let song = Scrobble::new(artist, title, album.unwrap_or(""));
        scrobbler
            .now_playing(&song)
            .map(|_| ())
            .map_err(Error::from)
    }

    /// Scrobble a finished track to Last.fm using an existing Scrobbler instance.
    pub fn scrobble_song(
        scrobbler: &Scrobbler, // Requires an authenticated Scrobbler
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        let song = Scrobble::new(artist, title, album.unwrap_or(""));
        scrobbler.scrobble(&song).map(|_| ()).map_err(Error::from)
    }

    /// Creates an authenticated Last.fm Scrobbler instance with provided credentials.
    /// Note: This assumes the session_key is valid. Validity is checked on first API call.
    pub fn create_scrobbler(
        api_key: Option<&str>,
        api_secret: Option<&str>,
        session_key: Option<&str>,
    ) -> Result<Scrobbler, Error> {
        let (Some(api_key), Some(api_secret), Some(session_key)) =
            (api_key, api_secret, session_key)
        else {
            log::warn!("missing Last.fm API key, secret, or session key for scrobbler creation.");
            return Err(Error::ConfigError(
                "Missing Last.fm API key, secret, or session key.".to_string(),
            ));
        };

        let mut scrobbler = Scrobbler::new(api_key, api_secret);
        // Associate the session key with the scrobbler instance.
        scrobbler.authenticate_with_session_key(session_key);
        log::info!("scrobbler instance created with session key (validity checked on first use).");
        Ok(scrobbler)
    }
}

impl From<ScrobblerError> for Error {
    fn from(value: ScrobblerError) -> Self {
        Self::ScrobblerError(Box::new(value))
    }
}

/// Generate a Last.fm authentication URL
pub fn generate_lastfm_auth_url(
    api_key: &str,
    callback_url: &str,
) -> Result<String, url::ParseError> {
    let base = "http://www.last.fm/api/auth/";
    let url = Url::parse_with_params(base, &[("api_key", api_key), ("cb", callback_url)])?;
    Ok(url.to_string())
}

/// Exchange a token for a Last.fm session key
pub fn exchange_token_for_session(
    api_key: &str,
    api_secret: &str,
    token: &str,
) -> Result<String, Error> {
    let mut scrobbler = Scrobbler::new(api_key, api_secret);
    scrobbler
        .authenticate_with_token(token) // Uses auth.getSession API call internally
        .map(|response: SessionResponse| response.key) // Extract the session key string
        .map_err(Error::from) // Map ScrobblerError to crate::error::Error
}

/// Listen for a Last.fm token from the callback
pub fn get_lastfm_token_listener(
    socket_address: SocketAddr,
    timeout: Duration,
) -> Result<String, Error> {
    // Use the shared listener function, specifying "token" as the parameter
    listen_for_callback_parameter(socket_address, timeout, "token")
}
