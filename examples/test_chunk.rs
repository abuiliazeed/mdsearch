use std::path::Path;

fn main() {
    let store = mdsearch::Store::open(Path::new("/tmp/mdsearch-test/.mdsearch")).unwrap();
    
    // Try to get a specific chunk
    let chunk_id = "./note1.md-3-0";
    match store.get_chunk(chunk_id) {
        Ok(Some(chunk)) => {
            println!("✅ Successfully retrieved chunk: {}", chunk.id);
            println!("  File: {}", chunk.file);
            println!("  Content: {} bytes", chunk.content.len());
        }
        Ok(None) => {
            println!("❌ Chunk not found: {}", chunk_id);
        }
        Err(e) => {
            println!("❌ Error retrieving chunk: {}", e);
        }
    }
}
