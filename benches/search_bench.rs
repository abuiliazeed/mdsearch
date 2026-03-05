//! Benchmarks for mdsearch

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};

fn bench_search(c: &mut Criterion) {
    // Simulate searching through chunks
    let chunks: Vec<String> = (0..1000)
        .map(|i| format!("This is chunk number {} with some content about Rust programming and async patterns.", i))
        .collect();

    c.bench_with_input(
        BenchmarkId::new("search", "1000_chunks"),
        &chunks,
        |b, chunks| {
            b.iter(|| {
                let query = "Rust";
                let results: Vec<_> = chunks
                    .iter()
                    .filter(|c| c.contains(query))
                    .collect();
                black_box(results)
            })
        },
    );
}

fn bench_indexing(c: &mut Criterion) {
    // Simulate parsing markdown content
    let markdown = r#"# Header 1

Some content here with **bold** and *italic* text.

## Header 2

- List item 1
- List item 2
- List item 3

```rust
fn main() {
    println!("Hello, world!");
}
```

More content after code block.
"#;

    c.bench_function("parse_markdown", |b| {
        b.iter(|| {
            // Simple markdown parsing simulation
            let lines: Vec<_> = markdown.lines().collect();
            let headers: Vec<_> = lines.iter().filter(|l| l.starts_with('#')).collect();
            let code_blocks = markdown.matches("```").count();
            black_box((lines.len(), headers.len(), code_blocks))
        })
    });
}

fn bench_chunking(c: &mut Criterion) {
    // Simulate chunking text into pieces
    let text = "First sentence. Second sentence. Third sentence. Fourth sentence. Fifth sentence. Sixth sentence. Seventh sentence. Eighth sentence. Ninth sentence. Tenth sentence.";
    
    let mut group = c.benchmark_group("chunking");
    
    for size in [64, 128, 256, 512].iter() {
        group.bench_with_input(BenchmarkId::new("size", size), size, |b, &size| {
            b.iter(|| {
                let mut chunks = Vec::new();
                let mut current = String::new();
                
                for word in text.split_whitespace() {
                    if current.len() + word.len() + 1 > size && !current.is_empty() {
                        chunks.push(current.clone());
                        current.clear();
                    }
                    if !current.is_empty() {
                        current.push(' ');
                    }
                    current.push_str(word);
                }
                
                if !current.is_empty() {
                    chunks.push(current);
                }
                
                black_box(chunks)
            })
        });
    }
    
    group.finish();
}

fn bench_similarity(c: &mut Criterion) {
    // Simulate cosine similarity calculation
    let a: Vec<f32> = (0..384).map(|i| (i as f32 * 0.01).sin()).collect();
    let b: Vec<f32> = (0..384).map(|i| (i as f32 * 0.02).cos()).collect();
    
    // Normalize vectors
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    let a: Vec<f32> = a.iter().map(|x| x / norm_a).collect();
    let b: Vec<f32> = b.iter().map(|x| x / norm_b).collect();
    
    c.bench_function("cosine_similarity_384d", |b| {
        b.iter(|| {
            let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
            black_box(dot)
        })
    });
}

criterion_group!(benches, bench_search, bench_indexing, bench_chunking, bench_similarity);
criterion_main!(benches);
