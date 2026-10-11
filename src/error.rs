#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid MAC address: {0}")]
    InvalidMac(String),
    #[error("truncated {0}")]
    Truncated(&'static str),
    #[error("invalid {0}")]
    Invalid(&'static str),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Transport(String),
    #[error("AF_PACKET transport is only available on Linux")]
    UnsupportedPlatform,
}
