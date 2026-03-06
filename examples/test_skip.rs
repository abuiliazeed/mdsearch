use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WithSkip {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    value: Option<String>,
}

fn main() {
    let data = WithSkip {
        id: "test".to_string(),
        value: None,
    };
    
    println!("Original: {:?}", data);
    
    let encoded = bincode::serialize(&data).unwrap();
    println!("Serialized: {} bytes", encoded.len());
    
    match bincode::deserialize::<WithSkip>(&encoded) {
        Ok(decoded) => {
            println!("✅ Success: {:?}", decoded);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }
}
