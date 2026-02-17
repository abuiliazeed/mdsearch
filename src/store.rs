//! Storage layer using RocksDB

use crate::chunk::Chunk;
use crate::error::{Error, Result};
use crate::parser::Document;
use rocksdb::{ColumnFamily, DB, Options, WriteBatch};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::sync::Arc;

/// Key types for the store
mod keys {
    pub const DOCUMENTS: &str = "documents";
    pub const CHUNKS: &str = "chunks";
    pub const TERMS: &str = "terms";
    pub const METADATA: &str = "metadata";
}

/// Index metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    pub version: u32,
    pub created_at: u64,
    pub updated_at: u64,
    pub document_count: u64,
    pub chunk_count: u64,
    pub total_bytes: u64,
}

impl Default for IndexMetadata {
    fn default() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self {
            version: 1,
            created_at: now,
            updated_at: now,
            document_count: 0,
            chunk_count: 0,
            total_bytes: 0,
        }
    }
}

/// Document record for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRecord {
    pub path: String,
    pub hash: u64,
    pub modified: Option<u64>,
    pub indexed_at: u64,
    pub chunk_count: usize,
}

/// Term posting for inverted index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TermPosting {
    pub term: String,
    pub doc_ids: Vec<(String, u32)>, // (doc_id, frequency)
    pub total_frequency: u64,
}

/// Storage backend using RocksDB
pub struct Store {
    db: Arc<DB>,
    path: std::path::PathBuf,
}

impl Store {
    /// Open or create a store at the given path
    pub fn open(path: &Path) -> Result<Self> {
        let mut options = Options::default();
        options.create_if_missing(true);
        options.create_missing_column_families(true);

        // Configure for performance
        options.set_max_open_files(1000);
        options.set_keep_log_file_num(3);
        options.set_max_log_file_size(1024 * 1024); // 1MB
        options.set_compression_type(rocksdb::DBCompressionType::Lz4);

        let cfs = vec![
            rocksdb::ColumnFamilyDescriptor::new(keys::DOCUMENTS, Options::default()),
            rocksdb::ColumnFamilyDescriptor::new(keys::CHUNKS, Options::default()),
            rocksdb::ColumnFamilyDescriptor::new(keys::TERMS, Options::default()),
            rocksdb::ColumnFamilyDescriptor::new(keys::METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&options, path, cfs)
            .map_err(|e| Error::Storage(format!("Failed to open database: {}", e)))?;

        let store = Self {
            db: Arc::new(db),
            path: path.to_path_buf(),
        };

        // Initialize metadata if needed
        store.ensure_metadata()?;

        Ok(store)
    }

    fn ensure_metadata(&self) -> Result<()> {
        let cf = self.cf_handle(keys::METADATA)?;
        if self.db.get_cf(&cf, b"index_meta")?.is_none() {
            let metadata = IndexMetadata::default();
            self.put_value(&cf, b"index_meta", &metadata)?;
        }
        Ok(())
    }

    /// Get column family handle by name
    fn cf_handle(&self, name: &str) -> Result<&ColumnFamily> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| Error::Storage(format!("Column family not found: {}", name)))
    }

    /// Put a serialized value into a column family
    fn put_value<K: AsRef<[u8]>, V: Serialize>(&self, cf: &ColumnFamily, key: K, value: &V) -> Result<()> {
        let encoded = bincode::serialize(value)?;
        self.db
            .put_cf(cf, key, encoded)
            .map_err(Error::from)
    }

    /// Get a deserialized value from a column family
    fn get_value<K: AsRef<[u8]>, V: for<'de> Deserialize<'de>>(
        &self,
        cf: &ColumnFamily,
        key: K,
    ) -> Result<Option<V>> {
        match self.db.get_cf(cf, key)? {
            Some(bytes) => {
                let value = bincode::deserialize(&bytes)?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// Get index metadata
    pub fn metadata(&self) -> Result<IndexMetadata> {
        let cf = self.cf_handle(keys::METADATA)?;
        self.get_value(&cf, b"index_meta")?
            .ok_or_else(|| Error::Storage("Metadata not found".into()))
    }

    /// Update index metadata
    pub fn update_metadata(&self, f: impl FnOnce(&mut IndexMetadata)) -> Result<()> {
        let cf = self.cf_handle(keys::METADATA)?;
        let mut metadata = self.metadata()?;
        f(&mut metadata);
        metadata.updated_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        self.put_value(&cf, b"index_meta", &metadata)
    }

    /// Index a document
    pub fn index_document(&self, doc: &Document) -> Result<()> {
        let cf = self.cf_handle(keys::DOCUMENTS)?;

        let record = DocumentRecord {
            path: doc.path.to_string_lossy().to_string(),
            hash: doc.hash,
            modified: doc.modified,
            indexed_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            chunk_count: 0,
        };

        let key = doc.path.to_string_lossy();
        self.put_value(&cf, key.as_bytes(), &record)?;

        // Update metadata
        self.update_metadata(|m| {
            m.document_count += 1;
        })?;

        Ok(())
    }

    /// Store a chunk
    pub fn store_chunk(&self, chunk: &Chunk) -> Result<()> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        self.put_value(&cf, chunk.id.as_bytes(), chunk)?;

        self.update_metadata(|m| {
            m.chunk_count += 1;
        })?;

        Ok(())
    }

    /// Store chunks in batch
    pub fn store_chunks_batch(&self, chunks: &[Chunk]) -> Result<()> {
        let cf = self.cf_handle(keys::CHUNKS)?;

        for chunk in chunks {
            let key = chunk.id.as_bytes();
            self.put_value(cf, key, chunk)?;
        }

        self.update_metadata(|m| {
            m.chunk_count += chunks.len() as u64;
        })?;

        Ok(())
    }

    /// Get a chunk by ID
    pub fn get_chunk(&self, id: &str) -> Result<Option<Chunk>> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        self.get_value(&cf, id.as_bytes())
    }

    /// Update a chunk (with embedding)
    pub fn update_chunk(&self, chunk: &Chunk) -> Result<()> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        self.put_value(&cf, chunk.id.as_bytes(), chunk)
    }

    /// Get all chunks
    pub fn get_all_chunks(&self) -> Result<Vec<Chunk>> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        let mut chunks = Vec::new();

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            match item {
                Ok((key, value)) => {
                    match bincode::deserialize::<Chunk>(&value) {
                        Ok(chunk) => chunks.push(chunk),
                        Err(e) => {
                            eprintln!("DEBUG: Failed to deserialize chunk: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("DEBUG: Iterator error: {}", e);
                }
            }
        }

        Ok(chunks)
    }

    /// Get all chunks for a document
    pub fn get_chunks_for_document(&self, doc_path: &str) -> Result<Vec<Chunk>> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        let mut chunks = Vec::new();

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item?;
            if let Ok(chunk) = bincode::deserialize::<Chunk>(&value) {
                if chunk.file == doc_path {
                    chunks.push(chunk);
                }
            }
        }

        Ok(chunks)
    }

    /// Store term posting
    pub fn store_term(&self, term: &str, posting: &TermPosting) -> Result<()> {
        let cf = self.cf_handle(keys::TERMS)?;
        self.put_value(&cf, term.as_bytes(), posting)
    }

    /// Get term posting
    pub fn get_term(&self, term: &str) -> Result<Option<TermPosting>> {
        let cf = self.cf_handle(keys::TERMS)?;
        self.get_value(&cf, term.as_bytes())
    }

    /// Search for chunks containing text (basic implementation)
    pub fn search_chunks(&self, query: &str, limit: usize) -> Result<Vec<Chunk>> {
        let cf = self.cf_handle(keys::CHUNKS)?;
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            if results.len() >= limit {
                break;
            }

            let (key, value) = item?;
            if let Ok(chunk) = bincode::deserialize::<Chunk>(&value) {
                if chunk.content.to_lowercase().contains(&query_lower) {
                    results.push(chunk);
                }
            }
        }

        Ok(results)
    }

    /// Get document by path
    pub fn get_document(&self, path: &str) -> Result<Option<DocumentRecord>> {
        let cf = self.cf_handle(keys::DOCUMENTS)?;
        self.get_value(&cf, path.as_bytes())
    }

    /// List all documents
    pub fn list_documents(&self) -> Result<Vec<DocumentRecord>> {
        let cf = self.cf_handle(keys::DOCUMENTS)?;
        let mut docs = Vec::new();

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (_, value) = item?;
            if let Ok(doc) = bincode::deserialize::<DocumentRecord>(&value) {
                docs.push(doc);
            }
        }

        Ok(docs)
    }

    /// Delete a document and its chunks
    pub fn delete_document(&self, path: &str) -> Result<()> {
        // Delete chunks first
        let chunks = self.get_chunks_for_document(path)?;
        let cf_chunks = self.cf_handle(keys::CHUNKS)?;
        let mut batch = WriteBatch::default();

        for chunk in chunks {
            batch.delete_cf(cf_chunks, chunk.id.as_bytes());
        }

        // Delete document record
        let cf_docs = self.cf_handle(keys::DOCUMENTS)?;
        batch.delete_cf(cf_docs, path.as_bytes());

        self.db.write(batch)?;

        self.update_metadata(|m| {
            m.document_count = m.document_count.saturating_sub(1);
        })?;

        Ok(())
    }

    /// Clear all data
    pub fn clear(&self) -> Result<()> {
        let cfs = [keys::DOCUMENTS, keys::CHUNKS, keys::TERMS];

        for cf_name in cfs {
            let cf = self.cf_handle(cf_name)?;
            let mut batch = WriteBatch::default();

            let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
            for item in iter {
                let (key, _) = item?;
                batch.delete_cf(cf, &key);
            }

            self.db.write(batch)?;
        }

        // Reset metadata
        let cf = self.cf_handle(keys::METADATA)?;
        self.put_value(&cf, b"index_meta", &IndexMetadata::default())?;

        Ok(())
    }

    /// Get store path
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Check if store exists
    pub fn exists(path: &Path) -> bool {
        path.join("CURRENT").exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_store_open_create() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");

        let store = Store::open(&path).unwrap();
        let meta = store.metadata().unwrap();

        assert_eq!(meta.version, 1);
        assert_eq!(meta.document_count, 0);
    }
}
