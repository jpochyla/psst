// Ported from librespot

use std::time::{Duration, Instant};

const EXPIRY_THRESHOLD: Duration = Duration::from_secs(10);

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

impl Token {
    pub fn is_expired(&self) -> bool {
        self.timestamp + (self.expires_in.saturating_sub(EXPIRY_THRESHOLD)) < Instant::now()
    }
}