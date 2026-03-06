use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Chunk {
    id: String,
    file: String,
    content: String,
    char_range: (usize, usize),
    line_range: (usize, usize),
    section_path: String,
    context_before: Option<String>,
    context_after: Option<String>,
    token_count: usize,
    metadata: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    embedding: Option<Vec<f32>>,
}

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
        metadata: HashMap::new(),
        embedding: None,
    };
    
    println!("Original chunk: {:?}", chunk.id);
    
    let encoded = bincode::serialize(&chunk).unwrap();
    println!("Serialized: {} bytes", encoded.len());
    
    match bincode::deserialize::<Chunk>(&encoded) {
        Ok(decoded) => {
            println!("✅ Success");
            println!("  ID: {}", decoded.id);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
}
