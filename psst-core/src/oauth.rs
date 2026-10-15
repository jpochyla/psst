use crate::error::Error;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, EndpointNotSet,
    EndpointSet, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RefreshToken, Scope,
    TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::{
    io::{BufRead, BufReader, Read, Write},
    net::TcpStream,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    sync::mpsc,
    time::{Duration, Instant},
};
use url::Url;

use crate::session::access_token::WEBAPI_SCOPES;

pub fn listen_for_callback_parameter(
    socket_address: SocketAddr,
    timeout: Duration,
    parameter_name: &'static str,
) -> Result<String, Error> {
    listen_for_callback(socket_address, timeout, parameter_name, None, || Ok(()))
}

fn listen_for_callback(
    socket_address: SocketAddr,
    timeout: Duration,
    parameter_name: &'static str,
    expected_state: Option<String>,
    on_ready: impl FnOnce() -> Result<(), Error>,
) -> Result<String, Error> {
    if !socket_address.ip().is_loopback() {
        return Err(Error::OAuthError(
            "OAuth callback must bind to loopback".into(),
        ));
    }
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

    listener.set_nonblocking(true)?;
    // Launch the browser only after the callback port is available.
    on_ready()?;
    let deadline = Instant::now() + timeout;
    // 2. Set up the channel for communication
    let (tx, rx) = mpsc::channel::<Result<String, Error>>();

    // 3. Spawn the thread. Loop so background requests (e.g. favicon.ico)
    //    don't consume the single accept and break the OAuth flow.
    let handle = std::thread::spawn(move || {
        while Instant::now() < deadline {
            match listener.accept() {
                Ok((mut stream, _)) => {
                    let _ = stream.set_read_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    if handle_callback_connection(
                        &mut stream,
                        &tx,
                        parameter_name,
                        expected_state.as_deref(),
                    ) {
                        break;
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    std::thread::sleep(Duration::from_millis(25));
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
    expected_state: Option<&str>,
) -> bool {
    let mut reader = BufReader::new((&mut *stream).take(8193));
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() || request_line.len() > 8192 {
        return false;
    }

    if let Some(expected) = expected_state {
        if !valid_oauth_callback(&request_line, expected) {
            send_not_found_response(stream);
            return false;
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
            log::warn!("ignoring invalid callback request");
            send_not_found_response(stream);
            false
        }
    }
}

fn valid_oauth_callback(line: &str, expected: &str) -> bool {
    let mut words = line.split_whitespace();
    if words.next() != Some("GET") {
        return false;
    }
    let Some(path) = words.next() else {
        return false;
    };
    if !path.starts_with("/login?") {
        return false;
    }
    let Ok(url) = Url::parse(&format!("http://localhost{path}")) else {
        return false;
    };
    let states: Vec<_> = url
        .query_pairs()
        .filter(|(key, _)| key == "state")
        .collect();
    states.len() == 1
        && ring::digest::digest(&ring::digest::SHA256, states[0].1.as_bytes()).as_ref()
            == ring::digest::digest(&ring::digest::SHA256, expected.as_bytes()).as_ref()
}

pub fn get_authcode_listener_with_state(
    socket_address: SocketAddr,
    timeout: Duration,
    authorization_url: &str,
) -> Result<AuthorizationCode, Error> {
    get_authcode_listener_with_state_and_ready(
        socket_address,
        timeout,
        authorization_url,
        || Ok(()),
    )
}

pub fn get_authcode_listener_with_state_and_ready(
    socket_address: SocketAddr,
    timeout: Duration,
    authorization_url: &str,
    on_ready: impl FnOnce() -> Result<(), Error>,
) -> Result<AuthorizationCode, Error> {
    let state = Url::parse(authorization_url)
        .ok()
        .and_then(|url| {
            url.query_pairs()
                .find(|(k, _)| k == "state")
                .map(|(_, v)| v.into_owned())
        })
        .ok_or_else(|| Error::OAuthError("Missing OAuth state".into()))?;
    listen_for_callback(socket_address, timeout, "code", Some(state), on_ready)
        .map(AuthorizationCode::new)
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

type SpotifyOAuthClient =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointNotSet, EndpointSet>;

fn http_client(request: oauth2::HttpRequest) -> Result<oauth2::HttpResponse, ureq::Error> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .max_redirects(0)
        .http_status_as_error(false)
        .build()
        .into();
    let response = agent.run(request)?;
    let (parts, mut body) = response.into_parts();
    let bytes = body.read_to_vec()?;
    Ok(ureq::http::Response::from_parts(parts, bytes))
}

fn create_oauth_client(client_id: &str, redirect_port: u16) -> SpotifyOAuthClient {
    let redirect_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let redirect_uri = format!("http://{redirect_address}/login");

    BasicClient::new(ClientId::new(client_id.to_string()))
        .set_auth_uri(AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap())
        .set_token_uri(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap())
        .set_redirect_uri(RedirectUrl::new(redirect_uri).expect("Invalid redirect URL"))
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
/// Spotify desktop playback uses its registered loopback callback and streaming scope.
pub fn generate_playback_auth_url() -> (String, PkceCodeVerifier) {
    generate_auth_url(
        crate::system_info::CLIENT_ID,
        8898,
        &[Scope::new("streaming".into())],
    )
}

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
        .request(&http_client)
        .map_err(|_| Error::OAuthError("No se pudo intercambiar el código de Spotify. Comprueba el Client ID y la dirección de retorno registrados.".into()))?;

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
        .request(&http_client)
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

#[cfg(test)]
mod callback_tests {
    use super::*;
    #[test]
    fn playback_uses_registered_desktop_callback_and_streaming_only() {
        let (authorization, _) = generate_playback_auth_url();
        let url = Url::parse(&authorization).unwrap();
        let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(params["client_id"], crate::system_info::CLIENT_ID);
        assert_eq!(params["redirect_uri"], "http://127.0.0.1:8898/login");
        assert_eq!(params["scope"], "streaming");
        assert_eq!(params["code_challenge_method"], "S256");
        assert!(!params["state"].is_empty());
    }

    #[test]
    fn configured_client_uses_registered_loopback_redirect_and_pkce() {
        let (authorization_url, _) = generate_webapi_auth_url("configured-client", 8888);
        let url = Url::parse(&authorization_url).unwrap();
        let parameters: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(parameters["client_id"], "configured-client");
        assert_eq!(parameters["redirect_uri"], "http://127.0.0.1:8888/login");
        assert_eq!(parameters["code_challenge_method"], "S256");
        assert!(parameters["scope"]
            .split_whitespace()
            .any(|scope| scope == "streaming"));
        assert!(!parameters["state"].is_empty());
        assert!(!parameters["code_challenge"].is_empty());
    }

    #[test]
    fn callback_listener_is_bound_before_browser_launch() {
        let reserve = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reserve.local_addr().unwrap();
        drop(reserve);
        let code = listen_for_callback(address, Duration::from_secs(2), "code", Some("expected".into()), || {
            assert!(TcpListener::bind(address).is_err());
            std::thread::spawn(move || {
                let mut stream = TcpStream::connect(address).unwrap();
                stream.write_all(b"GET /login?code=test-code&state=expected HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").unwrap();
                let mut response = String::new();
                stream.read_to_string(&mut response).unwrap();
                assert!(response.starts_with("HTTP/1.1 200"));
            });
            Ok(())
        }).unwrap();
        assert_eq!(code, "test-code");
    }

    #[test]
    fn occupied_callback_port_does_not_launch_browser() {
        let reserve = TcpListener::bind("127.0.0.1:0").unwrap();
        let mut launched = false;
        assert!(listen_for_callback(
            reserve.local_addr().unwrap(),
            Duration::from_millis(100),
            "code",
            Some("expected".into()),
            || {
                launched = true;
                Ok(())
            }
        )
        .is_err());
        assert!(!launched);
    }

    #[test]
    fn validates_state_method_path_and_duplicates() {
        assert!(valid_oauth_callback(
            "GET /login?code=abc&state=expected HTTP/1.1",
            "expected"
        ));
        assert!(!valid_oauth_callback(
            "GET /login?code=abc&state=attacker HTTP/1.1",
            "expected"
        ));
        assert!(!valid_oauth_callback(
            "GET /login?code=abc HTTP/1.1",
            "expected"
        ));
        assert!(!valid_oauth_callback(
            "POST /login?state=expected HTTP/1.1",
            "expected"
        ));
        assert!(!valid_oauth_callback(
            "GET /other?state=expected HTTP/1.1",
            "expected"
        ));
        assert!(!valid_oauth_callback(
            "GET /login?state=expected&state=expected HTTP/1.1",
            "expected"
        ));
    }
    #[test]
    fn callback_timeout_releases_port() {
        let reserve = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = reserve.local_addr().unwrap();
        drop(reserve);
        assert!(listen_for_callback(
            address,
            Duration::from_millis(100),
            "code",
            Some("expected".into()),
            || Ok(())
        )
        .is_err());
        std::thread::sleep(Duration::from_millis(50));
        assert!(TcpListener::bind(address).is_ok());
    }
}
