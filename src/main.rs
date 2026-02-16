//! mdsearch CLI - Blazingly fast markdown search and RAG indexing

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing_subscriber::EnvFilter;

mod chunk;
mod config;
mod error;
mod index;
mod parser;
mod search;
mod store;

#[derive(Parser, Debug)]
#[command(name = "mdsearch")]
#[command(author = "Ahmed Abuiliazeed <ahmed@abuiliazeed.com>")]
#[command(version = "0.1.0")]
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
    Stats,

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
            index::run_index(path, index_path, threads, include, exclude, gitignore, watch)?;
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
        } => {
            search::run_semantic(query, index_path, limit, format)?;
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
        Commands::Stats => {
            index::run_stats(index_path)?;
        }
        Commands::Clear { force } => {
            index::run_clear(index_path, force)?;
        }
        Commands::Doctor { repair } => {
            index::run_doctor(index_path, repair)?;
        }
    }

    Ok(())
}
