use crate::error::Error;
use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    sync::mpsc,
    time::Duration,
};
use url::Url;

use crate::session::access_token::WEBAPI_SCOPES;

pub fn listen_for_callback_parameter(
    socket_address: SocketAddr,
    timeout: Duration,
    parameter_name: &'static str,
) -> Result<String, Error> {
    log::info!(
        "starting callback listener for '{parameter_name}' on {socket_address:?}",
    );

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

    // 3. Spawn the thread. Loop so background requests (e.g. favicon.ico)
    //    don't consume the single accept and break the OAuth flow.
    let handle = std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    if handle_callback_connection(&mut stream, &tx, parameter_name) {
                        break;
                    }
                }
                Err(e) => {
                    log::error!("Failed to accept callback connection: {e}");
                    let _ = tx.send(Err(Error::IoError(e)));
                    break;
                }
            }
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

/// Handle one incoming TCP connection. Returns `true` if the OAuth
/// parameter was extracted (caller should stop listening), or `false` to
/// keep listening for the next request (e.g. favicon, malformed request).
fn handle_callback_connection(
    stream: &mut TcpStream,
    tx: &mpsc::Sender<Result<String, Error>>,
    parameter_name: &'static str,
) -> bool {
    let mut reader = BufReader::new(&mut *stream);
    let mut request_line = String::new();
    const MAX_REQUEST_LINE_LEN: usize = 8192; // 8 KiB limit

    // Read with a size limit to prevent memory exhaustion attacks
    let mut bytes_read = 0;
    let mut buf = [0u8; 1];
    loop {
        match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(1) => {
                bytes_read += 1;
                if bytes_read > MAX_REQUEST_LINE_LEN {
                    log::error!("Request line exceeds {MAX_REQUEST_LINE_LEN} bytes limit");
                    return false;
                }
                request_line.push(buf[0] as char);
                if buf[0] == b'\n' {
                    break;
                }
            }
            _ => return false,
        }
    }

    if request_line.contains("favicon.ico") {
        send_not_found_response(stream);
        return false;
    }

    match extract_parameter_from_request(&request_line, parameter_name) {
        Some(value) => {
            log::info!("received callback parameter '{parameter_name}'.");
            send_success_response(stream);
            let _ = tx.send(Ok(value));
            true
        }
        None => {
            log::warn!("ignoring request without '{parameter_name}': {request_line}");
            send_not_found_response(stream);
            false
        }
    }
}

fn send_not_found_response(stream: &mut TcpStream) {
    let _ = stream.write_all(b"HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n");
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

fn create_oauth_client(client_id: &str, redirect_port: u16) -> BasicClient {
    let redirect_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let redirect_uri = format!("http://{redirect_address}/login");

    BasicClient::new(
        ClientId::new(client_id.to_string()),
        None,
        AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap(),
        Some(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_uri).expect("Invalid redirect URL"))
}

pub fn generate_session_auth_url(redirect_port: u16) -> (String, PkceCodeVerifier) {
    let client_id = crate::session::access_token::CLIENT_ID;
    let scopes = get_scopes();
    generate_auth_url(client_id, redirect_port, &scopes)
}

pub fn exchange_session_code_for_token(
    redirect_port: u16,
    code: AuthorizationCode,
    pkce_verifier: PkceCodeVerifier,
) -> String {
    let client_id = crate::session::access_token::CLIENT_ID;
    let client = create_oauth_client(client_id, redirect_port);

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

fn get_webapi_scopes() -> Vec<Scope> {
    WEBAPI_SCOPES
        .iter()
        .map(|s| Scope::new(s.to_string()))
        .collect()
}

/// Generate the authorization URL for any OAuth flow.
/// Returns `(auth_url, pkce_verifier)`.
pub fn generate_auth_url(
    client_id: &str,
    redirect_port: u16,
    scopes: &[Scope],
) -> (String, PkceCodeVerifier) {
    let client = create_oauth_client(client_id, redirect_port);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes.iter().cloned())
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), pkce_verifier)
}

/// Generate the authorization URL specifically for the Web API OAuth flow.
/// Returns `(auth_url, pkce_verifier)`.
pub fn generate_webapi_auth_url(client_id: &str, redirect_port: u16) -> (String, PkceCodeVerifier) {
    let scopes = get_webapi_scopes();
    generate_auth_url(client_id, redirect_port, &scopes)
}

/// Exchange an authorization code for a full WebApiToken (including refresh token).
pub fn exchange_webapi_code_for_token(
    client_id: &str,
    redirect_port: u16,
    code: AuthorizationCode,
    pkce_verifier: PkceCodeVerifier,
) -> Result<WebApiToken, Error> {
    let client = create_oauth_client(client_id, redirect_port);

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
pub fn refresh_webapi_token(
    client_id: &str,
    refresh_token_str: &str,
) -> Result<WebApiToken, Error> {
    // For refresh, the redirect_port doesn't matter (no redirect happens),
    // but the client needs to be configured with the same redirect URI.
    let client = create_oauth_client(client_id, 8888);

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
