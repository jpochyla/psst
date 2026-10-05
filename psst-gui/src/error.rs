use std::{error, fmt};

use druid::Data;

#[derive(Clone, Debug, Data)]
pub enum Error {
    WebApiError(String),
    RateLimited { retry_at: u64, remaining: u64 },
}

impl Error {
    pub fn rate_limited(retry_at: u64) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self::RateLimited {
            retry_at,
            remaining: retry_at.saturating_sub(now),
        }
    }
    pub fn refresh_countdown(&mut self) {
        if let Self::RateLimited { retry_at, .. } = self {
            *self = Self::rate_limited(*retry_at);
        }
    }
    pub fn retry_blocked(&self) -> bool {
        matches!(self, Self::RateLimited { remaining, .. } if *remaining > 0)
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WebApiError(err) => f.write_str(err),
            Self::RateLimited { remaining, .. } => write!(f,
                "Spotify limitó las solicitudes (HTTP 429). Reintento disponible en {} h {} min {} s. Los datos guardados y la reproducción local siguen disponibles.",
                remaining / 3600, (remaining % 3600) / 60, remaining % 60),
        }
    }
}
