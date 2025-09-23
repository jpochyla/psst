use std::{
    io::Read,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::Deserialize;
use ureq::http::StatusCode;

use crate::{
    error::Error, item_id::FileId, oauth::refresh_access_token, session::SessionService,
    util::default_ureq_agent_builder,
};

pub type CdnHandle = Arc<Cdn>;

pub struct Cdn {
    session: SessionService,
    agent: ureq::Agent,
}

impl Cdn {
    pub fn new(session: SessionService, proxy_url: Option<&str>) -> Result<CdnHandle, Error> {
        let agent = default_ureq_agent_builder(proxy_url).build();
        Ok(Arc::new(Self {
            session,
            agent: agent.into(),
        }))
    }

    pub fn resolve_audio_file_url(&self, id: FileId) -> Result<CdnUrl, Error> {
        let locations_uri = format!(
            "https://api.spotify.com/v1/storage-resolve/files/audio/interactive/{}",
            id.to_base16()
        );
        // OAuth-only: requires a browser OAuth bearer; no Keymaster fallback for CDN.
        let mut access_token = self
            .session
            .oauth_bearer()
            .ok_or_else(|| Error::OAuthError("OAuth access token required".to_string()))?;

        let call = |token: &str| {
            self.agent
                .get(&locations_uri)
                .query("version", "10000000")
                .query("product", "9")
                .query("platform", "39")
                .query("alt", "json")
                .header("Authorization", &format!("Bearer {}", token))
                .call()
        };

        // First attempt; if unauthorized/forbidden, refresh access token and retry once.
        let response = match call(&access_token) {
            Ok(r) => r,
            Err(ureq::Error::StatusCode(code)) if code == 401 || code == 403 => {
                let Some(refresh_token) = self.session.oauth_refresh_token() else {
                    return Err(Error::OAuthError("Missing refresh token".into()));
                };
                let (new_access, new_refresh) = refresh_access_token(&refresh_token)
                    .map_err(|_| Error::OAuthError("Failed to refresh token".into()))?;
                // Update session tokens so future requests use the fresh token
                self.session.set_oauth_bearer(Some(new_access.clone()));
                if let Some(r) = new_refresh {
                    self.session.set_oauth_refresh_token(Some(r));
                }
                access_token = new_access;
                call(&access_token)?
            }
            Err(e) => return Err(Error::AudioFetchingError(Box::new(e))),
        };

        #[derive(Deserialize)]
        struct AudioFileLocations {
            cdnurl: Vec<String>,
        }

        // Deserialize the response and pick a file URL from the returned CDN list.
        let locations: AudioFileLocations = response.into_body().read_json()?;
        let file_uri = match locations.cdnurl.into_iter().next() {
            Some(uri) => uri,
            None => return Err(Error::UnexpectedResponse),
        };

        let uri = CdnUrl::new(file_uri);
        Ok(uri)
    }

    pub fn fetch_file_range(
        &self,
        uri: &str,
        offset: u64,
        length: u64,
    ) -> Result<(u64, impl Read), Error> {
        let req = self
            .agent
            .get(uri)
            .header("Range", &range_header(offset, length));
        match req.call() {
            Ok(response) => {
                let status = response.status();
                if status != StatusCode::PARTIAL_CONTENT {
                    return Err(Error::HttpStatus(status.as_u16()));
                }
                let total_length = parse_total_content_length(&response)?;
                let data_reader = response.into_body().into_reader();
                Ok((total_length, data_reader))
            }
            Err(e) => match e {
                ureq::Error::StatusCode(code) => Err(Error::HttpStatus(code)),
                other => Err(Error::AudioFetchingError(Box::new(other))),
            },
        }
    }
}

#[derive(Clone)]
pub struct CdnUrl {
    pub url: String,
    pub expires: Instant,
}

impl CdnUrl {
    // In case we fail to parse the expiration time from URL, this default is used.
    const DEFAULT_EXPIRATION: Duration = Duration::from_secs(60 * 30);

    // Consider URL expired even before the official expiration time.
    const EXPIRATION_TIME_THRESHOLD: Duration = Duration::from_secs(5);

    fn new(url: String) -> Self {
        let expires_in = parse_expiration(&url).unwrap_or_else(|| {
            log::warn!("failed to parse expiration time from URL {:?}", &url);
            Self::DEFAULT_EXPIRATION
        });
        let expires = Instant::now() + expires_in;
        Self { url, expires }
    }

    pub fn is_expired(&self) -> bool {
        self.expires.saturating_duration_since(Instant::now()) < Self::EXPIRATION_TIME_THRESHOLD
    }
}

impl From<ureq::Error> for Error {
    fn from(err: ureq::Error) -> Self {
        Error::AudioFetchingError(Box::new(err))
    }
}

/// Constructs a Range header value for given offset and length.
fn range_header(offfset: u64, length: u64) -> String {
    let last_byte = offfset + length - 1; // Offset of the last byte of the range is inclusive.
    format!("bytes={offfset}-{last_byte}")
}

/// Parses a total content length from a Content-Range response header.
///
/// For example, returns 146515 for a response with header
/// "Content-Range: bytes 0-1023/146515".
fn parse_total_content_length(
    response: &ureq::http::response::Response<ureq::Body>,
) -> Result<u64, Error> {
    let header = match response.headers().get("Content-Range") {
        Some(h) => h,
        None => return Err(Error::UnexpectedResponse),
    };
    let s = match header.to_str() {
        Ok(s) => s,
        Err(_) => return Err(Error::UnexpectedResponse),
    };
    let total_str = match s.split('/').next_back() {
        Some(x) => x,
        None => return Err(Error::UnexpectedResponse),
    };
    let total = match total_str.parse::<u64>() {
        Ok(n) => n,
        Err(_) => return Err(Error::UnexpectedResponse),
    };
    Ok(total)
}

/// Parses an expiration of an audio file URL.
fn parse_expiration(url: &str) -> Option<Duration> {
    let token_exp = url.split("__token__=exp=").nth(1);
    let expires_millis = if let Some(token_exp) = token_exp {
        // Parse from the expiration token param
        token_exp.split('~').next()?
    } else if let Some(verify_exp) = url.split("verify=").nth(1) {
        // Parse from verify parameter (new spotifycdn.com format)
        verify_exp.split('-').next()?
    } else {
        // Parse from the first param
        let first_param = url.split('?').nth(1)?;
        first_param.split('_').next()?
    };
    let expires_millis = expires_millis.parse().ok()?;
    let expires = Duration::from_millis(expires_millis);
    Some(expires)
}
