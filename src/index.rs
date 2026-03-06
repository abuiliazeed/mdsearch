//! Indexing logic for markdown files

use crate::chunk::Chunker;
use crate::config::IndexingConfig;
use crate::error::{Error, Result};
use crate::parser::{Document, Parser};
use crate::store::{Store, TermPosting};
use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info, warn};

/// Indexer for building and maintaining the search index
pub struct Indexer {
    config: IndexingConfig,
    store: Store,
    parser: Parser,
    chunker: Chunker,
}

impl Indexer {
    /// Create a new indexer
    pub fn new(store: Store, config: IndexingConfig) -> Self {
        Self {
            config,
            store,
            parser: Parser::new(),
            chunker: Chunker::new(512, 50),
        }
    }

    /// Index all markdown files in a directory
    pub fn index_directory(&self, path: &Path) -> Result<IndexStats> {
        let files = self.discover_files(path)?;
        info!("Found {} markdown files", files.len());

        if files.is_empty() {
            return Ok(IndexStats::default());
        }

        let pb = ProgressBar::new(files.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
            )
            .unwrap(),
        );

        let mut stats = IndexStats::default();
        let mut all_terms: HashMap<String, TermPosting> = HashMap::new();

        // Configure thread pool
        let threads = if self.config.threads > 0 {
            self.config.threads
        } else {
            num_cpus::get()
        };

        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()
            .map_err(|e| Error::Index(format!("Failed to create thread pool: {}", e)))?;

        let results: Vec<(Document, Vec<crate::chunk::Chunk>, HashMap<String, u32>)> = pool
            .install(|| {
                files
                    .par_iter()
                    .filter_map(|file| {
                        pb.set_message(format!("{:?}", file.file_name().unwrap()));
                        let result = self.index_file_internal(file);
                        pb.inc(1);
                        result
                    })
                    .collect()
            });

        for (doc, chunks, terms) in results {
            stats.files_indexed += 1;

            // Store document
            self.store.index_document(&doc)?;

            // Store chunks
            self.store.store_chunks_batch(&chunks)?;
            stats.chunks_created += chunks.len();

            // Aggregate terms
            for (term, freq) in terms {
                let posting = all_terms
                    .entry(term.clone())
                    .or_insert_with(|| TermPosting {
                        term,
                        doc_ids: Vec::new(),
                        total_frequency: 0,
                    });
                posting
                    .doc_ids
                    .push((doc.path.to_string_lossy().to_string(), freq));
                posting.total_frequency += freq as u64;
            }

            stats.bytes_indexed += doc.text.len() as u64;
        }

        pb.finish_with_message(format!("Indexed {} files", stats.files_indexed));

        // Store all term postings
        info!("Storing {} unique terms", all_terms.len());
        stats.unique_terms = all_terms.len();
        for (term, posting) in all_terms {
            self.store.store_term(&term, &posting)?;
        }

        Ok(stats)
    }

    /// Index a single file
    fn index_file_internal(
        &self,
        path: &Path,
    ) -> Option<(Document, Vec<crate::chunk::Chunk>, HashMap<String, u32>)> {
        // Check file size
        if let Ok(metadata) = std::fs::metadata(path) {
            if self.config.max_file_size > 0 && metadata.len() > self.config.max_file_size {
                warn!("Skipping large file: {:?}", path);
                return None;
            }
        }

        // Parse document
        let doc = match self.parser.parse_file(path) {
            Ok(d) => d,
            Err(e) => {
                warn!("Failed to parse {:?}: {}", path, e);
                return None;
            }
        };

        // Check if document changed
        if let Ok(Some(existing)) = self.store.get_document(&path.to_string_lossy()) {
            if existing.hash == doc.hash {
                debug!("Document unchanged, skipping: {:?}", path);
                return None;
            }
        }

        // Create chunks
        let chunks = match self.chunker.chunk_document(&doc) {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to chunk {:?}: {}", path, e);
                return None;
            }
        };

        // Extract terms for inverted index
        let terms = self.extract_terms(&doc);

        Some((doc, chunks, terms))
    }

    /// Discover markdown files in directory
    fn discover_files(&self, path: &Path) -> Result<Vec<PathBuf>> {
        use ignore::WalkBuilder;

        let mut files = Vec::new();

        let mut builder = WalkBuilder::new(path);
        builder
            .follow_links(self.config.follow_symlinks)
            .git_ignore(self.config.respect_gitignore)
            .git_global(self.config.respect_gitignore)
            .git_exclude(self.config.respect_gitignore);

        for result in builder.build() {
            match result {
                Ok(entry) => {
                    let path = entry.path();
                    if self.is_markdown_file(path) {
                        files.push(path.to_path_buf());
                    }
                }
                Err(err) => {
                    warn!("Error walking directory: {}", err);
                }
            }
        }

        Ok(files)
    }

    fn is_markdown_file(&self, path: &Path) -> bool {
        path.extension()
            .map(|ext| ext == "md" || ext == "markdown")
            .unwrap_or(false)
    }

    /// Extract searchable terms from document
    fn extract_terms(&self, doc: &Document) -> HashMap<String, u32> {
        let mut terms = HashMap::new();

        // Simple tokenization - split on whitespace and punctuation
        for word in doc.text.split(|c: char| !c.is_alphanumeric()) {
            if word.len() < 2 || word.len() > 50 {
                continue;
            }

            let term = word.to_lowercase();
            if !is_stop_word(&term) {
                *terms.entry(term).or_insert(0) += 1;
            }
        }

        terms
    }

    /// Watch directory for changes and reindex
    pub fn watch(&self, path: &Path) -> Result<()> {
        use notify::{RecommendedWatcher, RecursiveMode, Watcher};

        let (tx, rx) = std::sync::mpsc::channel();

        let mut watcher = RecommendedWatcher::new(tx, notify::Config::default())
            .map_err(|e| Error::Watch(e.to_string()))?;

        watcher
            .watch(path, RecursiveMode::Recursive)
            .map_err(|e| Error::Watch(e.to_string()))?;

        info!("Watching {:?} for changes...", path);

        loop {
            match rx.recv() {
                Ok(Ok(event)) => {
                    if let Some(path) = event.paths.first() {
                        if self.is_markdown_file(path) {
                            debug!("File changed: {:?}", path);
                            // Reindex the file
                            if let Some((doc, chunks, terms)) = self.index_file_internal(path) {
                                self.store.index_document(&doc)?;
                                self.store.store_chunks_batch(&chunks)?;

                                for (term, freq) in terms {
                                    let posting = TermPosting {
                                        term: term.clone(),
                                        doc_ids: vec![(
                                            doc.path.to_string_lossy().to_string(),
                                            freq,
                                        )],
                                        total_frequency: freq as u64,
                                    };
                                    self.store.store_term(&term, &posting)?;
                                }

                                info!("Reindexed: {:?}", path);
                            }
                        }
                    }
                }
                Ok(Err(e)) => warn!("Watch error: {}", e),
                Err(e) => {
                    warn!("Channel error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Check if word is a common stop word
fn is_stop_word(word: &str) -> bool {
    const STOP_WORDS: &[&str] = &[
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "been", "be", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "must", "can", "this", "that",
        "these", "those", "it", "its", "i", "you", "we", "they", "he", "she", "what", "which",
        "who", "when", "where", "why", "how", "all", "each", "every", "both", "few", "more",
        "most", "other", "some", "such", "no", "not", "only", "own", "same", "so", "than", "too",
        "very", "just", "also", "now", "here", "there", "then", "if",
    ];

    STOP_WORDS.contains(&word)
}

/// Statistics from indexing operation
#[derive(Debug, Default, Clone)]
pub struct IndexStats {
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub bytes_indexed: u64,
    pub unique_terms: usize,
}

/// Run the index command
pub fn run_index(
    path: PathBuf,
    index_path: PathBuf,
    threads: Option<usize>,
    include: String,
    exclude: Option<String>,
    gitignore: bool,
    watch: bool,
) -> Result<()> {
    let mut config = IndexingConfig::default();
    if let Some(t) = threads {
        config.threads = t;
    }
    config.include_patterns = include.split(',').map(|s| s.trim().to_string()).collect();
    if let Some(ex) = exclude {
        config.exclude_patterns = ex.split(',').map(|s| s.trim().to_string()).collect();
    }
    config.respect_gitignore = gitignore;

    let store = Store::open(&index_path)?;
    let indexer = Indexer::new(store, config);

    let stats = indexer.index_directory(&path)?;

    println!("\n📊 Index Statistics:");
    println!("  Files indexed:   {}", stats.files_indexed);
    println!("  Chunks created:  {}", stats.chunks_created);
    println!("  Bytes indexed:   {}", format_bytes(stats.bytes_indexed));
    println!("  Unique terms:    {}", stats.unique_terms);

    if watch {
        println!("\n👀 Watching for changes (Ctrl+C to stop)...");
        indexer.watch(&path)?;
    }

    Ok(())
}

/// Run stats command
pub fn run_stats(index_path: PathBuf, detailed: bool) -> Result<()> {
    if !Store::exists(&index_path) {
        return Err(Error::IndexNotFound(index_path));
    }

    let store = Store::open(&index_path)?;
    let meta = store.metadata()?;

    println!("📊 Index Statistics:");
    println!("  Version:         {}", meta.version);
    println!("  Created:         {}", format_timestamp(meta.created_at));
    println!("  Last updated:    {}", format_timestamp(meta.updated_at));
    println!("  Documents:       {}", meta.document_count);
    println!("  Chunks:          {}", meta.chunk_count);
    println!("  Total bytes:     {}", format_bytes(meta.total_bytes));
    println!("  Index path:      {}", store.path().display());

    if detailed {
        println!("\n📝 Chunks:");
        let chunks = store.get_all_chunks()?;
        for chunk in chunks.iter().take(10) {
            println!("  - {} ({} chars)", chunk.id, chunk.content.len());
            println!(
                "    Preview: {}...",
                chunk.content.chars().take(50).collect::<String>()
            );
        }
        if chunks.len() > 10 {
            println!("  ... and {} more", chunks.len() - 10);
        }
    }

    Ok(())
}

/// Run clear command
pub fn run_clear(index_path: PathBuf, force: bool) -> Result<()> {
    if !Store::exists(&index_path) {
        println!("Index does not exist at {}", index_path.display());
        return Ok(());
    }

    if !force {
        println!("This will delete the index at {}", index_path.display());
        println!("Use --force to confirm.");
        return Ok(());
    }

    let store = Store::open(&index_path)?;
    store.clear()?;

    println!("✅ Index cleared at {}", index_path.display());

    Ok(())
}

/// Run doctor command
pub fn run_doctor(index_path: PathBuf, repair: bool) -> Result<()> {
    if !Store::exists(&index_path) {
        return Err(Error::IndexNotFound(index_path));
    }

    let store = Store::open(&index_path)?;
    let meta = store.metadata()?;

    println!("🔍 Checking index integrity...");

    // Check metadata
    println!("  ✓ Metadata valid (v{})", meta.version);

    // Check documents
    let docs = store.list_documents()?;
    println!("  ✓ {} documents indexed", docs.len());

    // Check for orphaned chunks
    let mut orphaned = 0;
    for doc in &docs {
        let chunks = store.get_chunks_for_document(&doc.path)?;
        if chunks.is_empty() {
            orphaned += 1;
            if repair {
                println!("  ⚠ Removing empty document: {}", doc.path);
                store.delete_document(&doc.path)?;
            }
        }
    }

    if orphaned > 0 {
        if repair {
            println!("  ✓ Removed {} orphaned documents", orphaned);
        } else {
            println!(
                "  ⚠ Found {} orphaned documents (run with --repair to remove)",
                orphaned
            );
        }
    }

    println!("\n✅ Index health check complete");

    Ok(())
}

fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

fn format_timestamp(ts: u64) -> String {
    use chrono::{TimeZone, Utc};

    match Utc.timestamp_opt(ts as i64, 0) {
        chrono::LocalResult::Single(dt) => dt.format("%Y-%m-%d %H:%M:%S UTC").to_string(),
        _ => format!("Invalid timestamp ({})", ts),
    }
}
