use std::{
    io::Read,
    sync::Arc,
    time::{Duration, Instant},
};

use serde::Deserialize;

use crate::{
    error::Error,
    item_id::FileId,
    session::{access_token::TokenProvider, SessionService},
    util::default_ureq_agent_builder,
};

pub type CdnHandle = Arc<Cdn>;

pub struct Cdn {
    session: SessionService,
    agent: ureq::Agent,
    token_provider: TokenProvider,
}

impl Cdn {
    pub fn new(session: SessionService, proxy_url: Option<&str>) -> Result<CdnHandle, Error> {
        let agent = default_ureq_agent_builder(proxy_url)?.build();
        Ok(Arc::new(Self {
            session,
            agent,
            token_provider: TokenProvider::new(),
        }))
    }

    pub fn resolve_audio_file_url(&self, id: FileId) -> Result<CdnUrl, Error> {
        let locations_uri = format!(
            "https://api.spotify.com/v1/storage-resolve/files/audio/interactive/{}",
            id.to_base16()
        );
        let access_token = self.token_provider.get(&self.session)?;
        let response = self
            .agent
            .get(&locations_uri)
            .query("version", "10000000")
            .query("product", "9")
            .query("platform", "39")
            .query("alt", "json")
            .set("Authorization", &format!("Bearer {}", access_token.token))
            .call()?;

        #[derive(Deserialize)]
        struct AudioFileLocations {
            cdnurl: Vec<String>,
        }

        // Deserialize the response and pick a file URL from the returned CDN list.
        let locations: AudioFileLocations = response.into_json()?;
        let file_uri = locations
            .cdnurl
            .into_iter()
            // TODO:
            //  Now, we always pick the first URL in the list, figure out a better strategy.
            //  Choosing by random seems wrong.
            .next()
            // TODO: Avoid panicking here.
            .expect("No file URI found");

        let uri = CdnUrl::new(file_uri);
        Ok(uri)
    }

    pub fn fetch_file_range(
        &self,
        uri: &str,
        offset: u64,
        length: u64,
    ) -> Result<(u64, impl Read), Error> {
        let response = self
            .agent
            .get(uri)
            .set("Range", &range_header(offset, length))
            .call()?;
        let total_length = parse_total_content_length(&response);
        let data_reader = response.into_reader();
        Ok((total_length, data_reader))
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
    format!("bytes={}-{}", offfset, last_byte)
}

/// Parses a total content length from a Content-Range response header.
///
/// For example, returns 146515 for a response with header
/// "Content-Range: bytes 0-1023/146515".
fn parse_total_content_length(response: &ureq::Response) -> u64 {
    response
        .header("Content-Range")
        .expect("Content-Range header not found")
        .split('/')
        .last()
        .expect("Failed to parse Content-Range Header")
        .parse()
        .expect("Failed to parse Content-Range Header")
}

/// Parses an expiration of an audio file URL.
/// Expiration is stored either as:
///
///  1. `exp` field after `__token__`:
///     .../...a35817ca410?__token__=exp=1629466995~hmac=df348...
///                                      ^========^
///  2. or at the beginning of the first query parameter:
///     .../59db919e18d6336461a0c71da051842ceef1b5af?1602319025_wu-SPeHxn...
///                                                  ^========^
fn parse_expiration(url: &str) -> Option<Duration> {
    let token_exp = url.split("__token__=exp=").nth(1);
    let expires_millis = if let Some(token_exp) = token_exp {
        // Parse from the expiration token param.
        token_exp.split('~').next()?
    } else {
        // Parse from the first param.
        let first_param = url.split('?').nth(1)?;
        first_param.split('_').next()?
    };
    let expires_millis = expires_millis.parse().ok()?;
    let expires = Duration::from_millis(expires_millis);
    Some(expires)
}
