# mdsearch

**grep for semantic search** — Find concepts, not just keywords.

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

[![crates.io](https://img.shields.io/badge/crates.io-v0.1.0-blue.svg)](https://crates.io/crates/mdsearch)

```bash
cargo install mdsearch

# Index your notes
mdsearch index ~/notes

# Keyword search
mdsearch search "project timeline"

# Semantic search - finds concepts, not just words
mdsearch semantic "how do we handle authentication"
```

---

## Why mdsearch?

You have thousands of markdown notes. Keyword search fails when you don't remember exact terms. Cloud solutions send your data elsewhere.

**mdsearch gives you semantic search, locally, with zero setup.**

| | mdsearch | Alternatives |
|---|----------|---|
| **First run** | Works instantly | Download 2GB+ models |
| **Model size** | 23MB (built-in) | 2GB+ |
| **API key** | Not needed | Required for cloud |
| **Privacy** | 100% local | Sends data to cloud |

---

## Features

- **Keyword Search** - Sub-millisecond, FST-based inverted index
- **Semantic Search** - Local embeddings with Metal/CUDA acceleration (5x faster)
- **Hybrid Search** - Combines both for best results
- **RAG-Ready Chunking** - Structure-aware splitting, JSONL output
- **Incremental Indexing** - Only reindexes changed files

- **GPU Accelerated** - Metal (Apple Silicon) and CUDA (NVIDIA)

---

## Installation
```bash
# From source
git clone https://github.com/abuiliazeed/mdsearch
cd mdsearch
cargo install --path .

# With GPU acceleration (Apple Silicon)
cargo install --path . --features metal
```

## Usage
```bash
# Indexing
mdsearch index ~/notes                    # Index directory
mdsearch index ~/notes --watch            # Watch for changes

# Searching
mdsearch search "API"                     # Keyword search
mdsearch semantic "deployment process"    # Semantic search
mdsearch search "API" --hybrid            # Best of both

# RAG Pipeline
mdsearch chunk ./docs --size 512 --format jsonl > chunks.jsonl

# Utilities
mdsearch stats      # Show index statistics
mdsearch doctor     # Validate and repair
mdsearch clear      # Clear the index
```
## Performance
Benchmarks on M4 Pro (10K markdown files):
| Operation | Time |
|-----------|------|
| Keyword search | <1ms |
| Semantic search | ~50ms |
| Embed 100 chunks (CPU) | ~200ms |
| Embed 100 chunks (Metal) | ~40ms |

Index size: ~20% of source files.
## Roadmap
- [ ] Fuzzy matching
- [ ] GitHub-style search syntax (`lang:rust`, `path:src/`)
- [ ] WebAssembly build
- [ ] LSP integration
## License
MIT © [Ahmed Abuiliazeed](https://github.com/abuiliazeed)
---

**Star ⭐ this repo if you find it useful!**
