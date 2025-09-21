use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct Token {
    pub access_token: String,
    pub expires_in: Duration,
    pub token_type: TokenType,
    pub token_type_s: String,
    pub scopes: Vec<String>,
    pub timestamp: Instant,
}

#[derive(Clone, Debug)]
pub enum TokenType {
    AuthToken,
    ClientToken,
}
