use std::{error, fmt, io};

#[derive(Debug)]
pub enum Error {
    SessionDisconnected,
    UnexpectedResponse,
    AudioFileNotFound,
    AuthFailed { code: i32 },
    JsonError(Box<dyn error::Error + Send>),
    AudioFetchingError(Box<dyn error::Error + Send>),
    AudioDecodingError(Box<dyn error::Error + Send>),
    AudioOutputError(Box<dyn error::Error + Send>),
    IoError(io::Error),
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionDisconnected => write!(f, "Session disconnected"),
            Self::UnexpectedResponse => write!(f, "Unknown server response"),
            Self::AudioFileNotFound => write!(f, "Audio file not found"),
            Self::AuthFailed { code } => match code {
                0 => write!(f, "Authentication failed: protocol error"),
                2 => write!(f, "Authentication failed: try another AP"),
                5 => write!(f, "Authentication failed: bad connection id"),
                9 => write!(f, "Authentication failed: travel restriction"),
                11 => write!(f, "Authentication failed: premium account required"),
                12 => write!(f, "Authentication failed: bad credentials"),
                13 => write!(f, "Authentication failed: could not validate credentials"),
                14 => write!(f, "Authentication failed: account exists"),
                15 => write!(f, "Authentication failed: extra verification required"),
                16 => write!(f, "Authentication failed: invalid app key"),
                17 => write!(f, "Authentication failed: application banned"),
                _ => write!(f, "Authentication failed with error code {code}", code = code)
            },
            Self::JsonError(err)
            | Self::AudioFetchingError(err)
            | Self::AudioDecodingError(err)
            | Self::AudioOutputError(err) => err.fmt(f),
            Self::IoError(err) => err.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::IoError(err)
    }
}
