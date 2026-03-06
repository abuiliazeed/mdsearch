use serde::{Serialize, Deserialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestWithHashMap {
    id: String,
    metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestWithEmbedding {
    id: String,
    embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestBoth {
    id: String,
    metadata: HashMap<String, String>,
    embedding: Option<Vec<f32>>,
}

fn main() {
    // Test HashMap
    let t1 = TestWithHashMap {
        id: "test".to_string(),
        metadata: HashMap::new(),
    };
    let enc1 = bincode::serialize(&t1).unwrap();
    match bincode::deserialize::<TestWithHashMap>(&enc1) {
        Ok(_) => println!("✅ HashMap works"),
        Err(e) => println!("❌ HashMap fails: {}", e),
    }
    
    // Test Embedding
    let t2 = TestWithEmbedding {
        id: "test".to_string(),
        embedding: None,
    };
    let enc2 = bincode::serialize(&t2).unwrap();
    match bincode::deserialize::<TestWithEmbedding>(&enc2) {
        Ok(_) => println!("✅ Embedding works"),
        Err(e) => println!("❌ Embedding fails: {}", e),
    }
    
    // Test Both
    let t3 = TestBoth {
        id: "test".to_string(),
        metadata: HashMap::new(),
        embedding: None,
    };
    let enc3 = bincode::serialize(&t3).unwrap();
    match bincode::deserialize::<TestBoth>(&enc3) {
        Ok(_) => println!("✅ Both works"),
        Err(e) => println!("❌ Both fails: {}", e),
    }
}
