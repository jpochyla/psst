extern crate rustfm_scrobble_proxy;
use rustfm_scrobble_proxy::{Scrobble, Scrobbler};
use lazy_static::lazy_static;
use std::sync::{Arc, Mutex};

lazy_static! {
    static ref LASTFM_CLIENT: Mutex<Option<LastFmClient>> = Mutex::new(None);
}

pub struct LastFmClient {
    local_scrobbler: Scrobbler,
}

impl LastFmClient {
    pub fn default() -> Self {
        LastFmClient {
            local_scrobbler: Scrobbler::new("API_KEY", "API_SECRET"),
        }
    }

    pub fn scrobble_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);
        self.local_scrobbler.scrobble(&song).map(|_| ()).map_err(|e| e.to_string())
    }

    pub fn is_authenticated(&self) -> bool {
        self.local_scrobbler.session_key().is_some()
    }

    pub fn nowplaying_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);
        self.local_scrobbler.now_playing(&song).map(|_| ()).map_err(|e| e.to_string())
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
    ) -> Result<(), String> {
        if let (Some(api_key), Some(api_secret), Some(username), Some(password)) =
            (api_key, api_secret, username, password)
        {
            self.local_scrobbler = Scrobbler::new(api_key, api_secret);
            self.local_scrobbler.authenticate_with_password(username, password);

            // Store the authenticated client globally
            let mut client = LASTFM_CLIENT.lock().unwrap();
            *client = Some(LastFmClient {
                local_scrobbler: Scrobbler::new(api_key, api_secret),
            });

            Ok(())
        } else {
            Err("Please fill in all required fields.".to_string())
        }
    }

    pub fn get_global_client() -> Option<Arc<Mutex<LastFmClient>>> {
        let client = LASTFM_CLIENT.lock().unwrap();
        client.as_ref().map(|c| Arc::new(Mutex::new(LastFmClient {
            local_scrobbler: Scrobbler::new("API_KEY", "API_SECRET"),
        })))
    }
}
