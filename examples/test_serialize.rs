use mdsearch::chunk::Chunk;
use serde::{Serialize, Deserialize};

fn main() {
    let chunk = Chunk {
        id: "test-1".to_string(),
        file: "test.md".to_string(),
        content: "Hello world".to_string(),
        char_range: (0, 11),
        line_range: (1, 1),
        section_path: "Test".to_string(),
        context_before: None,
        context_after: None,
        token_count: 2,
        metadata: std::collections::HashMap::new(),
        embedding: None,
    };
    
    println!("Original chunk: {:?}", chunk.id);
    
    // Serialize
    let encoded = bincode::serialize(&chunk).unwrap();
    println!("Serialized: {} bytes", encoded.len());
    
    // Deserialize
    match bincode::deserialize::<Chunk>(&encoded) {
        Ok(decoded) => {
            println!("✅ Deserialization successful");
            println!("  ID: {}", decoded.id);
            println!("  Content: {}", decoded.content);
        }
        Err(e) => {
            println!("❌ Deserialization failed: {}", e);
        }
    }
}
