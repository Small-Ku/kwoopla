use std::fmt;

#[derive(Debug)]
pub enum Error {
    InvalidMac(String),
    Truncated(&'static str),
    Invalid(&'static str),
    Io(std::io::Error),
    Transport(String),
    UnsupportedPlatform,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMac(s) => write!(f, "invalid MAC address: {s}"),
            Self::Truncated(s) => write!(f, "truncated {s}"),
            Self::Invalid(s) => write!(f, "invalid {s}"),
            Self::Io(e) => e.fmt(f),
            Self::Transport(e) => f.write_str(e),
            Self::UnsupportedPlatform => {
                write!(f, "AF_PACKET transport is only available on Linux")
            }
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
