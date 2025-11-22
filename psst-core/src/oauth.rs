use crate::error::Error;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, CsrfToken, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,

    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    sync::mpsc,
    time::Duration,
};
use url::Url;

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

pub fn generate_auth_url(redirect_port: u16) -> (String, PkceCodeVerifier) {
    let redirect_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let redirect_uri = format!("http://{redirect_address}/login");

    let client = BasicClient::new(
        ClientId::new(crate::session::access_token::CLIENT_ID.to_string()),
    )
    .set_auth_uri(AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap())
    .set_token_uri(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap())
    .set_redirect_uri(RedirectUrl::new(redirect_uri).expect("Invalid redirect URL"));
    
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
    let redirect_address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), redirect_port);
    let redirect_uri = format!("http://{redirect_address}/login");

    let client = BasicClient::new(
        ClientId::new(crate::session::access_token::CLIENT_ID.to_string()),
    )
    .set_auth_uri(AuthUrl::new("https://accounts.spotify.com/authorize".to_string()).unwrap())
    .set_token_uri(TokenUrl::new("https://accounts.spotify.com/api/token".to_string()).unwrap())
    .set_redirect_uri(RedirectUrl::new(redirect_uri).expect("Invalid redirect URL"));

    let token_response = client
        .exchange_code(code)
        .set_pkce_verifier(pkce_verifier)
        .request(&|request: oauth2::HttpRequest| {
            let method = request.method().as_str();
            let url = request.uri().to_string();

            let agent = ureq::Agent::new_with_defaults();

            let result = match method {
                "POST" => {
                    let mut builder = agent.post(&url);
                    for (name, value) in request.headers().iter() {
                        if let Ok(v) = value.to_str() {
                            builder = builder.header(name.as_str(), v);
                        }
                    }
                    builder.send(request.body())
                }
                "GET" => {
                    let mut builder = agent.get(&url);
                    for (name, value) in request.headers().iter() {
                        if let Ok(v) = value.to_str() {
                            builder = builder.header(name.as_str(), v);
                        }
                    }
                    builder.call()
                }
                _ => return Err(Error::OAuthError(format!("Unsupported method: {}", method))),
            };

            let response = result.map_err(|e| Error::OAuthError(e.to_string()))?;

            let mut response_builder = oauth2::http::Response::builder()
                .status(response.status())
                .version(response.version());

            for (name, value) in response.headers().iter() {
                response_builder = response_builder.header(name, value);
            }

            let mut body = Vec::new();
            use std::io::Read;
            response
                .into_body()
                .into_reader()
                .read_to_end(&mut body)
                .map_err(|e| Error::OAuthError(e.to_string()))?;

            response_builder
                .body(body)
                .map_err(|e| Error::OAuthError(e.to_string()))
        })
        .expect("Failed to exchange code for token");

    token_response.access_token().secret().to_string()
}

fn get_scopes() -> Vec<Scope> {
    crate::session::access_token::ACCESS_SCOPES
        .split(',')
        .map(|s| Scope::new(s.trim().to_string()))
        .collect()
}
