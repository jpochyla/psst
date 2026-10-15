use crate::error::Error;
use crate::oauth::listen_for_callback_parameter;
use std::{
    collections::BTreeMap,
    net::SocketAddr,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use url::Url;

pub struct LastFmClient;

// Replaces the abandoned rustfm-scrobble HTTP/TLS stack with the same maintained
// HTTPS client used by Spotify. Last.fm requires MD5 for API request signatures.
pub struct Scrobbler {
    api_key: String,
    api_secret: String,
    session_key: String,
}

fn signature(params: &BTreeMap<String, String>, secret: &str) -> String {
    let mut data = String::new();
    for (key, value) in params {
        if !matches!(key.as_str(), "format" | "callback" | "api_sig") {
            data.push_str(key);
            data.push_str(value);
        }
    }
    data.push_str(secret);
    format!("{:x}", md5::compute(data.as_bytes()))
}

fn request(
    api_key: &str,
    secret: &str,
    method: &str,
    mut params: BTreeMap<String, String>,
) -> Result<serde_json::Value, Error> {
    params.insert("api_key".into(), api_key.into());
    params.insert("method".into(), method.into());
    params.insert("api_sig".into(), signature(&params, secret));
    params.insert("format".into(), "json".into());
    let form: Vec<_> = params
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(10)))
        .max_redirects(0)
        .build()
        .into();
    let mut response = agent
        .post("https://ws.audioscrobbler.com/2.0/")
        .send_form(form)
        .map_err(|_| Error::ConfigError("Last.fm request failed".into()))?;
    let payload: serde_json::Value = response.body_mut().read_json()?;
    if let Some(code) = payload["error"].as_u64() {
        return Err(Error::ConfigError(format!("Last.fm returned error {code}")));
    }
    Ok(payload)
}

impl Scrobbler {
    fn report(
        &self,
        method: &str,
        artist: &str,
        title: &str,
        album: Option<&str>,
        scrobble: bool,
    ) -> Result<(), Error> {
        let mut params = BTreeMap::from([
            ("artist".into(), artist.into()),
            ("track".into(), title.into()),
            ("sk".into(), self.session_key.clone()),
        ]);
        if let Some(album) = album {
            params.insert("album".into(), album.into());
        }
        if scrobble {
            params.insert(
                "timestamp".into(),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_string(),
            );
        }
        request(&self.api_key, &self.api_secret, method, params).map(|_| ())
    }
}

impl LastFmClient {
    /// Report a track as "now playing" to Last.fm using an existing Scrobbler instance.
    pub fn now_playing_song(
        scrobbler: &Scrobbler, // Requires an authenticated Scrobbler
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        scrobbler.report("track.updateNowPlaying", artist, title, album, false)
    }

    /// Scrobble a finished track to Last.fm using an existing Scrobbler instance.
    pub fn scrobble_song(
        scrobbler: &Scrobbler, // Requires an authenticated Scrobbler
        artist: &str,
        title: &str,
        album: Option<&str>,
    ) -> Result<(), Error> {
        scrobbler.report("track.scrobble", artist, title, album, true)
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

        let scrobbler = Scrobbler {
            api_key: api_key.into(),
            api_secret: api_secret.into(),
            session_key: session_key.into(),
        };
        log::info!("scrobbler instance created with session key (validity checked on first use).");
        Ok(scrobbler)
    }
}

/// Generate a Last.fm authentication URL
pub fn generate_lastfm_auth_url(
    api_key: &str,
    callback_url: &str,
) -> Result<String, url::ParseError> {
    let base = "https://www.last.fm/api/auth/";
    let url = Url::parse_with_params(base, &[("api_key", api_key), ("cb", callback_url)])?;
    Ok(url.to_string())
}

/// Exchange a token for a Last.fm session key
pub fn exchange_token_for_session(
    api_key: &str,
    api_secret: &str,
    token: &str,
) -> Result<String, Error> {
    let response = request(
        api_key,
        api_secret,
        "auth.getSession",
        BTreeMap::from([("token".into(), token.into())]),
    )?;
    response["session"]["key"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(String::from)
        .ok_or_else(|| Error::ConfigError("Last.fm did not return a session key".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn signature_uses_sorted_parameters_and_excludes_format_and_callback() {
        let mut params = BTreeMap::from([
            ("token".into(), "yyyyyy".into()),
            ("method".into(), "auth.getSession".into()),
            ("api_key".into(), "xxxxxxxxxx".into()),
        ]);
        let expected = format!(
            "{:x}",
            md5::compute(b"api_keyxxxxxxxxxxmethodauth.getSessiontokenyyyyyyilovecher")
        );
        assert_eq!(signature(&params, "ilovecher"), expected);
        params.insert("format".into(), "json".into());
        params.insert("callback".into(), "untrusted".into());
        assert_eq!(signature(&params, "ilovecher"), expected);
    }
}

/// Listen for a Last.fm token from the callback
pub fn get_lastfm_token_listener(
    socket_address: SocketAddr,
    timeout: Duration,
) -> Result<String, Error> {
    // Use the shared listener function, specifying "token" as the parameter
    listen_for_callback_parameter(socket_address, timeout, "token")
}
