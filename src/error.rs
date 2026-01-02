use thiserror::Error;

#[derive(Error, Debug)]
pub enum RsortError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Invalid key specification: {0}")]
    InvalidKey(String),

    #[error("Invalid field delimiter: must be a single byte")]
    InvalidDelimiter,
}

pub type Result<T> = std::result::Result<T, RsortError>;
