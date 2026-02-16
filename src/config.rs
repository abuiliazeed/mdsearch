//! Configuration for mdsearch

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Indexing settings
    pub indexing: IndexingConfig,

    /// Search settings
    pub search: SearchConfig,

    /// Chunking settings for RAG
    pub chunking: ChunkingConfig,

    /// Storage settings
    pub storage: StorageConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            indexing: IndexingConfig::default(),
            search: SearchConfig::default(),
            chunking: ChunkingConfig::default(),
            storage: StorageConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexingConfig {
    /// Number of parallel threads (0 = auto)
    pub threads: usize,

    /// File patterns to include
    pub include_patterns: Vec<String>,

    /// File patterns to exclude
    pub exclude_patterns: Vec<String>,

    /// Respect .gitignore files
    pub respect_gitignore: bool,

    /// Maximum file size in bytes (0 = unlimited)
    pub max_file_size: u64,

    /// Follow symlinks
    pub follow_symlinks: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        Self {
            threads: 0, // auto
            include_patterns: vec!["**/*.md".into(), "**/*.markdown".into()],
            exclude_patterns: vec!["**/node_modules/**".into(), "**/.git/**".into()],
            respect_gitignore: true,
            max_file_size: 10 * 1024 * 1024, // 10MB
            follow_symlinks: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchConfig {
    /// Default result limit
    pub default_limit: usize,

    /// Enable hybrid search (keyword + semantic)
    pub hybrid: bool,

    /// BM25 weight in hybrid search (0.0 - 1.0)
    pub bm25_weight: f32,

    /// Vector weight in hybrid search (0.0 - 1.0)
    pub vector_weight: f32,

    /// Minimum score threshold
    pub min_score: f32,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            default_limit: 10,
            hybrid: true,
            bm25_weight: 0.3,
            vector_weight: 0.7,
            min_score: 0.1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkingConfig {
    /// Target chunk size in characters
    pub target_size: usize,

    /// Overlap between chunks
    pub overlap: usize,

    /// Respect markdown structure (don't split mid-section)
    pub respect_structure: bool,

    /// Include code blocks in chunks
    pub include_code_blocks: bool,

    /// Include frontmatter metadata
    pub include_frontmatter: bool,
}

impl Default for ChunkingConfig {
    fn default() -> Self {
        Self {
            target_size: 512,
            overlap: 50,
            respect_structure: true,
            include_code_blocks: true,
            include_frontmatter: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// Path to index database
    pub index_path: PathBuf,

    /// Enable compression
    pub compress: bool,

    /// Cache size in bytes
    pub cache_size: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            index_path: PathBuf::from(".mdsearch"),
            compress: true,
            cache_size: 256 * 1024 * 1024, // 256MB
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load(path: &std::path::Path) -> crate::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    /// Save configuration to file
    pub fn save(&self, path: &std::path::Path) -> crate::Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
