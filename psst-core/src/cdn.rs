use std::{
    io::Read,
    sync::Arc,
    time::{Duration, Instant},
};

use librespot_protocol::storage_resolve::StorageResolveResponse;
use parking_lot::Mutex;
use protobuf::Message;

use crate::{
    connection::Transport,
    error::Error,
    item_id::FileId,
    session::{
        client_token::{ClientTokenProvider, ClientTokenProviderHandle},
        login5::Login5,
        SessionService,
    },
    util::default_ureq_agent_builder,
};

pub type CdnHandle = Arc<Cdn>;

pub struct Cdn {
    session: SessionService,
    agent: ureq::Agent,
    login5: Login5,
    client_token_provider: ClientTokenProviderHandle,
    spclient_base: Mutex<Option<String>>,
    proxy_url: Option<String>,
}

impl Cdn {
    pub fn new(session: SessionService, proxy_url: Option<&str>) -> Result<CdnHandle, Error> {
        let agent = default_ureq_agent_builder(proxy_url).build();
        // Share a single ClientTokenProvider between Login5 and Cdn to avoid
        // redundant round-trips to the client token API.
        let client_token_provider = ClientTokenProvider::new_shared(proxy_url);
        Ok(Arc::new(Self {
            session,
            agent: agent.into(),
            login5: Login5::new(Some(Arc::clone(&client_token_provider)), proxy_url),
            client_token_provider,
            spclient_base: Mutex::new(None),
            proxy_url: proxy_url.map(String::from),
        }))
    }

    /// Resolve and cache the spclient base URL (e.g. "https://gew1-spclient.spotify.com:443").
    fn get_spclient_base(&self) -> Result<String, Error> {
        let mut cached = self.spclient_base.lock();
        if let Some(ref url) = *cached {
            return Ok(url.clone());
        }
        let hosts = Transport::resolve_spclient(self.proxy_url.as_deref())?;
        let host = hosts.first().ok_or(Error::UnexpectedResponse)?;
        let base = format!("https://{host}");
        log::info!("using spclient base URL: {base}");
        *cached = Some(base.clone());
        Ok(base)
    }

    pub fn resolve_audio_file_url(&self, id: FileId) -> Result<CdnUrl, Error> {
        let spclient_base = self.get_spclient_base()?;
        // The spclient endpoint returns protobuf natively and does not require
        // the query parameters that the old api.spotify.com/v1/storage-resolve
        // JSON endpoint needed (?alt=json&version=10000000&product=9&platform=39).
        // This matches librespot's implementation.
        let locations_uri = format!(
            "{spclient_base}/storage-resolve/files/audio/interactive/{}",
            id.to_base16()
        );
        let access_token = self.login5.get_access_token(&self.session)?;
        let client_token = self.client_token_provider.get()?;
        let mut response = self
            .agent
            .get(&locations_uri)
            .header(
                "Authorization",
                &format!("Bearer {}", access_token.access_token),
            )
            .header("client-token", &client_token)
            .call()?;

        // Parse the protobuf StorageResolveResponse.
        let bytes = response.body_mut().read_to_vec()?;
        let msg = StorageResolveResponse::parse_from_bytes(&bytes)
            .map_err(|e| Error::AudioFetchingError(Box::new(e)))?;

        // Pick a file URL from the returned CDN list.
        let file_uri = msg
            .cdnurl
            .into_iter()
            .next()
            .ok_or(Error::UnexpectedResponse)?;

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
            .header("Range", &range_header(offset, length))
            .call()?;
        let total_length = parse_total_content_length(&response);
        let data_reader = response.into_body().into_reader();
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
    format!("bytes={offfset}-{last_byte}")
}

/// Parses a total content length from a Content-Range response header.
///
/// For example, returns 146515 for a response with header
/// "Content-Range: bytes 0-1023/146515".
fn parse_total_content_length(response: &ureq::http::response::Response<ureq::Body>) -> u64 {
    response
        .headers()
        .get("Content-Range")
        .expect("Content-Range header not found")
        .to_str()
        .expect("Failed to parse Content-Range Header")
        .split('/')
        .next_back()
        .expect("Failed to parse Content-Range Header")
        .parse()
        .expect("Failed to parse Content-Range Header")
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
