//! Error types for mdsearch

use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Index error: {0}")]
    Index(String),

    #[error("Search error: {0}")]
    Search(String),

    #[error("Storage error: {0}")]
    Storage(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("File not found: {0}")]
    FileNotFound(std::path::PathBuf),

    #[error("Index not found at {0}. Run 'mdsearch index' first.")]
    IndexNotFound(std::path::PathBuf),

    #[error("Invalid query: {0}")]
    InvalidQuery(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("RocksDB error: {0}")]
    RocksDb(String),

    #[error("Watch error: {0}")]
    Watch(String),

    #[error("Cancelled by user")]
    Cancelled,
}

impl From<rocksdb::Error> for Error {
    fn from(err: rocksdb::Error) -> Self {
        Error::RocksDb(err.to_string())
    }
}
