# Contributing to mdsearch

Thank you for your interest in contributing!

## Development Setup

```bash
# Clone the repo
git clone https://github.com/abuiliazeed/mdsearch.git
cd mdsearch

# Build
cargo build

# Run tests
cargo test

# Run with logging
RUST_LOG=debug cargo run -- search "query"
```

## Code Style

This project uses standard Rust conventions:

```bash
# Format code
cargo fmt

# Run linter
cargo clippy -- -D warnings
```

## Project Structure

```
src/
├── main.rs       # CLI entry point
├── lib.rs        # Library exports
├── config.rs     # Configuration types
├── parser.rs     # Markdown parsing
├── chunk.rs      # RAG chunking
├── index.rs      # Indexing logic
├── search.rs     # Search functionality
├── store.rs      # Storage (RocksDB)
├── embeddings.rs # Embedding providers
└── error.rs      # Error types
```

## Adding New Commands

1. Add command to `Commands` enum in `main.rs`
2. Create module or add to existing module
3. Add tests
4. Update README

## Running Benchmarks

```bash
cargo bench
```

## Pull Request Process

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Make your changes
4. Run tests (`cargo test`)
5. Run clippy (`cargo clippy`)
6. Commit with clear message
7. Push and open PR

## Release Process

1. Update version in `Cargo.toml`
2. Update `CHANGELOG.md`
3. Create git tag (`git tag v0.x.0`)
4. Push tag (`git push --tags`)
5. CI will create GitHub release with binaries

## Questions?

Open an issue or discussion on GitHub.
