use std::{error, fmt};

#[derive(Debug)]
pub enum Error {
    WebApiError(Box<dyn error::Error + Send>),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Self::WebApiError(e) => e.fmt(f),
        }
    }
}
