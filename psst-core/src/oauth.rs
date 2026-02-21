use crate::error::Error;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    path::PathBuf,
    sync::mpsc,
    time::Duration,
};
use url::Url;

use crate::session::access_token::{PSST_CLIENT_ID, WEBAPI_SCOPES};

// ── Callback listener (shared by all OAuth flows) ──────────────────────────

pub fn listen_for_callback_parameter(
    socket_address: SocketAddr,
    timeout: Duration,
    parameter_name: &'static str,
) -> Result<String, Error> {
    log::info!("starting callback listener for '{parameter_name}' on {socket_address:?}",);

    // Create a simpler, linear flow
    // 1. Bind the listener
    let listener = match TcpListener::bind(socket_address) {
        Ok(l) => {
            log::info!("listener bound successfully");
            l
        }
        Err(e) => {
            log::error!("Failed to bind listener: {e}");
            return Err(Error::IoError(e));
        }
    };

    // 2. Set up the channel for communication
    let (tx, rx) = mpsc::channel::<Result<String, Error>>();

    // 3. Spawn the thread
    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            handle_callback_connection(&mut stream, &tx, parameter_name);
        } else {
            log::error!("Failed to accept connection on callback listener");
            let _ = tx.send(Err(Error::IoError(std::io::Error::other(
                "Failed to accept connection",
            ))));
        }
    });

    // 4. Wait for the result with timeout
    let result = match rx.recv_timeout(timeout) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Timed out or channel error: {e}");
            return Err(Error::from(e));
        }
    };

    // 5. Wait for thread completion
    if handle.join().is_err() {
        log::warn!("thread join failed, but continuing with result");
    }

    // 6. Return the result
    result
}

/// Handles an incoming TCP connection for a generic OAuth callback.
fn handle_callback_connection(
    stream: &mut TcpStream,
    tx: &mpsc::Sender<Result<String, Error>>,
    parameter_name: &'static str,
) {
    let mut reader = BufReader::new(&mut *stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_ok() {
        match extract_parameter_from_request(&request_line, parameter_name) {
            Some(value) => {
                log::info!("received callback parameter '{parameter_name}'.");
                send_success_response(stream);
                let _ = tx.send(Ok(value));
            }
            None => {
                let err_msg = format!(
                    "Failed to extract parameter '{parameter_name}' from request: {request_line}",
                );
                log::error!("{err_msg}");
                let _ = tx.send(Err(Error::OAuthError(err_msg)));
            }
        }
    } else {
        log::error!("Failed to read request line from callback.");
        let _ = tx.send(Err(Error::IoError(std::io::Error::other(
            "Failed to read request line",
        ))));
    }
}

/// Extracts a specified query parameter from an HTTP request line.
fn extract_parameter_from_request(request_line: &str, parameter_name: &str) -> Option<String> {
    request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| Url::parse(&format!("http://localhost{path}")).ok())
        .and_then(|url| {
            url.query_pairs()
                .find(|(key, _)| key == parameter_name)
                .map(|(_, value)| value.into_owned())
        })
}

pub fn get_authcode_listener(
    socket_address: SocketAddr,
    timeout: Duration,
) -> Result<AuthorizationCode, Error> {
    listen_for_callback_parameter(socket_address, timeout, "code").map(AuthorizationCode::new)
}

pub fn send_success_response(stream: &mut TcpStream) {
    let response = "HTTP/1.1 200 OK\r\n\r\n\
        <html>\
        <head>\
            <style>\
                body {\
                    background-color: #121212;\
                    color: #ffffff;\
                    font-family: sans-serif;\
                    display: flex;\
                    justify-content: center;\
                    align-items: center;\
                    height: 100vh;\
                    margin: 0;\
                }\
                a {\
                    color: #aaaaaa;\
                    text-decoration: underline;\
                    cursor: pointer;\
                }\
            </style>\
        </head>\
        <body>\
            <div>Successfully authenticated! You can close this window now.</div>\
        </body>\
        </html>";
    let _ = stream.write_all(response.as_bytes());
}

// ── Session OAuth (official Spotify Client ID, for Shannon session) ────────

fn create_spotify_oauth_client(redirect_port: u16) -> BasicClient {
    let redirect_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let redirect_uri = format!("http://{redirect_address}/login");

    BasicClient::new(
        ClientId::new(crate::session::access_token::CLIENT_ID.to_string()),
        None,
        AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri).expect("Invalid redirect URL"))
}

pub fn generate_auth_url(redirect_port: u16) -> (String, PkceCodeVerifier) {
    let client = create_spotify_oauth_client(redirect_port);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(get_scopes())
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), pkce_verifier)
}

pub fn exchange_code_for_token(
    redirect_port: u16,
    code: AuthorizationCode,
    pkce_verifier: PkceCodeVerifier,
) -> String {
    let client = create_spotify_oauth_client(redirect_port);

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)
        .expect("Failed to exchange code for token");

    token_response.access_token().secret().to_string()
}

fn get_scopes() -> Vec<Scope> {
    crate::session::access_token::ACCESS_SCOPES
        .split(',')
        .map(|s| Scope::new(s.trim().to_string()))
        .collect()
}

// ── Web API OAuth (Psst's own Client ID) ───────────────────────────────────

/// Token for Web API calls, serializable to/from disk.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WebApiToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix timestamp (seconds) when the token expires.
    pub expires_at: u64,
}

impl WebApiToken {
    pub fn is_expired(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Consider expired 60 seconds early to avoid edge cases
        now + 60 >= self.expires_at
    }
}

fn webapi_redirect_uri(redirect_port: u16) -> String {
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    format!("http://{addr}/login")
}

fn create_webapi_oauth_client(redirect_port: u16) -> BasicClient {
    BasicClient::new(
        ClientId::new(PSST_CLIENT_ID.to_string()),
        None,
        AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap()),
    )
    .set_redirect_uri(
        RedirectUrl::new(webapi_redirect_uri(redirect_port)).expect("Invalid redirect URL"),
    )
}

fn get_webapi_scopes() -> Vec<Scope> {
    WEBAPI_SCOPES
        .iter()
        .map(|s| Scope::new(s.to_string()))
        .collect()
}

/// Generate the authorization URL for the Web API OAuth flow.
/// Returns `(auth_url, pkce_verifier)`.
pub fn generate_webapi_auth_url(redirect_port: u16) -> (String, PkceCodeVerifier) {
    let client = create_webapi_oauth_client(redirect_port);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(get_webapi_scopes())
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), pkce_verifier)
}

/// Exchange an authorization code for a full WebApiToken (including refresh token).
pub fn exchange_webapi_code_for_token(
    redirect_port: u16,
    code: AuthorizationCode,
    pkce_verifier: PkceCodeVerifier,
) -> Result<WebApiToken, Error> {
    let client = create_webapi_oauth_client(redirect_port);

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)
        .map_err(|e| Error::OAuthError(format!("Failed to exchange Web API code: {e}")))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let expires_in = token_response
        .expires_in()
        .unwrap_or(Duration::from_secs(3600))
        .as_secs();

    Ok(WebApiToken {
        access_token: token_response.access_token().secret().to_string(),
        refresh_token: token_response
            .refresh_token()
            .map(|t| t.secret().to_string()),
        expires_at: now + expires_in,
    })
}

/// Refresh a Web API token using a refresh token (no browser interaction needed).
pub fn refresh_webapi_token(refresh_token_str: &str) -> Result<WebApiToken, Error> {
    // For refresh, the redirect_port doesn't matter (no redirect happens),
    // but the client needs to be configured with the same redirect URI.
    let client = create_webapi_oauth_client(8888);

    let refresh_token = RefreshToken::new(refresh_token_str.to_string());

    let token_response = client
        .exchange_refresh_token(&refresh_token)
        .request(http_client)
        .map_err(|e| Error::OAuthError(format!("Failed to refresh Web API token: {e}")))?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let expires_in = token_response
        .expires_in()
        .unwrap_or(Duration::from_secs(3600))
        .as_secs();

    // Spotify may or may not return a new refresh token on refresh.
    // If it doesn't, we keep the old one.
    let new_refresh_token = token_response
        .refresh_token()
        .map(|t| t.secret().to_string())
        .unwrap_or_else(|| refresh_token_str.to_string());

    Ok(WebApiToken {
        access_token: token_response.access_token().secret().to_string(),
        refresh_token: Some(new_refresh_token),
        expires_at: now + expires_in,
    })
}

// ── Token persistence ──────────────────────────────────────────────────────

const WEBAPI_TOKEN_FILENAME: &str = "webapi_token.json";

fn webapi_token_path() -> Option<PathBuf> {
    platform_dirs::AppDirs::new(Some("Psst"), false)
        .map(|dirs| dirs.config_dir.join(WEBAPI_TOKEN_FILENAME))
}

/// Save a Web API token to disk.
pub fn save_webapi_token(token: &WebApiToken) -> Result<(), Error> {
    let path = webapi_token_path()
        .ok_or_else(|| Error::OAuthError("Cannot determine config directory".to_string()))?;

    // Ensure the directory exists
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(Error::IoError)?;
    }

    let json = serde_json::to_string_pretty(token)
        .map_err(|e| Error::OAuthError(format!("Failed to serialize token: {e}")))?;

    #[cfg(target_family = "unix")]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&path)
            .map_err(Error::IoError)?;
        use std::io::Write as _;
        let mut writer = std::io::BufWriter::new(file);
        writer.write_all(json.as_bytes()).map_err(Error::IoError)?;
    }

    #[cfg(not(target_family = "unix"))]
    {
        fs::write(&path, json).map_err(Error::IoError)?;
    }

    log::info!("saved Web API token to {:?}", path);
    Ok(())
}

/// Load a Web API token from disk.
pub fn load_webapi_token() -> Option<WebApiToken> {
    let path = webapi_token_path()?;
    let json = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&json).ok()
}

/// Delete the cached Web API token from disk.
pub fn delete_webapi_token() {
    if let Some(path) = webapi_token_path() {
        let _ = fs::remove_file(path);
    }
}

/// Try to get a valid Web API access token from disk cache, refreshing if needed.
///
/// Returns `Ok(token)` if successful.
/// Returns `Err(...)` if no cached token exists or refresh failed — caller should
/// trigger a full browser-based OAuth flow.
pub fn get_or_refresh_webapi_token() -> Result<WebApiToken, Error> {
    if let Some(token) = load_webapi_token() {
        if !token.is_expired() {
            return Ok(token);
        }

        // Token is expired, try to refresh
        if let Some(ref refresh_token) = token.refresh_token {
            log::info!("Web API token expired, attempting refresh...");
            match refresh_webapi_token(refresh_token) {
                Ok(new_token) => {
                    if let Err(e) = save_webapi_token(&new_token) {
                        log::warn!("Failed to save refreshed Web API token: {e}");
                    }
                    return Ok(new_token);
                }
                Err(e) => {
                    log::error!("Failed to refresh Web API token: {e}");
                    // Fall through to error below
                }
            }
        }
    }

    Err(Error::OAuthError(
        "No valid Web API token available. Browser-based authentication required.".to_string(),
    ))
}

/// Perform the full Web API OAuth flow: open browser, wait for callback, exchange code.
/// This is a blocking call that waits for the user to complete the browser flow.
///
/// Returns the new `WebApiToken` and saves it to disk.
pub fn perform_webapi_oauth_flow(redirect_port: u16) -> Result<WebApiToken, Error> {
    let (auth_url, pkce_verifier) = generate_webapi_auth_url(redirect_port);

    // Open the browser
    if open::that(&auth_url).is_err() {
        return Err(Error::OAuthError(
            "Failed to open browser for Web API authentication".to_string(),
        ));
    }

    // Listen for the callback
    let socket_addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let code = get_authcode_listener(socket_addr, Duration::from_secs(300))?;

    // Exchange code for token
    let token = exchange_webapi_code_for_token(redirect_port, code, pkce_verifier)?;

    // Save to disk
    save_webapi_token(&token)?;

    log::info!("Web API OAuth flow completed successfully");
    Ok(token)
}
