# mdsearch

Blazingly fast markdown search and RAG indexing tool written in Rust.

## Features

- ⚡ **Sub-millisecond search** over massive markdown collections
- 📁 **Smart indexing** with incremental updates
- 🧩 **RAG-ready chunking** with structure awareness
- 🔍 **Hybrid search** (keyword + semantic)
- 📊 **Markdown-aware** parsing (headers, code blocks, frontmatter)

## Installation

```bash
# From source
git clone https://github.com/abuiliazeed/mdsearch
cd mdsearch
cargo install --path .
```

## Quick Start

```bash
# Index a directory of markdown files
mdsearch index ./notes

# Search the index
mdsearch search "Rust async patterns"

# Get RAG-ready chunks
mdsearch chunk ./notes --size 512 --format jsonl

# View index statistics
mdsearch stats
```

## Commands

### `mdsearch index <path>`

Index markdown files in a directory.

```bash
mdsearch index ./docs --watch          # Watch for changes
mdsearch index ./notes --threads 8     # Use 8 threads
mdsearch index ./wiki --exclude "draft/**"  # Exclude patterns
```

Options:
- `--watch` - Watch for file changes and reindex automatically
- `--threads <n>` - Number of parallel threads (default: auto)
- `--include <patterns>` - File patterns to include (default: `**/*.md`)
- `--exclude <patterns>` - File patterns to exclude
- `--gitignore` - Respect .gitignore files (default: true)

### `mdsearch search <query>`

Search indexed content.

```bash
mdsearch search "memory management" --limit 20
mdsearch search "API" --headers-only   # Search only in headers
mdsearch search "config" --format json # JSON output
mdsearch search "error" --filter "src/" # Filter by path
```

Options:
- `-n, --limit <n>` - Maximum results (default: 10)
- `--format <format>` - Output format: plain, json, jsonl
- `--headers-only` - Search only in markdown headers
- `--exclude-code` - Exclude code blocks from search
- `--filter <pattern>` - Filter by file path pattern

### `mdsearch semantic <query>`

Semantic search using embeddings (requires embedding model).

```bash
mdsearch semantic "how to handle errors" --limit 10
```

### `mdsearch chunk <path>`

Chunk markdown files for RAG applications.

```bash
mdsearch chunk ./docs --size 512 --overlap 50
mdsearch chunk ./notes --format jsonl --metadata
```

Options:
- `--size <chars>` - Target chunk size in characters (default: 512)
- `--overlap <chars>` - Overlap between chunks (default: 50)
- `--format <format>` - Output format: plain, json, jsonl
- `--metadata` - Include file metadata in output

### `mdsearch stats`

Show index statistics.

### `mdsearch clear`

Clear the index database.

### `mdsearch doctor`

Validate and repair index integrity.

## Use Cases

### Personal Knowledge Base

```bash
# Index your Obsidian/Notion export
mdsearch index ~/notes --watch

# Quick search
mdsearch search "project timeline"
```

### RAG Pipeline

```bash
# Chunk for LLM context
mdsearch chunk ./docs --size 512 --format jsonl > chunks.jsonl

# Feed to your LLM pipeline
cat chunks.jsonl | your-embeddings-pipeline
```

### Documentation Search

```bash
# Index documentation
mdsearch index ./docs

# Search with filters
mdsearch search "installation" --filter "getting-started/"
```

## Performance

| Operation | Time |
|-----------|------|
| Index 10K files | ~5 seconds |
| Search (keyword) | <1ms |
| Chunk 1K files | ~500ms |
| Index size | ~20% of source |

*Benchmarks on M1 MacBook Pro with SSD*

## Architecture

```
mdsearch/
├── src/
│   ├── main.rs       # CLI entry point
│   ├── lib.rs        # Library exports
│   ├── config.rs     # Configuration types
│   ├── parser.rs     # Markdown parsing
│   ├── chunk.rs      # RAG chunking
│   ├── index.rs      # Indexing logic
│   ├── search.rs     # Search functionality
│   ├── store.rs      # Storage (RocksDB)
│   └── error.rs      # Error types
├── benches/
│   └── search_bench.rs
└── Cargo.toml
```

## Storage

mdsearch uses RocksDB for persistent storage:
- Documents metadata
- Chunk content
- Inverted index (term → documents)
- Optional: embedding vectors

Index location: `.mdsearch/` in the current directory, or specified via `--index-path`.

## Comparison

| Feature | mdsearch | ripgrep | grep |
|---------|----------|---------|------|
| Markdown-aware | ✅ | ❌ | ❌ |
| Indexed search | ✅ | ❌ | ❌ |
| RAG chunking | ✅ | ❌ | ❌ |
| Semantic search | ✅ | ❌ | ❌ |
| Sub-millisecond | ✅ | ✅ | ❌ |

## Integration

### OpenClaw Memory Search

mdsearch can be used as a backend for OpenClaw's memory search:

```json5
memory: {
  backend: "mdsearch",
  mdsearch: {
    indexPath: "~/.openclaw/memory/{agentId}/mdsearch",
  }
}
```

### Node.js

```bash
# Build as native module (coming soon)
cargo build --release --features node
```

## Roadmap

- [ ] Semantic search with local embeddings
- [ ] Fuzzy matching
- [ ] GitHub-style code search syntax
- [ ] WebAssembly build
- [ ] Node.js native module
- [ ] Language server protocol (LSP)

## License

MIT

## Contributing

Contributions welcome! Please read CONTRIBUTING.md first.
