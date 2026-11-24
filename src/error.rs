use std::io;

use thiserror::Error;

use crate::http::Status;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Connection error")]
    ConnectionError(#[from] io::Error),
    #[error("Serialization error")]
    SerializationError,
    #[error("Parse error {0}")]
    ParseError(String),
    #[error("Status return {0}")]
    Status(Status),
}

pub type Result<T> = std::result::Result<T, Error>;
