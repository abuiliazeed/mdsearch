//! Local embeddings using Candle transformers
//!
//! Supports downloading and running embedding models locally without API calls.

use crate::error::{Error, Result};
use std::path::PathBuf;

/// Default embedding model (small and fast)
pub const DEFAULT_MODEL: &str = "sentence-transformers/all-MiniLM-L6-v2";

/// Model configuration
#[derive(Debug, Clone)]
pub struct LocalModelConfig {
    /// HuggingFace model ID
    pub model_id: String,

    /// Cache directory for downloaded models
    pub cache_dir: PathBuf,

    /// Number of dimensions (model-specific)
    pub dimensions: usize,
}

impl Default for LocalModelConfig {
    fn default() -> Self {
        Self {
            model_id: DEFAULT_MODEL.to_string(),
            cache_dir: default_cache_dir(),
            dimensions: 384, // all-MiniLM-L6-v2 has 384 dimensions
        }
    }
}

impl LocalModelConfig {
    /// Create config with custom model
    pub fn new(model_id: impl Into<String>) -> Self {
        Self {
            model_id: model_id.into(),
            ..Default::default()
        }
    }

    /// Set custom cache directory
    pub fn with_cache_dir(mut self, path: PathBuf) -> Self {
        self.cache_dir = path;
        self
    }
}

/// Get default cache directory (~/.cache/mdsearch or platform equivalent)
pub fn default_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("mdsearch")
}

/// Local embedder using Candle transformers
#[cfg(feature = "local")]
pub struct LocalEmbedder {
    model: candle_transformers::models::bert::BertModel,
    tokenizer: tokenizers::Tokenizer,
    device: candle_core::Device,
    #[allow(dead_code)]
    dimensions: usize,
    model_name: String,
}

#[cfg(feature = "local")]
impl LocalEmbedder {
    /// Create a new local embedder
    pub fn new(config: &LocalModelConfig) -> Result<Self> {
        use candle_core::Device;
        use candle_nn::VarBuilder;
        use candle_transformers::models::bert::{BertModel, Config, DTYPE};
        use hf_hub::api::sync::Api;
        use hf_hub::Repo;
        use tokenizers::Tokenizer;

        println!("Loading model {}...", config.model_id);

        // Download model files using sync API
        let api = Api::new()
            .map_err(|e| Error::Search(format!("Failed to create HuggingFace API: {}", e)))?;

        let api_repo = api.repo(Repo::model(config.model_id.clone()));

        let config_path = api_repo
            .get("config.json")
            .map_err(|e| Error::Search(format!("Failed to download config: {}", e)))?;

        let tokenizer_path = api_repo
            .get("tokenizer.json")
            .map_err(|e| Error::Search(format!("Failed to download tokenizer: {}", e)))?;

        let model_path = api_repo
            .get("model.safetensors")
            .map_err(|e| Error::Search(format!("Failed to download model: {}", e)))?;

        // Load config
        let config_content = std::fs::read_to_string(&config_path)?;
        let bert_config: Config = serde_json::from_str(&config_content)
            .map_err(|e| Error::Search(format!("Failed to parse config: {}", e)))?;

        // Load tokenizer
        let mut tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| Error::Search(format!("Failed to load tokenizer: {}", e)))?;

        // Configure tokenizer for batch processing
        use tokenizers::PaddingParams;
        let pp = PaddingParams {
            strategy: tokenizers::PaddingStrategy::BatchLongest,
            ..Default::default()
        };
        tokenizer.with_padding(Some(pp));

        // Use CPU (can add GPU support later with Metal/CUDA features)
        let device = Device::Cpu;

        // Load model weights
        let vb = unsafe {
            VarBuilder::from_mmaped_safetensors(&[&model_path], DTYPE, &device)
                .map_err(|e| Error::Search(format!("Failed to load model weights: {}", e)))?
        };

        // Build model
        let model = BertModel::load(vb, &bert_config)
            .map_err(|e| Error::Search(format!("Failed to build model: {}", e)))?;

        println!("✅ Model loaded successfully");

        Ok(Self {
            model,
            tokenizer,
            device,
            dimensions: config.dimensions,
            model_name: config.model_id.clone(),
        })
    }

    /// Generate embeddings for texts
    fn encode(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        use candle_core::Tensor;
        use candle_transformers::models::bert::DTYPE;

        // Tokenize batch
        let tokens = self
            .tokenizer
            .encode_batch(texts.to_vec(), true)
            .map_err(|e| Error::Search(format!("Tokenization error: {}", e)))?;

        // Create token ID tensors
        let token_ids: Vec<Tensor> = tokens
            .iter()
            .map(|t| {
                let ids = t.get_ids().to_vec();
                Tensor::new(ids.as_slice(), &self.device)
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| Error::Search(format!("Failed to create token tensors: {}", e)))?;

        // Create attention mask tensors
        let attention_masks: Vec<Tensor> = tokens
            .iter()
            .map(|t| {
                let mask = t.get_attention_mask().to_vec();
                Tensor::new(mask.as_slice(), &self.device)
            })
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| {
                Error::Search(format!("Failed to create attention mask tensors: {}", e))
            })?;

        // Stack into batches
        let token_ids = Tensor::stack(&token_ids, 0)
            .map_err(|e| Error::Search(format!("Failed to stack token IDs: {}", e)))?;

        let attention_mask = Tensor::stack(&attention_masks, 0)
            .map_err(|e| Error::Search(format!("Failed to stack attention masks: {}", e)))?;

        let token_type_ids = token_ids
            .zeros_like()
            .map_err(|e| Error::Search(format!("Failed to create token type IDs: {}", e)))?;

        // Run forward pass
        let embeddings = self
            .model
            .forward(&token_ids, &token_type_ids, Some(&attention_mask))
            .map_err(|e| Error::Search(format!("Model forward pass error: {}", e)))?;

        // Apply mean pooling with attention mask
        let attention_mask_for_pooling = attention_mask
            .to_dtype(DTYPE)
            .map_err(|e| Error::Search(format!("Failed to convert attention mask dtype: {}", e)))?
            .unsqueeze(2)
            .map_err(|e| Error::Search(format!("Failed to unsqueeze attention mask: {}", e)))?;

        let sum_mask = attention_mask_for_pooling
            .sum(1)
            .map_err(|e| Error::Search(format!("Failed to sum attention mask: {}", e)))?;

        let pooled_embeddings = (embeddings
            .broadcast_mul(&attention_mask_for_pooling)
            .map_err(|e| Error::Search(format!("Failed to multiply embeddings: {}", e)))?)
        .sum(1)
        .map_err(|e| Error::Search(format!("Failed to sum embeddings: {}", e)))?
        .broadcast_div(&sum_mask)
        .map_err(|e| Error::Search(format!("Failed to divide embeddings: {}", e)))?;

        // L2 normalize
        let normalized = normalize_l2(&pooled_embeddings)
            .map_err(|e| Error::Search(format!("Failed to normalize embeddings: {}", e)))?;

        // Convert to vectors
        let n_sentences = texts.len();
        let mut result = Vec::with_capacity(n_sentences);

        for i in 0..n_sentences {
            let embedding = normalized
                .get(i)
                .map_err(|e| Error::Search(format!("Failed to get embedding {}: {}", i, e)))?
                .to_vec1::<f32>()
                .map_err(|e| {
                    Error::Search(format!("Failed to convert embedding to vector: {}", e))
                })?;

            result.push(embedding);
        }

        Ok(result)
    }
}

/// Normalize embeddings using L2 norm
#[cfg(feature = "local")]
fn normalize_l2(v: &candle_core::Tensor) -> candle_core::Result<candle_core::Tensor> {
    v.broadcast_div(&v.sqr()?.sum_keepdim(1)?.sqrt()?)
}

#[cfg(feature = "local")]
impl crate::embeddings::Embedder for LocalEmbedder {
    fn embed(&self, text: &str) -> Result<Vec<f32>> {
        let embeddings = self.encode(&[text.to_string()])?;
        embeddings
            .into_iter()
            .next()
            .ok_or_else(|| Error::Search("No embedding generated".into()))
    }

    fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.encode(texts)
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

/// Available local models (safetensors format)
#[allow(dead_code)]
pub const AVAILABLE_MODELS: &[(&str, usize)] = &[
    // (model_id, dimensions)
    ("sentence-transformers/all-MiniLM-L6-v2", 384),
    ("sentence-transformers/all-MiniLM-L12-v2", 384),
    ("sentence-transformers/bge-small-en", 384),
    ("sentence-transformers/bge-base-en", 768),
    ("BAAI/bge-small-en-v1.5", 384),
    ("BAAI/bge-base-en-v1.5", 768),
];

/// Parse model string (short name or full HuggingFace ID)
pub fn parse_model_string(s: &str) -> String {
    match s {
        "minilm" | "all-minilm" | "default" => DEFAULT_MODEL.to_string(),
        "minilm-l12" => "sentence-transformers/all-MiniLM-L12-v2".to_string(),
        "bge-small" => "BAAI/bge-small-en-v1.5".to_string(),
        "bge-base" => "BAAI/bge-base-en-v1.5".to_string(),
        _ => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_model_string() {
        assert_eq!(parse_model_string("minilm"), DEFAULT_MODEL);

        assert_eq!(
            parse_model_string("sentence-transformers/paraphrase-MiniLM-L6-v2"),
            "sentence-transformers/paraphrase-MiniLM-L6-v2"
        );
    }
}
