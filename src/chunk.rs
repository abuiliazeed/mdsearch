//! RAG chunking for markdown content

use crate::error::{Error, Result};
use crate::parser::{Document, Parser};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// A chunk of markdown content for RAG
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Chunk {
    /// Unique chunk ID
    pub id: String,

    /// Source file path
    pub file: String,

    /// Chunk content
    pub content: String,

    /// Character range in original file
    pub char_range: (usize, usize),

    /// Line range in original file
    pub line_range: (usize, usize),

    /// Section path (e.g., "Introduction > Getting Started")
    pub section_path: String,

    /// Preceding context (for overlap)
    pub context_before: Option<String>,

    /// Following context (for overlap)
    pub context_after: Option<String>,

    /// Token count (approximate)
    pub token_count: usize,

    /// Metadata from frontmatter
    pub metadata: std::collections::HashMap<String, String>,
}

/// Chunker for creating RAG-ready chunks
pub struct Chunker {
    /// Target chunk size in characters
    pub target_size: usize,

    /// Overlap between chunks
    pub overlap: usize,

    /// Respect markdown structure
    pub respect_structure: bool,

    /// Include code blocks
    pub include_code: bool,
}

impl Default for Chunker {
    fn default() -> Self {
        Self {
            target_size: 512,
            overlap: 50,
            respect_structure: true,
            include_code: true,
        }
    }
}

impl Chunker {
    /// Create a new chunker
    pub fn new(target_size: usize, overlap: usize) -> Self {
        Self {
            target_size,
            overlap,
            respect_structure: true,
            include_code: true,
        }
    }

    /// Chunk a single file
    pub fn chunk_file(&self, path: &Path) -> Result<Vec<Chunk>> {
        let parser = Parser::new();
        let doc = parser.parse_file(path)?;
        self.chunk_document(&doc)
    }

    /// Chunk a parsed document
    pub fn chunk_document(&self, doc: &Document) -> Result<Vec<Chunk>> {
        let mut chunks = Vec::new();
        let file_str = doc.path.to_string_lossy().to_string();
        let metadata = doc
            .frontmatter
            .as_ref()
            .map(|f| f.fields.clone())
            .unwrap_or_default();

        if self.respect_structure {
            // Section-aware chunking
            for section in &doc.sections {
                let section_chunks = self.chunk_section(
                    section,
                    &file_str,
                    &metadata,
                    &doc.sections,
                );
                chunks.extend(section_chunks);
            }
        } else {
            // Simple character-based chunking
            chunks = self.chunk_text(&doc.text, &file_str, &metadata);
        }

        Ok(chunks)
    }

    fn chunk_section(
        &self,
        section: &crate::parser::Section,
        file: &str,
        metadata: &std::collections::HashMap<String, String>,
        all_sections: &[crate::parser::Section],
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        // Build section path
        let section_path = self.build_section_path(section, all_sections);

        // Prepare content (with or without code blocks)
        let content = if self.include_code {
            section.content.clone()
        } else {
            self.remove_code_blocks(&section.content)
        };

        // If content fits in one chunk, use it directly
        if content.len() <= self.target_size {
            let token_count = self.estimate_tokens(&content);
            let char_len = content.len();
            chunks.push(Chunk {
                id: format!("{}-{}-{}", file, section.line_range.0, 0),
                file: file.to_string(),
                content,
                char_range: (0, char_len),
                line_range: section.line_range,
                section_path: section_path.clone(),
                context_before: None,
                context_after: None,
                token_count,
                metadata: metadata.clone(),
            });
        } else {
            // Split into multiple chunks respecting sentence boundaries
            let splits = self.smart_split(&content);
            for (i, (text, range)) in splits.iter().enumerate() {
                let token_count = self.estimate_tokens(text);

                // Add overlap context
                let context_before = if i > 0 && self.overlap > 0 {
                    splits.get(i - 1).map(|(prev, _)| {
                        prev.chars()
                            .rev()
                            .take(self.overlap)
                            .collect::<String>()
                            .chars()
                            .rev()
                            .collect()
                    })
                } else {
                    None
                };

                let context_after = if i < splits.len() - 1 && self.overlap > 0 {
                    splits.get(i + 1).map(|(next, _)| {
                        next.chars().take(self.overlap).collect()
                    })
                } else {
                    None
                };

                chunks.push(Chunk {
                    id: format!("{}-{}-{}", file, section.line_range.0, i),
                    file: file.to_string(),
                    content: text.clone(),
                    char_range: *range,
                    line_range: section.line_range,
                    section_path: section_path.clone(),
                    context_before,
                    context_after,
                    token_count,
                    metadata: metadata.clone(),
                });
            }
        }

        chunks
    }

    fn chunk_text(
        &self,
        text: &str,
        file: &str,
        metadata: &std::collections::HashMap<String, String>,
    ) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        let splits = self.smart_split(text);

        for (i, (content, range)) in splits.iter().enumerate() {
            let token_count = self.estimate_tokens(content);

            chunks.push(Chunk {
                id: format!("{}-{}", file, i),
                file: file.to_string(),
                content: content.clone(),
                char_range: *range,
                line_range: (0, 0), // Not tracked in simple mode
                section_path: String::new(),
                context_before: None,
                context_after: None,
                token_count,
                metadata: metadata.clone(),
            });
        }

        chunks
    }

    fn smart_split(&self, text: &str) -> Vec<(String, (usize, usize))> {
        let mut chunks = Vec::new();
        let mut current_chunk = String::new();
        let mut chunk_start = 0;
        let mut last_sentence_end = 0;
        let mut pos = 0;

        let sentence_enders = ['.', '!', '?', '\n'];

        for (i, c) in text.char_indices() {
            current_chunk.push(c);
            pos = i + c.len_utf8();

            if sentence_enders.contains(&c) {
                last_sentence_end = pos;
            }

            if current_chunk.len() >= self.target_size {
                // Try to break at sentence boundary
                if last_sentence_end > chunk_start && last_sentence_end < pos {
                    let break_point = last_sentence_end;
                    let chunk_text: String = text[chunk_start..break_point].chars().collect();

                    chunks.push((chunk_text, (chunk_start, break_point)));

                    current_chunk = text[break_point..].chars().collect();
                    chunk_start = break_point;
                    last_sentence_end = break_point;
                } else {
                    // Force break at target size
                    let chunk_text = current_chunk.clone();
                    chunks.push((chunk_text, (chunk_start, pos)));

                    current_chunk = String::new();
                    chunk_start = pos;
                    last_sentence_end = pos;
                }
            }
        }

        // Don't forget the last chunk
        if !current_chunk.is_empty() {
            chunks.push((current_chunk, (chunk_start, text.len())));
        }

        chunks
    }

    fn build_section_path(
        &self,
        section: &crate::parser::Section,
        all_sections: &[crate::parser::Section],
    ) -> String {
        // Build hierarchical path from parent headings
        let mut path = Vec::new();

        for s in all_sections {
            if s.line_range.0 >= section.line_range.0 {
                break;
            }
            if s.level < section.level {
                if let Some(ref title) = s.title {
                    path.push(title.clone());
                }
            }
        }

        if let Some(ref title) = section.title {
            path.push(title.clone());
        }

        path.join(" > ")
    }

    fn remove_code_blocks(&self, content: &str) -> String {
        let mut result = String::new();
        let mut in_code = false;

        for line in content.lines() {
            if line.starts_with("```") {
                in_code = !in_code;
                continue;
            }
            if !in_code {
                result.push_str(line);
                result.push('\n');
            }
        }

        result
    }

    fn estimate_tokens(&self, text: &str) -> usize {
        // Simple estimation: ~4 characters per token for English
        // This is rough but good enough for chunking decisions
        text.len() / 4
    }
}

/// Run the chunk command
pub fn run_chunk(
    path: std::path::PathBuf,
    size: usize,
    overlap: usize,
    format: String,
    include_metadata: bool,
) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};

    let chunker = Chunker::new(size, overlap);

    if path.is_file() {
        let chunks = chunker.chunk_file(&path)?;
        output_chunks(&chunks, &format, include_metadata);
    } else if path.is_dir() {
        let md_files: Vec<std::path::PathBuf> = walkdir::WalkDir::new(&path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "md" || ext == "markdown")
                    .unwrap_or(false)
            })
            .map(|e| e.path().to_path_buf())
            .collect();

        let pb = ProgressBar::new(md_files.len() as u64);
        pb.set_style(
            ProgressStyle::with_template(
                "{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}",
            )
            .unwrap(),
        );

        let mut all_chunks = Vec::new();
        for file in md_files {
            pb.set_message(format!("Chunking {:?}", file.file_name().unwrap()));
            if let Ok(chunks) = chunker.chunk_file(&file) {
                all_chunks.extend(chunks);
            }
            pb.inc(1);
        }

        pb.finish_with_message(format!("Chunked {} files", all_chunks.len()));
        output_chunks(&all_chunks, &format, include_metadata);
    } else {
        return Err(Error::FileNotFound(path));
    }

    Ok(())
}

fn output_chunks(chunks: &[Chunk], format: &str, include_metadata: bool) {
    match format {
        "json" => {
            if include_metadata {
                println!("{}", serde_json::to_string_pretty(&chunks).unwrap());
            } else {
                let simplified: Vec<_> = chunks
                    .iter()
                    .map(|c| serde_json::json!({
                        "id": c.id,
                        "file": c.file,
                        "content": c.content,
                        "section_path": c.section_path,
                    }))
                    .collect();
                println!("{}", serde_json::to_string_pretty(&simplified).unwrap());
            }
        }
        "jsonl" => {
            for chunk in chunks {
                if include_metadata {
                    println!("{}", serde_json::to_string(chunk).unwrap());
                } else {
                    println!(
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "id": chunk.id,
                            "file": chunk.file,
                            "content": chunk.content,
                            "section_path": chunk.section_path,
                        }))
                        .unwrap()
                    );
                }
            }
        }
        _ => {
            for chunk in chunks {
                println!("--- {} ---", chunk.id);
                println!("{}", chunk.content);
                println!();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_respects_target_size() {
        let text = "A".repeat(1000);
        let chunker = Chunker::new(100, 0);
        let chunks = chunker.smart_split(&text);

        assert!(chunks.iter().all(|(c, _)| c.len() <= 150)); // Allow some overflow
    }

    #[test]
    fn test_chunker_respects_sentence_boundaries() {
        let text = "First sentence. Second sentence. Third sentence.";
        let chunker = Chunker::new(20, 0);
        let chunks = chunker.smart_split(text);

        // Should break at sentence boundaries, not mid-word
        for (chunk, _) in &chunks {
            assert!(
                chunk.ends_with('.') || chunk.ends_with('\n'),
                "Chunk doesn't end at sentence boundary: {:?}",
                chunk
            );
        }
    }
}
