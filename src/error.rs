use std::io;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Connection error")]
    ConnectionError(#[from] io::Error),
    #[error("Parse error")]
    ParseError,
}

pub type Result<T> = std::result::Result<T, Error>;
