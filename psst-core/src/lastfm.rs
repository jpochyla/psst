extern crate rustfm_scrobble_proxy;
use once_cell::sync::OnceCell;
use rustfm_scrobble_proxy::{Scrobble, Scrobbler};

thread_local! {
    static LASTFM_CLIENT: OnceCell<Scrobbler> = OnceCell::new();
}

pub struct LastFmClient;

impl LastFmClient {
    pub fn scrobble_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);

        LASTFM_CLIENT.with(|client| {
            if let Some(client) = client.get() {
                client.scrobble(&song).map(|_| ()).map_err(|e| e.to_string())
            } else {
                Err("LastFmClient is not initialized.".to_string())
            }
        })
    }

    pub fn nowplaying_song(&self, artist: &str, title: &str, album: Option<&str>) -> Result<(), String> {
        let song = Scrobble::new(artist, title, album);
        LASTFM_CLIENT.with(|client| {
            if let Some(client) = client.get() {
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
    ) -> Result<(), String> {
        if let (Some(api_key), Some(api_secret), Some(username), Some(password)) =
            (api_key, api_secret, username, password)
        {
            let mut scrobbler = Scrobbler::new(api_key, api_secret);
            scrobbler.authenticate_with_password(username, password);

            // Store the authenticated client globally
            LASTFM_CLIENT.with(|client| {
                client.set(scrobbler).map_err(|_| "Failed to set LastFmClient".to_string())
            })?;
            log::info!("Authenticated with Last.fm successfully.");

            Ok(())
        } else {
            Err("Please fill in all required fields.".to_string())
        }
    }
}
