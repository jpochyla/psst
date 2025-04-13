extern crate rustfm_scrobble_proxy;
use crate::error::Error;
use crate::oauth::listen_for_callback_parameter;
use rustfm_scrobble_proxy::{responses::SessionResponse, Scrobble, Scrobbler, ScrobblerError};
use std::cell::RefCell;
use std::{net::SocketAddr, time::Duration};
use url::Url;

// Handle Last.fm client as a thread-local singleton
thread_local! {
    static LASTFM_CLIENT: RefCell<Option<Scrobbler>> = const { RefCell::new(None)};
}

pub struct LastFmClient;

impl LastFmClient {
    /// Report a track as "now playing" to Last.fm
    pub fn now_playing_song(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        self.submit_track(artist, title, album, |client, song| {
            client.now_playing(song).map(|_| ())
        })
    }

    /// Scrobble a finished track to Last.fm
    pub fn scrobble_song(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        self.submit_track(artist, title, album, |client, song| {
            client.scrobble(song).map(|_| ())
        })
    }

    /// Helper method to handle common track submission logic
    fn submit_track<F>(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
        f: F,
    ) -> Result<(), Error>
    where
        F: FnOnce(&Scrobbler, &Scrobble) -> Result<(), ScrobblerError>,
    {
        let song = Scrobble::new(artist, title, album);
        LASTFM_CLIENT.with(|client| {
            if let Some(client) = &*client.borrow() {
                f(client, &song)?
            } else {
                log::warn!("LastFmClient is not initialized.");
            }
            Ok(())
        })
    }

    /// Authenticate the Last.fm client with provided credentials
    pub fn authenticate_with_config(
        &mut self,
        api_key: Option<&str>,
        api_secret: Option<&str>,
        session_key: Option<&str>,
    ) -> Result<(), Error> {
        // Use guard clauses for missing parameters
        let (Some(api_key), Some(api_secret), Some(session_key)) =
            (api_key, api_secret, session_key) else {
            log::warn!("Missing Last.fm API key, secret, or session key.");
            return Err(Error::ConfigError("Missing Last.fm API key, secret, or session key.".to_string()));
        };

        let mut scrobbler = Scrobbler::new(api_key, api_secret);

        // Call authenticate_with_session_key - it returns () and doesn't indicate immediate error
        scrobbler.authenticate_with_session_key(session_key);

        // Assume success for now and store the client (as per docs, errors detected later)
        LASTFM_CLIENT.with(|client| {
            let mut client = client.borrow_mut();
            *client = Some(scrobbler); // Store the configured scrobbler
        });
        log::info!("Last.fm client configured with session key (validity checked on first use).");

        // Return Ok because the configuration step itself succeeded
        Ok(())
    }

    /// Check if the Last.fm client is currently authenticated
    pub fn is_authenticated() -> bool {
        let mut result = false;
        LASTFM_CLIENT.with(|client| {
            result = client.borrow().is_some();
        });
        result
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
