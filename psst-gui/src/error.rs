use std::{error, fmt};

use druid::Data;

#[derive(Clone, Debug, Data)]
pub enum Error {
    WebApiError(String),
    /// A request that came back with a non-success HTTP status.  Kept apart
    /// from `WebApiError` so callers can react to a specific code, such as
    /// falling back to another endpoint on 404.
    WebApiStatus(u16),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WebApiError(err) => f.write_str(err),
            Self::WebApiStatus(code) => write!(f, "HTTP status {code}"),
        }
    }
}
