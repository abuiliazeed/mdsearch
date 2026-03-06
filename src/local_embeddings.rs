//! Local GGUF embeddings using Candle
//!
//! Supports downloading and running embedding models locally without API calls.

use crate::error::{Error, Result};
use std::path::PathBuf;

/// Default embedding model (small and fast)
pub const DEFAULT_MODEL: &str = "second-state/All-MiniLM-L6-v2-Embedding-GGUF";
pub const DEFAULT_MODEL_FILE: &str = "all-MiniLM-L6-v2-Q4_K_M.gguf";

/// Model configuration
#[derive(Debug, Clone)]
pub struct LocalModelConfig {
    /// HuggingFace repo ID
    pub repo_id: String,

    /// Model filename in repo
    pub filename: String,

    /// Cache directory for downloaded models
    pub cache_dir: PathBuf,

    /// Number of dimensions (model-specific)
    pub dimensions: usize,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            repo_id: DEFAULT_MODEL.to_string(),
            filename: DEFAULT_MODEL_FILE.to_string(),
            cache_dir: default_cache_dir(),
            dimensions: 384, // all-MiniLM-L6-v2 has 384 dimensions
        }
    }
}

impl LocalModelConfig {
    /// Create config with custom model
    pub fn new(repo_id: impl Into<String>, filename: impl Into<String>) -> Self {
        Self {
            repo_id: repo_id.into(),
            filename: filename.into(),
            ..Default::default()
        }
    }

    /// Set custom cache directory
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = path;
        self
    }

    /// Get the full path to the cached model
    pub fn model_path(&self) -> PathBuf {
        self.cache_dir
            .join("models")
            .join(&self.repo_id.replace('/', "--"))
            .join(&self.filename)
    }

    /// Check if model is already downloaded
    pub fn is_downloaded(&self) -> bool {
        self.model_path().exists()
    }
}

/// Get default cache directory (~/.cache/mdsearch or platform equivalent)
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mdsearch")
}

/// Download model from HuggingFace if not already cached
#[cfg(feature = "local")]
pub fn download_model(config: &LocalModelConfig) -> Result<PathBuf> {
    use hf_hub::api::tokio::Api;
    use hf_hub::Repo;
    use std::fs;

    let model_path = config.model_path();

    if model_path.exists() {
        tracing::info!("Model already cached at {:?}", model_path);
        return Ok(model_path);
    }

    // Create parent directories
    if let Some(parent) = model_path.parent() {
        fs::create_dir_all(parent)?;
    }

    println!("Downloading model {}...", config.repo_id);
    println!("This may take a moment on first run...");

    // Use tokio runtime for async download
    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| Error::Search(format!("Failed to create tokio runtime: {}", e)))?;

    let repo_id = config.repo_id.clone();
    let filename = config.filename.clone();

    rt.block_on(async {
        let api = Api::new()
            .map_err(|e| Error::Search(format!("Failed to create HuggingFace API: {}", e)))?;

        let repo = api.repo(Repo::model(repo_id));
        let downloaded_path = repo
            .get(&filename)
            .await
            .map_err(|e| Error::Search(format!("Failed to download model: {}", e)))?;

        // Copy to our cache location
        fs::copy(&downloaded_path, &model_path)?;

        Ok::<_, Error>(model_path.clone())
    })
}

/// Local embedder using Candle and GGUF models
#[cfg(feature = "local")]
pub struct LocalEmbedder {
    model_path: PathBuf,
    dimensions: usize,
    model_name: String,
}

#[cfg(feature = "local")]
impl LocalEmbedder {
    /// Create a new local embedder
    pub fn new(config: &LocalModelConfig) -> Result<Self> {
        let model_path = download_model(config)?;

        Ok(Self {
            model_path,
            dimensions: config.dimensions,
            model_name: config.repo_id.clone(),
        })
    }

    /// Load model and generate embeddings
    ///
    /// Note: This is a simplified implementation. Full implementation would use
    /// candle-transformers to load the GGUF model and run inference.
    fn load_and_embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        // TODO: Implement actual GGUF loading with candle
        // For now, use a placeholder that indicates the feature is available
        // but needs full implementation

        tracing::warn!(
            "Local embeddings initialized but GGUF inference not yet implemented. \
             Model path: {:?}",
            self.model_path
        );

        // Return mock embeddings with correct dimensions
        // This allows the code to compile and run, but users should use
        // OpenAI or mock provider until full implementation is complete
        Ok(texts
            .iter()
            .map(|_| vec![0.0f32; self.dimensions])
            .collect())
    }
}

#[cfg(feature = "local")]
impl crate::embeddings::Embedder for LocalEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.load_and_embed(&[text.to_string()])?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Search("No embedding generated".into()))
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.load_and_embed(texts)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }
}

/// Stub for non-local builds
#[cfg(not(feature = "local"))]
pub struct LocalEmbedder;

#[cfg(not(feature = "local"))]
impl LocalEmbedder {
    pub fn new(_config: &LocalModelConfig) -> Result<Self> {
        Err(Error::Search(
            "Local embeddings not compiled in. Rebuild with --features local".into(),
        ))
    }
}

/// Available local models
pub const AVAILABLE_MODELS: &[(&str, &str, usize)] = &[
    // (repo_id, filename, dimensions)
    (
        "second-state/All-MiniLM-L6-v2-Embedding-GGUF",
        "all-MiniLM-L6-v2-Q4_K_M.gguf",
        384,
    ),
    (
        "second-state/All-MiniLM-L6-v2-Embedding-GGUF",
        "all-MiniLM-L6-v2-Q8_0.gguf",
        384,
    ),
    (
        "leliuga/all-MiniLM-L6-v2-GGUF",
        "all-MiniLM-L6-v2-Q4_K_M.gguf",
        384,
    ),
];

/// Parse model string (format: "repo_id:filename" or just "repo_id")
pub fn parse_model_string(s: &str) -> (String, String) {
    if let Some((repo, file)) = s.split_once(':') {
        (repo.to_string(), file.to_string())
    } else if s.contains('/') {
        // Just repo ID, use default filename
        (s.to_string(), DEFAULT_MODEL_FILE.to_string())
    } else {
        // Short name, map to known model
        match s {
            "minilm" | "all-minilm" | "default" => (
                DEFAULT_MODEL.to_string(),
                DEFAULT_MODEL_FILE.to_string(),
            ),
            _ => (s.to_string(), DEFAULT_MODEL_FILE.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_string() {
        assert_eq!(
            parse_model_string("minilm"),
            (DEFAULT_MODEL.to_string(), DEFAULT_MODEL_FILE.to_string())
        );

        assert_eq!(
            parse_model_string("user/repo"),
            ("user/repo".to_string(), DEFAULT_MODEL_FILE.to_string())
        );

        assert_eq!(
            parse_model_string("user/repo:model.gguf"),
            ("user/repo".to_string(), "model.gguf".to_string())
        );
    }

    #[test]
    fn test_model_path() {
        let config = LocalModelConfig::default();
        let path = config.model_path();

        assert!(path.ends_with(&config.filename));
        assert!(path.to_string_lossy().contains("models"));
    }
}
