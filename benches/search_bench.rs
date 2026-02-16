//! Benchmarks for mdsearch

use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn bench_search(c: &mut Criterion) {
    // TODO: Add real benchmarks
    c.bench_function("search_100_docs", |b| {
        b.iter(|| {
            black_box(1 + 1);
        })
    });
}

fn bench_indexing(c: &mut Criterion) {
    // TODO: Add real benchmarks
    c.bench_function("index_1000_docs", |b| {
        b.iter(|| {
            black_box(1 + 1);
        })
    });
}

fn bench_chunking(c: &mut Criterion) {
    // TODO: Add real benchmarks
    c.bench_function("chunk_100_docs", |b| {
        b.iter(|| {
            black_box(1 + 1);
        })
    });
}

criterion_group!(benches, bench_search, bench_indexing, bench_chunking);
criterion_main!(benches);
