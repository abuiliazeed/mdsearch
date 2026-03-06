use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Test1 {
    id: String,
    file: String,
    content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Test2 {
    id: String,
    file: String,
    content: String,
    char_range: (usize, usize),
    line_range: (usize, usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Test3 {
    id: String,
    file: String,
    content: String,
    char_range: (usize, usize),
    line_range: (usize, usize),
    section_path: String,
    context_before: Option<String>,
    context_after: Option<String>,
    token_count: usize,
}

fn main() {
    // Test 1
    let t1 = Test1 {
        id: "test".to_string(),
        file: "test.md".to_string(),
        content: "Hello".to_string(),
    };
    let enc1 = bincode::serialize(&t1).unwrap();
    match bincode::deserialize::<Test1>(&enc1) {
        Ok(_) => println!("✅ Test1 works"),
        Err(e) => println!("❌ Test1 fails: {}", e),
    }
    
    // Test 2
    let t2 = Test2 {
        id: "test".to_string(),
        file: "test.md".to_string(),
        content: "Hello".to_string(),
        char_range: (0, 5),
        line_range: (1, 1),
    };
    let enc2 = bincode::serialize(&t2).unwrap();
    match bincode::deserialize::<Test2>(&enc2) {
        Ok(_) => println!("✅ Test2 works"),
        Err(e) => println!("❌ Test2 fails: {}", e),
    }
    
    // Test 3
    let t3 = Test3 {
        id: "test".to_string(),
        file: "test.md".to_string(),
        content: "Hello".to_string(),
        char_range: (0, 5),
        line_range: (1, 1),
        section_path: "Test".to_string(),
        context_before: None,
        context_after: None,
        token_count: 1,
    };
    let enc3 = bincode::serialize(&t3).unwrap();
    match bincode::deserialize::<Test3>(&enc3) {
        Ok(_) => println!("✅ Test3 works"),
        Err(e) => println!("❌ Test3 fails: {}", e),
    }
}
