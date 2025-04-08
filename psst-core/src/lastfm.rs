extern crate rustfm_scrobble_proxy;
use rustfm_scrobble_proxy::{Scrobble, Scrobbler, ScrobblerError};
use std::cell::RefCell;
use crate::error::Error;


thread_local! {
    static LASTFM_CLIENT: RefCell<Option<Scrobbler>> = RefCell::new(None); //Stores the auth as a thread local variable
}

pub struct LastFmClient;

impl LastFmClient {
    pub fn scrobble_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);

        LASTFM_CLIENT.with(|client| {
            if let Some(client) = &*client.borrow() {
                client.scrobble(&song).map(|_| ()).map_err(|e| e.to_string())
            } else {
                Err("LastFmClient is not initialized.".to_string())
            }
        })
    }

    pub fn nowplaying_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);
        LASTFM_CLIENT.with(|client| {
            if let Some(client) = &*client.borrow() {
                client.now_playing(&song).map(|_| ()).map_err(|e| e.to_string())
            } else {
                Err("LastFmClient is not initialized.".to_string())
            }
        })
    }

    pub fn save_credentials(&self, username: &str, password: &str) -> Result<(), String> {
        log::info!("Saving credentials: username: {} and password: {}", username, password);
        Ok(())
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

            Ok(())
        } else {
            Ok(())
        }
    }
}

//Stinky error implementation
impl From<ScrobblerError> for Error{
    fn from(value: ScrobblerError) -> Self {
        Self::ScrobblerError(Box::new(value))
    }
}