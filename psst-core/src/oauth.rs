use oauth2::{
    basic::BasicClient, reqwest::http_client, AuthUrl, AuthorizationCode, ClientId, CsrfToken,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpStream,
    net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener},
    sync::mpsc,
    time::Duration,
};
use url::Url;

pub fn get_authcode_listener(
    socket_address: SocketAddr,
    timeout: Duration,
) -> Result<AuthorizationCode, String> {
    log::info!("starting OAuth listener on {:?}", socket_address);
    let listener = TcpListener::bind(socket_address)
        .map_err(|e| format!("Failed to bind to address: {}", e))?;
    log::info!("listener bound successfully");

    let (tx, rx) = mpsc::channel();

    let handle = std::thread::spawn(move || {
        if let Ok((mut stream, _)) = listener.accept() {
            handle_connection(&mut stream, &tx);
        }
    });

    let result = rx
        .recv_timeout(timeout)
        .map_err(|_| "Timed out waiting for authorization code".to_string())?;

    handle
        .join()
        .map_err(|_| "Failed to join server thread".to_string())?;

    result
}

fn handle_connection(stream: &mut TcpStream, tx: &mpsc::Sender<Result<AuthorizationCode, String>>) {
    let mut reader = BufReader::new(&mut *stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_ok() {
        if let Some(code) = extract_code_from_request(&request_line) {
            send_success_response(stream);
            let _ = tx.send(Ok(code));
        } else {
            let _ = tx.send(Err("Failed to extract code from request".to_string()));
        }
    }
}

fn extract_code_from_request(request_line: &str) -> Option<AuthorizationCode> {
    request_line.split_whitespace().nth(1).and_then(|path| {
        Url::parse(&format!("http://localhost{}", path))
            .ok()?
            .query_pairs()
            .find(|(key, _)| key == "code")
            .map(|(_, code)| AuthorizationCode::new(code.into_owned()))
    })
}

fn send_success_response(stream: &mut TcpStream) {
    let response =
        "HTTP/1.1 200 OK\r\n\r\n<html><body>You can close this window now.</body></html>";
    let _ = stream.write_all(response.as_bytes());
}

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
