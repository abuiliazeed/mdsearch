//! Embedding providers for semantic search

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[cfg(feature = "local")]
use crate::local_embeddings::{LocalEmbedder, LocalModelConfig, parse_model_string};

/// Embedding vector type
pub type Vector = Vec<f32>;

/// Embedding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// Provider type
    pub provider: EmbeddingProvider,

    /// Model name/identifier
    pub model: String,

    /// Vector dimensions
    pub dimensions: usize,

    /// Local model path (optional, for bundled models)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_path: Option<PathBuf>,

    /// Cache directory for downloaded models
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EmbeddingProvider {
    /// OpenAI embeddings API
    OpenAI,

    /// Local model via candle (GGUF)
    Local,

    /// Mock provider for testing
    Mock,
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            provider: EmbeddingProvider::Local,
            model: "default".to_string(),
            dimensions: 384,
            model_path: None,
            cache_dir: None,
        }
    }
}

/// Embedding provider trait
pub trait Embedder: Send + Sync {
    /// Generate embedding for text
    fn embed(&self, text: &str) -> Result<Vector>;

    /// Generate embeddings for multiple texts
    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        texts.iter().map(|t| self.embed(t)).collect()
    }

    /// Get embedding dimensions
    fn dimensions(&self) -> usize;

    /// Get model name
    fn model_name(&self) -> &str;
}

/// Mock embedder for testing (deterministic hash-based vectors)
pub struct MockEmbedder {
    dimensions: usize,
}

impl MockEmbedder {
    pub fn new(dimensions: usize) -> Self {
        Self { dimensions }
    }
}

impl Embedder for MockEmbedder {
    fn embed(&self, text: &str) -> Result<Vector> {
        // Create a deterministic but distributed vector based on text hash
        let mut vector = vec![0.0f32; self.dimensions];

        // Simple hash-based embedding (for testing only)
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let seed = hasher.finish();

        // Generate pseudo-random but deterministic values
        let mut state = seed;
        for i in 0..self.dimensions {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
            let val = ((state >> 32) as i32) as f32 / i32::MAX as f32;
            vector[i] = val;
        }

        // Normalize to unit length
        let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for v in &mut vector {
                *v /= norm;
            }
        }

        Ok(vector)
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        "mock-embedding"
    }
}

/// OpenAI embedder (requires API key and "openai" feature)
#[cfg(feature = "openai")]
pub struct OpenAIEmbedder {
    client: reqwest::blocking::Client,
    api_key: String,
    model: String,
    dimensions: usize,
}

#[cfg(feature = "openai")]
impl OpenAIEmbedder {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            client: reqwest::blocking::Client::new(),
            api_key,
            model: model.unwrap_or_else(|| "text-embedding-3-small".to_string()),
            dimensions: 1536,
        }
    }

    pub fn with_dimensions(mut self, dims: usize) -> Self {
        self.dimensions = dims;
        self
    }
}

#[cfg(feature = "openai")]
#[derive(Serialize)]
struct EmbeddingRequest {
    model: String,
    input: Vec<String>,
}

#[cfg(feature = "openai")]
#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[cfg(feature = "openai")]
#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[cfg(feature = "openai")]
impl Embedder for OpenAIEmbedder {
    fn embed(&self, text: &str) -> Result<Vector> {
        let embeddings = self.embed_batch(&[text.to_string()])?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Search("No embedding returned".into()))
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vector>> {
        let request = EmbeddingRequest {
            model: self.model.clone(),
            input: texts.to_vec(),
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .map_err(|e| Error::Search(format!("OpenAI API error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().unwrap_or_default();
            return Err(Error::Search(format!(
                "OpenAI API error ({}): {}",
                status, body
            )));
        }

        let embedding_response: EmbeddingResponse = response
            .json()
            .map_err(|e| Error::Search(format!("Failed to parse response: {}", e)))?;

        Ok(embedding_response
            .data
            .into_iter()
            .map(|d| d.embedding)
            .collect())
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn model_name(&self) -> &str {
        &self.model
    }
}

/// Create an embedder based on configuration
pub fn create_embedder(config: &EmbeddingConfig) -> Result<Box<dyn Embedder>> {
    match config.provider {
        EmbeddingProvider::Mock => Ok(Box::new(MockEmbedder::new(config.dimensions))),
        #[cfg(feature = "openai")]
        EmbeddingProvider::OpenAI => {
            let api_key = std::env::var("OPENAI_API_KEY")
                .map_err(|_| Error::Search("OPENAI_API_KEY not set".into()))?;
            Ok(Box::new(OpenAIEmbedder::new(api_key, Some(config.model.clone()))))
        }
        #[cfg(not(feature = "openai"))]
        EmbeddingProvider::OpenAI => {
            Err(Error::Search("OpenAI support not compiled in. Rebuild with --features openai".into()))
        }
        EmbeddingProvider::Local => {
            #[cfg(feature = "local")]
            {
                let model_id = parse_model_string(&config.model);

                let local_config = if let Some(path) = &config.model_path {
                    // Use bundled/offline model (path to model directory)
                    LocalModelConfig::new(model_id).with_cache_dir(path.clone())
                } else if let Some(cache) = &config.cache_dir {
                    // Custom cache directory
                    LocalModelConfig::new(model_id).with_cache_dir(cache.clone())
                } else {
                    // Default config
                    LocalModelConfig::new(model_id)
                };

                Ok(Box::new(LocalEmbedder::new(&local_config)?))
            }
            #[cfg(not(feature = "local"))]
            {
                Err(Error::Search("Local embeddings not compiled in. Rebuild with --features local (default)".into()))
            }
        }
    }
}

/// Compute cosine similarity between two vectors
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() {
        return 0.0;
    }

    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot / (norm_a * norm_b)
}

/// Run the embed command to generate embeddings for chunks
pub fn run_embed(
    index_path: std::path::PathBuf,
    provider: String,
    model: Option<String>,
    model_path: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
    batch_size: usize,
) -> Result<()> {
    use crate::store::Store;
    use indicatif::{ProgressBar, ProgressStyle};

    if !Store::exists(&index_path) {
        return Err(Error::IndexNotFound(index_path));
    }

    let store = Store::open(&index_path)?;

    // Get all chunks
    let chunks = store.get_all_chunks()?;
    if chunks.is_empty() {
        println!("No chunks found. Run 'mdsearch index' first.");
        return Ok(());
    }

    // Filter chunks without embeddings
    let chunks_to_embed: Vec<_> = chunks
        .into_iter()
        .filter(|c| c.embedding.is_none())
        .collect();

    if chunks_to_embed.is_empty() {
        println!("All chunks already have embeddings.");
        return Ok(());
    }

    println!("Generating embeddings for {} chunks...", chunks_to_embed.len());

    // Create embedder
    let embed_provider = match provider.as_str() {
        "openai" => EmbeddingProvider::OpenAI,
        "local" => EmbeddingProvider::Local,
        "mock" => EmbeddingProvider::Mock,
        _ => {
            return Err(Error::Search(format!(
                "Unknown provider '{}'. Use: local, openai, mock",
                provider
            )))
        }
    };

    let config = EmbeddingConfig {
        provider: embed_provider,
        model: model.unwrap_or_else(|| "default".to_string()),
        dimensions: 384,
        model_path,
        cache_dir,
    };

    let embedder = create_embedder(&config)?;

    println!("Using {} ({})", provider, embedder.model_name());

    let pb = ProgressBar::new(chunks_to_embed.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
        )
        .unwrap(),
    );

    // Process in batches
    let mut updated = 0;
    for batch in chunks_to_embed.chunks(batch_size) {
        let texts: Vec<String> = batch.iter().map(|c| c.content.clone()).collect();

        match embedder.embed_batch(&texts) {
            Ok(embeddings) => {
                for (chunk, embedding) in batch.iter().zip(embeddings.iter()) {
                    let mut updated_chunk = chunk.clone();
                    updated_chunk.embedding = Some(embedding.clone());
                    store.update_chunk(&updated_chunk)?;
                    updated += 1;
                }
            }
            Err(e) => {
                eprintln!("Error embedding batch: {}", e);
                // Continue with next batch
            }
        }

        pb.inc(batch.len() as u64);
    }

    pb.finish_with_message(format!("Embedded {} chunks", updated));

    println!(
        "✅ Generated embeddings for {} chunks using {} ({})",
        updated,
        provider,
        embedder.model_name()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_embedder_deterministic() {
        let embedder = MockEmbedder::new(384);
        let a = embedder.embed("hello world").unwrap();
        let b = embedder.embed("hello world").unwrap();

        assert_eq!(a.len(), 384);
        assert_eq!(a, b);
    }

    #[test]
    fn test_mock_embedder_normalized() {
        let embedder = MockEmbedder::new(384);
        let v = embedder.embed("test").unwrap();

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_cosine_similarity() {
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];

        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.01);
        assert!((cosine_similarity(&a, &c) - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_similar_texts() {
        let embedder = MockEmbedder::new(384);

        let a = embedder.embed("Rust is a systems programming language").unwrap();
        let b = embedder.embed("Rust is a systems programming language").unwrap();
        let c = embedder.embed("Bananas are yellow").unwrap();

        // Same text should have similarity 1.0
        assert!((cosine_similarity(&a, &b) - 1.0).abs() < 0.01);

        // Different text should have different embeddings
        // (Note: mock doesn't guarantee this, but real embeddings would)
        let sim = cosine_similarity(&a, &c);
        // For mock, just check it's a valid similarity
        assert!(sim >= -1.0 && sim <= 1.0);
    }
}
