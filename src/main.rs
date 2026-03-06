//! mdsearch CLI - Blazingly fast markdown search and RAG indexing

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod chunk;
mod config;
mod embeddings;
mod error;
mod index;
mod local_embeddings;
mod parser;
mod search;
mod store;

#[derive(Parser, Debug)]
#[command(name = "mdsearch")]
#[command(author = "Ahmed Abuiliazeed <ahmed@abuiliazeed.com>")]
#[command(version = "0.2.0-dev")]
#[command(about = "Blazingly fast markdown search and RAG indexing", long_about = None)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to index database (default: .mdsearch in current directory)
    #[arg(short, long, global = true, env = "MDSEARCH_INDEX_PATH")]
    index_path: Option<PathBuf>,

    /// Verbosity level (-v, -vv, -vvv)
    #[arg(short, long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Index markdown files in a directory
    Index {
        /// Path to directory containing markdown files
        path: PathBuf,

        /// Watch for file changes and reindex automatically
        #[arg(short, long)]
        watch: bool,

        /// Number of parallel threads (default: auto)
        #[arg(short, long)]
        threads: Option<usize>,

        /// File patterns to include (comma-separated glob patterns)
        #[arg(long, default_value = "**/*.md")]
        include: String,

        /// File patterns to exclude (comma-separated glob patterns)
        #[arg(long)]
        exclude: Option<String>,

        /// Respect .gitignore files
        #[arg(long, default_value = "true")]
        gitignore: bool,
    },

    /// Search the indexed markdown files
    Search {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Output format: json, jsonl, plain
        #[arg(short, long, default_value = "plain")]
        format: String,

        /// Search only in headers
        #[arg(long)]
        headers_only: bool,

        /// Exclude code blocks from search
        #[arg(long)]
        exclude_code: bool,

        /// Filter by file path pattern
        #[arg(short, long)]
        filter: Option<String>,
    },

    /// Perform semantic search using embeddings (requires embeddings index)
    Semantic {
        /// Search query
        query: String,

        /// Maximum number of results
        #[arg(short = 'n', long, default_value = "10")]
        limit: usize,

        /// Output format: json, jsonl, plain
        #[arg(short, long, default_value = "plain")]
        format: String,

        /// Embedding provider: local, openai, mock
        #[arg(long, default_value = "local")]
        provider: String,
    },

    /// Generate embeddings for indexed chunks
    Embed {
        /// Embedding provider: local, openai, mock
        #[arg(long, default_value = "local")]
        provider: String,

        /// Model name or "repo_id:filename" (e.g., "minilm" or "user/repo:model.gguf")
        #[arg(long, default_value = "minilm")]
        model: Option<String>,

        /// Path to bundled/offline model (skips download)
        #[arg(long)]
        model_path: Option<PathBuf>,

        /// Cache directory for downloaded models (default: ~/.cache/mdsearch)
        #[arg(long)]
        cache_dir: Option<PathBuf>,

        /// Batch size for embedding API calls
        #[arg(long, default_value = "100")]
        batch_size: usize,
    },

    /// Chunk markdown files for RAG applications
    Chunk {
        /// Path to markdown file or directory
        path: PathBuf,

        /// Target chunk size in characters
        #[arg(long, default_value = "512")]
        size: usize,

        /// Overlap between chunks in characters
        #[arg(long, default_value = "50")]
        overlap: usize,

        /// Output format: json, jsonl, plain
        #[arg(short, long, default_value = "jsonl")]
        format: String,

        /// Include file metadata in output
        #[arg(long)]
        metadata: bool,
    },

    /// Show index statistics
    Stats {
        /// Show detailed chunk information
        #[arg(long, default_value = "false")]
        detailed: bool,
    },

    /// Clear the index database
    Clear {
        /// Force clear without confirmation
        #[arg(short, long)]
        force: bool,
    },

    /// Validate and repair index integrity
    Doctor {
        /// Attempt to repair issues
        #[arg(long)]
        repair: bool,
    },

    /// Manage embedding models
    Models {
        #[command(subcommand)]
        command: ModelCommands,
    },
}

#[derive(Subcommand, Debug)]
enum ModelCommands {
    /// List available embedding models
    List,

    /// Download a model for offline use
    Download {
        /// Model name (e.g., "minilm") or "repo_id:filename"
        #[arg(default_value = "minilm")]
        model: String,

        /// Cache directory (default: ~/.cache/mdsearch)
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },

    /// Show model cache status
    Status {
        /// Cache directory (default: ~/.cache/mdsearch)
        #[arg(long)]
        cache_dir: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging based on verbosity
    let log_level = match cli.verbose {
        0 => "warn",
        1 => "info",
        2 => "debug",
        _ => "trace",
    };

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_level)),
        )
        .init();

    // Determine index path
    let index_path = cli.index_path.unwrap_or_else(|| {
        std::env::current_dir()
            .expect("Could not determine current directory")
            .join(".mdsearch")
    });

    match cli.command {
        Commands::Index {
            path,
            watch,
            threads,
            include,
            exclude,
            gitignore,
        } => {
            index::run_index(
                path, index_path, threads, include, exclude, gitignore, watch,
            )?;
        }
        Commands::Search {
            query,
            limit,
            format,
            headers_only,
            exclude_code,
            filter,
        } => {
            search::run_search(
                query,
                index_path,
                limit,
                format,
                headers_only,
                exclude_code,
                filter,
            )?;
        }
        Commands::Semantic {
            query,
            limit,
            format,
            provider,
        } => {
            search::run_semantic(query, index_path, limit, format, provider)?;
        }
        Commands::Embed {
            provider,
            model,
            model_path,
            cache_dir,
            batch_size,
        } => {
            embeddings::run_embed(index_path, provider, model, model_path, cache_dir, batch_size)?;
        }
        Commands::Chunk {
            path,
            size,
            overlap,
            format,
            metadata,
        } => {
            chunk::run_chunk(path, size, overlap, format, metadata)?;
        }
        Commands::Stats { detailed } => {
            index::run_stats(index_path, detailed)?;
        }
        Commands::Clear { force } => {
            index::run_clear(index_path, force)?;
        }
        Commands::Doctor { repair } => {
            index::run_doctor(index_path, repair)?;
        }
        Commands::Models { command } => {
            run_model_command(command)?;
        }
    }

    Ok(())
}

fn run_model_command(command: ModelCommands) -> Result<()> {
    use mdsearch::local_embeddings::{self, LocalModelConfig, parse_model_string, default_cache_dir};

    match command {
        ModelCommands::List => {
            println!("Available embedding models:\n");
            println!("{:<40} {:<30} {:>10}", "MODEL", "FILE", "DIMS");
            println!("{}", "-".repeat(82));

            for (repo, file, dims) in local_embeddings::AVAILABLE_MODELS {
                println!("{:<40} {:<30} {:>10}", repo, file, dims);
            }

            println!("\nUsage:");
            println!("  mdsearch embed --model minilm              # Use default model");
            println!("  mdsearch embed --model user/repo:model.gguf # Use specific model");
            println!("  mdsearch models download minilm            # Pre-download for offline use");
            Ok(())
        }
        ModelCommands::Download { model, cache_dir } => {
            let (repo_id, filename) = parse_model_string(&model);

            let config = if let Some(cache) = cache_dir {
                LocalModelConfig::new(repo_id, filename).with_cache_dir(cache)
            } else {
                LocalModelConfig::new(repo_id, filename)
            };

            println!("Downloading model: {}:{}", config.repo_id, config.filename);
            println!("Cache directory: {:?}", config.cache_dir);

            #[cfg(feature = "local")]
            {
                let path = local_embeddings::download_model(&config)?;
                println!("\n✅ Model downloaded to: {:?}", path);
            }

            #[cfg(not(feature = "local"))]
            {
                println!("❌ Local embeddings not compiled in. Rebuild with --features local");
            }
            Ok(())
        }
        ModelCommands::Status { cache_dir } => {
            let cache = cache_dir.unwrap_or_else(default_cache_dir);

            println!("Model cache directory: {:?}\n", cache);

            let models_dir = cache.join("models");
            if !models_dir.exists() {
                println!("No models downloaded yet.");
                println!("\nRun 'mdsearch models download minilm' to download the default model.");
                return Ok(());
            }

            println!("Downloaded models:\n");

            let mut found = false;
            if let Ok(entries) = std::fs::read_dir(&models_dir) {
                for entry in entries.flatten() {
                    if entry.path().is_dir() {
                        let repo_name = entry.file_name().to_string_lossy().replace("--", "/");
                        println!("  📁 {}", repo_name);

                        if let Ok(files) = std::fs::read_dir(entry.path()) {
                            for file in files.flatten() {
                                if file.path().extension().map(|e| e == "gguf").unwrap_or(false) {
                                    let size = file.metadata().map(|m| m.len()).unwrap_or(0);
                                    let size_mb = size as f64 / (1024.0 * 1024.0);
                                    println!(
                                        "     └── {} ({:.1} MB)",
                                        file.file_name().to_string_lossy(),
                                        size_mb
                                    );
                                    found = true;
                                }
                            }
                        }
                    }
                }
            }

            if !found {
                println!("No GGUF models found in cache.");
            }
            Ok(())
        }
    }
}
