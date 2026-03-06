use std::collections::HashMap;
use tempfile::tempdir;

#[test]
fn test_store_chunk_roundtrip() {
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");

    let store = mdsearch::Store::open(&path).unwrap();

    let chunk = mdsearch::chunk::Chunk {
        id: "test-chunk-1".to_string(),
        file: "test.md".to_string(),
        content: "This is Rust programming content".to_string(),
        char_range: (0, 30),
        line_range: (1, 2),
        section_path: "Test".to_string(),
        context_before: None,
        context_after: None,
        token_count: 10,
        metadata: HashMap::new(),
        embedding: None,
    };

    // Store
    store.store_chunks_batch(&[chunk.clone()]).unwrap();

    // Retrieve
    let chunks = store.get_all_chunks().unwrap();
    println!("Retrieved {} chunks", chunks.len());
    for c in &chunks {
        println!("  - {} : {}", c.id, c.content);
    }

    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "This is Rust programming content");
}
