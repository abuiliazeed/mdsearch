//! mdsearch - Blazingly fast markdown search and RAG indexing
//!
//! A high-performance CLI tool for searching massive markdown file collections
//! with sub-millisecond response times and RAG-ready output.

pub mod chunk;
pub mod config;
pub mod embeddings;
pub mod error;
pub mod index;
pub mod local_embeddings;
pub mod parser;
pub mod search;
pub mod store;

pub use chunk::{Chunk, Chunker};
pub use config::Config;
pub use embeddings::{Embedder, EmbeddingConfig, EmbeddingProvider};
pub use error::{Error, Result};
pub use index::Indexer;
pub use search::{SearchResult, Searcher};
pub use store::Store;

/// Prelude for common imports
pub mod prelude {
    pub use crate::{
        Chunk, Chunker, Config, Embedder, EmbeddingConfig, EmbeddingProvider, Error, Indexer,
        Result, SearchResult, Searcher, Store,
    };
}
