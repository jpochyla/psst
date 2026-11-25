use crate::error::Error;
use oauth2::{
    basic::{BasicClient, BasicErrorResponseType, BasicTokenResponse},
    reqwest::http_client,
    AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge, PkceCodeVerifier,
    RedirectUrl, RefreshToken, RequestTokenError, Scope, TokenResponse, TokenUrl,
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

use crate::system_info::CLIENT_ID;

// Scopes for the session login through Spotify's own client ID.
const SESSION_SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "playlist-read-private",
    "playlist-read-collaborative",
    "playlist-modify-public",
    "playlist-modify-private",
    "user-follow-modify",
    "user-follow-read",
    "user-library-read",
    "user-library-modify",
    "user-top-read",
    "user-read-recently-played",
];

// Scopes for Web API calls through the user's own client ID.
const WEBAPI_SCOPES: &[&str] = &[
    "streaming",
    "user-read-email",
    "user-read-private",
    "user-library-read",
    "user-library-modify",
    "user-read-playback-state",
    "user-modify-playback-state",
    "playlist-read-private",
    "playlist-read-collaborative",
    "playlist-modify-public",
    "playlist-modify-private",
    "user-follow-read",
    "user-follow-modify",
    "user-top-read",
    "user-read-currently-playing",
    "user-read-recently-played",
];

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

    if reader.read_line(&mut request_line).is_err() {
        return false;
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
    generate_auth_url(CLIENT_ID, redirect_port, SESSION_SCOPES)
}

pub fn exchange_session_code_for_token(
    redirect_port: u16,
    code: AuthorizationCode,
    pkce_verifier: PkceCodeVerifier,
) -> String {
    let client = create_oauth_client(CLIENT_ID, redirect_port);

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request(http_client)
        .expect("Failed to exchange code for token");

    token_response.access_token().secret().to_string()
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
        // Consider expired 60 seconds early to avoid edge cases
        unix_now() + 60 >= self.expires_at
    }

    fn from_response(response: &BasicTokenResponse, previous_refresh_token: Option<&str>) -> Self {
        let expires_in = response
            .expires_in()
            .unwrap_or(Duration::from_secs(3600))
            .as_secs();
        // Spotify mostly omits (or blanks) the refresh token on refresh, in
        // which case the previous one stays valid.
        let refresh_token = response
            .refresh_token()
            .map(|t| t.secret().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| previous_refresh_token.map(String::from));
        Self {
            access_token: response.access_token().secret().to_string(),
            refresh_token,
            expires_at: unix_now() + expires_in,
        }
    }
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Generate the authorization URL for any OAuth flow.
/// Returns `(auth_url, pkce_verifier)`.
fn generate_auth_url(
    client_id: &str,
    redirect_port: u16,
    scopes: &[&str],
) -> (String, PkceCodeVerifier) {
    let client = create_oauth_client(client_id, redirect_port);
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, _) = client
        .authorize_url(CsrfToken::new_random)
        .add_scopes(scopes.iter().map(|s| Scope::new(s.to_string())))
        .set_pkce_challenge(pkce_challenge)
        .url();

    (auth_url.to_string(), pkce_verifier)
}

/// Generate the authorization URL specifically for the Web API OAuth flow.
/// Returns `(auth_url, pkce_verifier)`.
pub fn generate_webapi_auth_url(client_id: &str, redirect_port: u16) -> (String, PkceCodeVerifier) {
    generate_auth_url(client_id, redirect_port, WEBAPI_SCOPES)
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

    Ok(WebApiToken::from_response(&token_response, None))
}

/// Refresh a Web API token using a refresh token (no browser interaction needed).
/// Fails with `Error::OAuthInvalidGrant` once Spotify stops accepting the
/// refresh token (they expire 6 months after login).
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
        .map_err(|e| match e {
            RequestTokenError::ServerResponse(resp)
                if *resp.error() == BasicErrorResponseType::InvalidGrant =>
            {
                Error::OAuthInvalidGrant
            }
            e => Error::OAuthError(format!("Failed to refresh Web API token: {e}")),
        })?;

    Ok(WebApiToken::from_response(
        &token_response,
        Some(refresh_token_str),
    ))
}
