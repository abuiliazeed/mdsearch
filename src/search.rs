//! Search functionality

use crate::chunk::Chunk;
use crate::embeddings::{cosine_similarity, create_embedder, EmbeddingConfig, EmbeddingProvider};
use crate::error::{Error, Result};
use crate::store::Store;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

/// Search result with relevance scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    /// Matching chunk
    pub chunk: Chunk,

    /// Relevance score (0.0 - 1.0)
    pub score: f32,

    /// Highlights of matched text
    pub highlights: Vec<Highlight>,

    /// Match type
    pub match_type: MatchType,
}

/// Highlight span
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Highlight {
    pub start: usize,
    pub end: usize,
    pub text: String,
}

/// Type of match
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MatchType {
    Exact,
    Fuzzy,
    Semantic,
    Prefix,
}

/// Searcher for querying the index
pub struct Searcher {
    store: Store,
    config: SearchConfig,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct SearchConfig {
    pub limit: usize,
    pub min_score: f32,
    pub headers_only: bool,
    pub exclude_code: bool,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            limit: 10,
            min_score: 0.1,
            headers_only: false,
            exclude_code: false,
        }
    }
}

impl Searcher {
    /// Create a new searcher
    pub fn new(store: Store) -> Self {
        Self {
            store,
            config: SearchConfig::default(),
        }
    }

    /// Configure search
    pub fn with_config(mut self, config: SearchConfig) -> Self {
        self.config = config;
        self
    }

    /// Perform keyword search
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>> {
        let terms = self.tokenize(query);
        if terms.is_empty() {
            return Err(Error::InvalidQuery("Empty query".into()));
        }

        // Get postings for each term
        let mut doc_scores: HashMap<String, f32> = HashMap::new();
        let mut matched_chunks: HashSet<String> = HashSet::new();

        for term in &terms {
            if let Some(posting) = self.store.get_term(term)? {
                let idf = self.calculate_idf(posting.doc_ids.len());

                for (doc_id, tf) in posting.doc_ids {
                    let score = tf as f32 * idf;
                    *doc_scores.entry(doc_id).or_default() += score;
                }
            }
        }

        // Search chunks directly for exact matches
        let chunks = self.store.search_chunks(query, self.config.limit * 3)?;

        for chunk in &chunks {
            matched_chunks.insert(chunk.id.clone());
        }

        // Build results
        let mut results: Vec<SearchResult> = chunks
            .into_iter()
            .filter_map(|chunk| {
                let score = self.score_chunk(&chunk, &terms);

                if score < self.config.min_score {
                    return None;
                }

                let highlights = self.find_highlights(&chunk.content, &terms);

                Some(SearchResult {
                    chunk,
                    score,
                    highlights,
                    match_type: MatchType::Exact,
                })
            })
            .collect();

        // Sort by score
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Limit results
        results.truncate(self.config.limit);

        Ok(results)
    }

    /// Perform semantic search using embeddings
    pub fn semantic_search(&self, query: &str, provider: &str) -> Result<Vec<SearchResult>> {
        // Create embedder based on provider
        let embedding_config = EmbeddingConfig {
            provider: match provider {
                "openai" => EmbeddingProvider::OpenAI,
                _ => EmbeddingProvider::Mock,
            },
            model: "default".to_string(),
            dimensions: 384,
        };

        let embedder = create_embedder(&embedding_config)?;

        // Generate query embedding
        let query_embedding = embedder.embed(query)?;

        // Get all chunks with embeddings
        let chunks = self.store.get_all_chunks()?;

        // Filter chunks that have embeddings
        let chunks_with_embeddings: Vec<_> = chunks
            .into_iter()
            .filter(|c| c.embedding.is_some())
            .collect();

        if chunks_with_embeddings.is_empty() {
            return Err(Error::Search(
                "No embeddings found. Run 'mdsearch embed' first to generate embeddings.".into(),
            ));
        }

        // Compute similarities
        let mut scored: Vec<_> = chunks_with_embeddings
            .into_iter()
            .filter_map(|chunk| {
                let embedding = chunk.embedding.as_ref()?;
                let score = cosine_similarity(&query_embedding, embedding);

                if score < self.config.min_score {
                    return None;
                }

                Some(SearchResult {
                    chunk,
                    score,
                    highlights: Vec::new(),
                    match_type: MatchType::Semantic,
                })
            })
            .collect();

        // Sort by score (highest first)
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        // Limit results
        scored.truncate(self.config.limit);

        Ok(scored)
    }

    fn tokenize(&self, text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| s.len() >= 2)
            .map(|s| s.to_string())
            .collect()
    }

    fn calculate_idf(&self, doc_freq: usize) -> f32 {
        // Simple IDF calculation
        if doc_freq == 0 {
            return 0.0;
        }

        let meta = self.store.metadata().unwrap_or_default();
        let total_docs = meta.document_count.max(1) as f32;
        let df = doc_freq.max(1) as f32;

        (total_docs / df).ln() + 1.0
    }

    fn score_chunk(&self, chunk: &Chunk, terms: &[String]) -> f32 {
        let content_lower = chunk.content.to_lowercase();
        let mut score = 0.0f32;

        for term in terms {
            let term_lower = term.to_lowercase();

            // Count occurrences
            let count = content_lower.matches(&term_lower).count();
            score += count as f32;

            // Bonus for title/section path match
            if chunk
                .section_path
                .to_lowercase()
                .contains(&term_lower)
            {
                score += 2.0;
            }
        }

        // Normalize by content length
        let length_factor = 1.0 / (1.0 + chunk.content.len() as f32 / 1000.0);
        score * length_factor / terms.len() as f32
    }

    fn find_highlights(&self, content: &str, terms: &[String]) -> Vec<Highlight> {
        let mut highlights = Vec::new();
        let content_lower = content.to_lowercase();

        for term in terms {
            let term_lower = term.to_lowercase();
            let mut start = 0;

            while let Some(pos) = content_lower[start..].find(&term_lower) {
                let abs_start = start + pos;
                let abs_end = abs_start + term.len();

                highlights.push(Highlight {
                    start: abs_start,
                    end: abs_end,
                    text: content[abs_start..abs_end].to_string(),
                });

                start = abs_end;
            }
        }

        // Sort and dedupe
        highlights.sort_by_key(|h| h.start);
        highlights.dedup_by_key(|h| h.start);

        highlights
    }
}

/// Run the search command
pub fn run_search(
    query: String,
    index_path: PathBuf,
    limit: usize,
    format: String,
    headers_only: bool,
    exclude_code: bool,
    filter: Option<String>,
) -> Result<()> {
    if !Store::exists(&index_path) {
        return Err(Error::IndexNotFound(index_path));
    }

    let store = Store::open(&index_path)?;
    let config = SearchConfig {
        limit,
        min_score: 0.1,
        headers_only,
        exclude_code,
    };

    let searcher = Searcher::new(store).with_config(config);
    let results = searcher.search(&query)?;

    if results.is_empty() {
        println!("No results found for: {}", query);
        return Ok(());
    }

    match format.as_str() {
        "json" => output_json(&results),
        "jsonl" => output_jsonl(&results),
        _ => output_plain(&results, &query, filter.as_deref()),
    }

    Ok(())
}

/// Run semantic search command
pub fn run_semantic(
    query: String,
    index_path: PathBuf,
    limit: usize,
    format: String,
    provider: String,
) -> Result<()> {
    if !Store::exists(&index_path) {
        return Err(Error::IndexNotFound(index_path));
    }

    let store = Store::open(&index_path)?;
    let config = SearchConfig {
        limit,
        min_score: 0.1,
        headers_only: false,
        exclude_code: false,
    };

    let searcher = Searcher::new(store).with_config(config);

    match searcher.semantic_search(&query, &provider) {
        Ok(results) => {
            if results.is_empty() {
                println!("No semantic results found for: {}", query);
                return Ok(());
            }

            match format.as_str() {
                "json" => output_json(&results),
                "jsonl" => output_jsonl(&results),
                _ => output_plain(&results, &query, None),
            }
        }
        Err(e) => {
            println!("{}", e);
            println!("\nTip: Run 'mdsearch embed' first to generate embeddings.");
        }
    }

    Ok(())
}

fn output_json(results: &[SearchResult]) {
    println!("{}", serde_json::to_string_pretty(results).unwrap());
}

fn output_jsonl(results: &[SearchResult]) {
    for result in results {
        println!("{}", serde_json::to_string(result).unwrap());
    }
}

fn output_plain(results: &[SearchResult], query: &str, filter: Option<&str>) {
    use colored::Colorize;

    println!("\n{} results for \"{}\"\n", results.len(), query.cyan());

    for (i, result) in results.iter().enumerate() {
        // Apply filter
        if let Some(f) = filter {
            if !result.chunk.file.contains(f) {
                continue;
            }
        }

        println!(
            "{} {}",
            format!("{}. ", i + 1).dimmed(),
            result.chunk.file.blue()
        );

        if !result.chunk.section_path.is_empty() {
            println!("   Section: {}", result.chunk.section_path.dimmed());
        }

        println!("   Score: {:.2}", result.score);

        // Print content with highlights
        let content = highlight_matches(&result.chunk.content, &result.highlights);
        println!();
        for line in content.lines().take(5) {
            println!("   {}", line);
        }

        if result.chunk.content.lines().count() > 5 {
            println!("   {}", "...".dimmed());
        }

        println!();
    }
}

fn highlight_matches(content: &str, highlights: &[Highlight]) -> String {
    if highlights.is_empty() {
        return content.to_string();
    }

    // Simple highlighting - add ** around matches
    let mut result = String::new();
    let mut last_end = 0;

    for highlight in highlights {
        if highlight.start > last_end {
            result.push_str(&content[last_end..highlight.start]);
        }
        result.push_str("**");
        result.push_str(&highlight.text);
        result.push_str("**");
        last_end = highlight.end;
    }

    if last_end < content.len() {
        result.push_str(&content[last_end..]);
    }

    result
}
