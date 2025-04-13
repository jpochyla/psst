extern crate rustfm_scrobble_proxy;
use crate::error::Error;
use rustfm_scrobble_proxy::{Scrobble, Scrobbler, ScrobblerError};
use std::cell::RefCell;

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
        username: Option<&str>,
        password: Option<&str>,
    ) -> Result<(), Error> {
        if let (Some(api_key), Some(api_secret), Some(username), Some(password)) =
            (api_key, api_secret, username, password)
        {
            let mut scrobbler = Scrobbler::new(api_key, api_secret);
            scrobbler.authenticate_with_password(username, password)?;

            // Store the authenticated client globally
            LASTFM_CLIENT.with(|client| {
                let mut client = client.borrow_mut();
                *client = Some(scrobbler);
            });
            log::info!("Authenticated with Last.fm successfully.");
        } else {
            log::warn!("Missing authentication parameters.");
        }
        Ok(())
    }
}

impl From<ScrobblerError> for Error {
    fn from(value: ScrobblerError) -> Self {
        Self::ScrobblerError(Box::new(value))
    }
}
