use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Simple {
    id: String,
    value: usize,
}

fn main() {
    let simple = Simple {
        id: "test".to_string(),
        value: 42,
    };
    
    println!("Original: {:?}", simple);
    
    // Serialize
    let encoded = bincode::serialize(&simple).unwrap();
    println!("Serialized: {} bytes", encoded.len());
    
    // Deserialize
    match bincode::deserialize::<Simple>(&encoded) {
        Ok(decoded) => {
            println!("✅ Success: {:?}", decoded);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
}
