extern crate rustfm_scrobble_proxy;
use crate::error::Error;
use crate::oauth::listen_for_callback_parameter;
use rustfm_scrobble_proxy::{responses::SessionResponse, Scrobble, Scrobbler, ScrobblerError};
use std::cell::RefCell;
use std::{net::SocketAddr, time::Duration};
use url::Url;

thread_local! {
    static LASTFM_CLIENT: RefCell<Option<Scrobbler>> = const { RefCell::new(None)}; //Stores the auth as a thread local variable
}

pub struct LastFmClient;

impl LastFmClient {
    pub fn scrobble_song(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        let song = Scrobble::new(artist, title, album);
        LASTFM_CLIENT.with(|client| {
            if let Some(client) = &*client.borrow() {
                client.scrobble(&song).map(|_| ())?
            } else {
                log::warn!("LastFmClient is not initialized.");
            }
            Ok(())
        })
    }

    pub fn now_playing_song(
        &self,
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        let song = Scrobble::new(artist, title, album);
        LASTFM_CLIENT.with(|client| {
            if let Some(client) = &*client.borrow() {
                client.now_playing(&song).map(|_| ())?
            } else {
                log::warn!("LastFmClient is not initialized.");
            }
            Ok(())
        })
    }

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
}

impl From<ScrobblerError> for Error {
    fn from(value: ScrobblerError) -> Self {
        Self::ScrobblerError(Box::new(value))
    }
}

pub fn generate_lastfm_auth_url(
    api_key: &str,
    callback_url: &str,
) -> Result<String, url::ParseError> {
    let base = "http://www.last.fm/api/auth/";
    let url = Url::parse_with_params(base, &[("api_key", api_key), ("cb", callback_url)])?;
    Ok(url.to_string())
}

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

pub fn get_lastfm_token_listener(
    socket_address: SocketAddr,
    timeout: Duration,
) -> Result<String, Error> {
    // Use the shared listener function, specifying "token" as the parameter
    listen_for_callback_parameter(socket_address, timeout, "token")
}
