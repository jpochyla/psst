use std::{error, fmt};

use druid::Data;

#[derive(Clone, Debug, Data)]
pub enum Error {
    WebApiError(String),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WebApiError(err) => f.write_str(err),
        }
    }
}

impl From<psst_core::error::Error> for Error {
    fn from(err: psst_core::error::Error) -> Self {
        Self::WebApiError(err.to_string())
    }
}
