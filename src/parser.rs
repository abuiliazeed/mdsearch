//! Markdown parser with structure awareness

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Parsed markdown document with structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    /// File path
    pub path: PathBuf,

    /// File content hash
    pub hash: u64,

    /// Parsed sections
    pub sections: Vec<Section>,

    /// Frontmatter (if present)
    pub frontmatter: Option<Frontmatter>,

    /// All text content (for indexing)
    pub text: String,

    /// Links found in the document
    pub links: Vec<Link>,

    /// Last modified timestamp
    pub modified: Option<u64>,
}

/// A section of markdown content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Section {
    /// Section level (1-6 for headings, 0 for content)
    pub level: u8,

    /// Section title (if heading)
    pub title: Option<String>,

    /// Content text
    pub content: String,

    /// Line range in original file
    pub line_range: (usize, usize),

    /// Code blocks in this section
    pub code_blocks: Vec<CodeBlock>,

    /// Section slug (derived from title)
    pub slug: Option<String>,
}

/// Code block within a section
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeBlock {
    /// Language identifier
    pub language: Option<String>,

    /// Code content
    pub code: String,

    /// Line range in original file
    pub line_range: (usize, usize),
}

/// Frontmatter metadata
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frontmatter {
    /// Raw frontmatter content
    pub raw: String,

    /// Parsed fields (if YAML/JSON)
    pub fields: std::collections::HashMap<String, String>,
}

/// A link in the document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Link {
    /// Link text
    pub text: String,

    /// Link target (URL or file path)
    pub target: String,

    /// Whether it's an internal link
    pub internal: bool,
}

/// Markdown parser
pub struct Parser {
    /// Extract code blocks
    pub extract_code: bool,

    /// Extract links
    pub extract_links: bool,

    /// Parse frontmatter
    pub parse_frontmatter: bool,
}

impl Default for Parser {
    fn default() -> Self {
        Self {
            extract_code: true,
            extract_links: true,
            parse_frontmatter: true,
        }
    }
}

impl Parser {
    /// Create a new parser
    pub fn new() -> Self {
        Self::default()
    }

    /// Parse a markdown file
    pub fn parse_file(&self, path: &Path) -> Result<Document> {
        let content =
            std::fs::read_to_string(path).map_err(|_| Error::FileNotFound(path.to_path_buf()))?;

        self.parse(path, &content)
    }

    /// Parse markdown content
    pub fn parse(&self, path: &Path, content: &str) -> Result<Document> {
        let hash = self.hash_content(content);
        let modified = self.get_modified_time(path);

        let (frontmatter, content_start) = if self.parse_frontmatter {
            self.extract_frontmatter(content)
        } else {
            (None, 0)
        };

        let main_content = &content[content_start..];
        let sections = self.extract_sections(main_content, content_start);
        let links = if self.extract_links {
            self.extract_links_from_content(main_content)
        } else {
            Vec::new()
        };

        let text = self.extract_text(main_content);

        Ok(Document {
            path: path.to_path_buf(),
            hash,
            sections,
            frontmatter,
            text,
            links,
            modified,
        })
    }

    fn hash_content(&self, content: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        hasher.finish()
    }

    fn get_modified_time(&self, path: &Path) -> Option<u64> {
        use std::fs;
        fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
    }

    fn extract_frontmatter(&self, content: &str) -> (Option<Frontmatter>, usize) {
        let content = content.trim_start();

        // Check for YAML frontmatter
        if content.starts_with("---\n") {
            if let Some(end) = content[4..].find("\n---\n") {
                let frontmatter_content = &content[4..end + 4];
                let fields = self.parse_yaml_frontmatter(frontmatter_content);

                return (
                    Some(Frontmatter {
                        raw: frontmatter_content.to_string(),
                        fields,
                    }),
                    end + 9, // Skip opening ---, content, and closing ---
                );
            }
        }

        // Check for TOML frontmatter
        if content.starts_with("+++\n") {
            if let Some(end) = content[4..].find("\n+++\n") {
                let frontmatter_content = &content[4..end + 4];
                let fields = self.parse_toml_frontmatter(frontmatter_content);

                return (
                    Some(Frontmatter {
                        raw: frontmatter_content.to_string(),
                        fields,
                    }),
                    end + 9,
                );
            }
        }

        (None, 0)
    }

    fn parse_yaml_frontmatter(&self, content: &str) -> std::collections::HashMap<String, String> {
        let mut fields = std::collections::HashMap::new();

        for line in content.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let key = key.trim().to_string();
                let value = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !key.is_empty() {
                    fields.insert(key, value);
                }
            }
        }

        fields
    }

    fn parse_toml_frontmatter(&self, content: &str) -> std::collections::HashMap<String, String> {
        let mut fields = std::collections::HashMap::new();

        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !key.is_empty() {
                    fields.insert(key, value);
                }
            }
        }

        fields
    }

    fn extract_sections(&self, content: &str, offset: usize) -> Vec<Section> {
        let mut sections = Vec::new();
        let mut current_section = Section {
            level: 0,
            title: None,
            content: String::new(),
            line_range: (1, 1),
            code_blocks: Vec::new(),
            slug: None,
        };

        let mut in_code_block = false;
        let mut code_block_start = 0;
        let mut current_code_lang: Option<String> = None;
        let mut current_code: String = String::new();
        let mut line_num = 1 + offset;
        let mut section_start = 1;

        for line in content.lines() {
            // Track code blocks
            if line.starts_with("```") {
                if in_code_block {
                    // End code block
                    if self.extract_code {
                        current_section.code_blocks.push(CodeBlock {
                            language: current_code_lang.take(),
                            code: std::mem::take(&mut current_code),
                            line_range: (code_block_start, line_num),
                        });
                    }
                    in_code_block = false;
                } else {
                    // Start code block
                    in_code_block = true;
                    code_block_start = line_num;
                    current_code_lang = line[3..].split_whitespace().next().map(|s| s.to_string());
                }
                line_num += 1;
                continue;
            }

            if in_code_block {
                if self.extract_code {
                    current_code.push_str(line);
                    current_code.push('\n');
                }
                line_num += 1;
                continue;
            }

            // Check for heading
            let heading_level = self.get_heading_level(line);
            if heading_level > 0 {
                // Save current section if it has content
                if !current_section.content.trim().is_empty() {
                    current_section.line_range = (section_start, line_num - 1);
                    sections.push(current_section);
                }

                // Start new section
                let title = line[heading_level..].trim().to_string();
                section_start = line_num;
                current_section = Section {
                    level: heading_level as u8,
                    title: Some(title.clone()),
                    content: String::new(),
                    line_range: (section_start, section_start),
                    code_blocks: Vec::new(),
                    slug: Some(self.slugify(&title)),
                };
            } else {
                // Add to current section
                if !current_section.content.is_empty() {
                    current_section.content.push('\n');
                }
                current_section.content.push_str(line);
            }

            line_num += 1;
        }

        // Don't forget the last section
        if !current_section.content.trim().is_empty() {
            current_section.line_range = (section_start, line_num - 1);
            sections.push(current_section);
        }

        sections
    }

    fn get_heading_level(&self, line: &str) -> usize {
        let trimmed = line.trim_start();
        let count = trimmed.chars().take_while(|&c| c == '#').count();
        if count > 0 && count <= 6 && trimmed.chars().nth(count) == Some(' ') {
            count
        } else {
            0
        }
    }

    fn slugify(&self, text: &str) -> String {
        text.to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-")
    }

    fn extract_links_from_content(&self, content: &str) -> Vec<Link> {
        LinkExtractor.extract(content)
    }

    fn extract_text(&self, content: &str) -> String {
        use pulldown_cmark::{Options, Parser};

        let mut text = String::new();
        let parser = Parser::new_ext(content, Options::empty());

        for event in parser {
            use pulldown_cmark::Event;
            if let Event::Text(t) = event {
                text.push_str(&t);
                text.push(' ');
            }
        }

        text
    }
}

impl LinkExtractor {
    fn extract(&self, content: &str) -> Vec<Link> {
        let mut links = Vec::new();
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            // Look for [text](target) pattern
            if chars[i] == '[' {
                let text_start = i + 1;
                if let Some(text_end) = self.find_matching_bracket(&chars, i) {
                    let text: String = chars[text_start..text_end].iter().collect();

                    if text_end + 1 < chars.len() && chars[text_end + 1] == '(' {
                        let target_start = text_end + 2;
                        if let Some(target_end) = self.find_matching_paren(&chars, text_end + 1) {
                            let target: String = chars[target_start..target_end].iter().collect();

                            links.push(Link {
                                text,
                                target: target.clone(),
                                internal: !target.starts_with("http") && !target.starts_with("//"),
                            });

                            i = target_end + 1;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }

        links
    }

    fn find_matching_bracket(&self, chars: &[char], start: usize) -> Option<usize> {
        let mut depth = 1;
        for (i, &c) in chars[start + 1..].iter().enumerate() {
            match c {
                '[' => depth += 1,
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(start + 1 + i);
                    }
                }
                _ => {}
            }
        }
        None
    }

    fn find_matching_paren(&self, chars: &[char], start: usize) -> Option<usize> {
        let mut depth = 1;
        for (i, &c) in chars[start + 1..].iter().enumerate() {
            match c {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(start + 1 + i);
                    }
                }
                _ => {}
            }
        }
        None
    }
}

struct LinkExtractor;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_markdown() {
        let content = r#"
# Heading 1

Some content here.

## Heading 2

More content.

```rust
fn main() {}
```
"#;

        let parser = Parser::new();
        let doc = parser.parse(Path::new("test.md"), content).unwrap();

        assert_eq!(doc.sections.len(), 2);
        assert_eq!(doc.sections[0].title, Some("Heading 1".to_string()));
        assert_eq!(doc.sections[0].level, 1);
    }

    #[test]
    fn test_slugify() {
        let parser = Parser::new();
        assert_eq!(parser.slugify("Hello World"), "hello-world");
        assert_eq!(parser.slugify("API Reference"), "api-reference");
    }
}
