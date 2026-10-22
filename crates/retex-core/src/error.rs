use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid zip container: {0}")]
    Zip(String),

    #[error("invalid package: {0}")]
    Package(String),

    #[error("zip crypto check failed (wrong password or corrupt entry)")]
    Decrypt,

    #[error("crc mismatch for entry {name}")]
    Crc { name: String },

    #[error("entry not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;
