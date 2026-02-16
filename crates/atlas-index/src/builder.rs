use atlas_core::{ChunkKind, DeepIndex, FileEntry, FileInfo, TermFreqs};
use atlas_treesit::{Chunker, RegexChunker};
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Builds a DeepIndex from a list of scanned files.
pub struct IndexBuilder<'a> {
    root: &'a Path,
}

impl<'a> IndexBuilder<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// Build a deep index from a list of scanned file metadata.
    ///
    /// When `existing` is provided, files whose SHA-256 matches the existing
    /// entry are carried forward without re-reading or re-indexing.
    ///
    /// Returns `(index, reindexed_count)` — the number of files that were
    /// actually re-indexed (0 means nothing changed).
    pub fn build(
        &self,
        files: &[FileInfo],
        existing: Option<&DeepIndex>,
    ) -> anyhow::Result<(DeepIndex, usize)> {
        use std::sync::atomic::{AtomicUsize, Ordering};
        let reindexed = AtomicUsize::new(0);

        // Process files in parallel, skipping unchanged files
        let entries: Vec<(String, FileEntry)> = files
            .par_iter()
            .filter_map(|info| {
                // Skip unchanged files — carry forward existing entry
                if let Some(existing) = existing
                    && let Some(old_entry) = existing.files.get(&info.path)
                    && old_entry.sha256 == info.sha256
                {
                    return Some((info.path.clone(), old_entry.clone()));
                }

                let full_path = self.root.join(&info.path);
                let content = fs::read_to_string(&full_path).ok()?;
                let entry = build_file_entry(info, &content);
                reindexed.fetch_add(1, Ordering::Relaxed);
                Some((info.path.clone(), entry))
            })
            .collect();

        let reindexed_count = reindexed.load(Ordering::Relaxed);

        // Compute corpus-level stats
        let total_docs = entries.len() as u32;
        let total_length: u32 = entries.iter().map(|(_, e)| e.doc_length).sum();
        let avg_doc_length = if total_docs > 0 {
            total_length as f64 / total_docs as f64
        } else {
            1.0
        };

        // Document frequencies: how many docs contain each term
        let mut doc_frequencies: HashMap<String, u32> = HashMap::new();
        for (_, entry) in &entries {
            for term in entry.term_frequencies.keys() {
                *doc_frequencies.entry(term.clone()).or_default() += 1;
            }
        }

        let file_map: HashMap<String, FileEntry> = entries.into_iter().collect();

        Ok((
            DeepIndex {
                version: 1,
                files: file_map,
                avg_doc_length,
                total_docs,
                doc_frequencies,
            },
            reindexed_count,
        ))
    }
}

/// Build a FileEntry from file metadata and content.
fn build_file_entry(info: &FileInfo, content: &str) -> FileEntry {
    let mut term_frequencies: HashMap<String, TermFreqs> = HashMap::new();

    // Tokenize filename for filename field
    let filename_tokens = tokenize_path(&info.path);
    for token in &filename_tokens {
        term_frequencies.entry(token.clone()).or_default().filename += 1;
    }

    // Tokenize content for body field
    let body_tokens = tokenize_content(content);
    let doc_length = body_tokens.len() as u32;
    for token in &body_tokens {
        term_frequencies.entry(token.clone()).or_default().body += 1;
    }

    // Extract chunks via regex (fast indexing pass)
    let chunks = RegexChunker.chunk(content, info.language);

    // Tokenize chunk names for symbols field
    for chunk in &chunks {
        if matches!(
            chunk.kind,
            ChunkKind::Function | ChunkKind::Type | ChunkKind::Impl
        ) {
            let symbol_tokens = tokenize_identifier(&chunk.name);
            for token in &symbol_tokens {
                term_frequencies.entry(token.clone()).or_default().symbols += 1;
            }
        }
    }

    FileEntry {
        sha256: info.sha256,
        chunks,
        term_frequencies,
        doc_length,
    }
}

/// Tokenize a file path into search terms.
fn tokenize_path(path: &str) -> Vec<String> {
    path.split(['/', '.', '-', '_'])
        .flat_map(split_camel_case)
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Tokenize file content into search terms (whitespace-split, lowercased, min length 2).
fn tokenize_content(content: &str) -> Vec<String> {
    content
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .flat_map(|word| {
            word.split('_')
                .flat_map(split_camel_case)
                .collect::<Vec<_>>()
        })
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Tokenize a single identifier (function/type name).
fn tokenize_identifier(name: &str) -> Vec<String> {
    name.split('_')
        .flat_map(split_camel_case)
        .filter(|t| t.len() >= 2)
        .map(|t| t.to_lowercase())
        .collect()
}

/// Simple camelCase splitting.
fn split_camel_case(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return parts;
    }

    let mut start = 0;
    for i in 1..bytes.len() {
        let prev_upper = bytes[i - 1].is_ascii_uppercase();
        let curr_upper = bytes[i].is_ascii_uppercase();
        let curr_lower = bytes[i].is_ascii_lowercase();

        let split_camel = !prev_upper && curr_upper;
        let split_acronym = prev_upper && curr_lower && i >= 2 && bytes[i - 2].is_ascii_uppercase();

        if split_camel {
            parts.push(s[start..i].to_string());
            start = i;
        } else if split_acronym {
            if start < i - 1 {
                parts.push(s[start..i - 1].to_string());
            }
            start = i - 1;
        }
    }

    if start < s.len() {
        parts.push(s[start..].to_string());
    }

    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{ChunkKind, Language};
    use std::fs;

    fn make_file_info(path: &str, content: &str) -> FileInfo {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();

        FileInfo {
            path: path.to_string(),
            size: content.len() as u64,
            language: Language::from_path(Path::new(path)),
            role: atlas_core::FileRole::from_path(Path::new(path)),
            sha256: hash,
        }
    }

    #[test]
    fn build_index_from_files() {
        let dir = tempfile::tempdir().unwrap();
        let content = "fn main() {\n    println!(\"hello\");\n}\n";
        fs::write(dir.path().join("main.rs"), content).unwrap();

        let files = vec![make_file_info("main.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        assert_eq!(index.total_docs, 1);
        assert!(index.files.contains_key("main.rs"));
    }

    #[test]
    fn index_term_frequencies() {
        let dir = tempfile::tempdir().unwrap();
        let content = "fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}\n";
        fs::write(dir.path().join("auth.rs"), content).unwrap();

        let files = vec![make_file_info("auth.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        let entry = &index.files["auth.rs"];
        // "auth" should appear in filename field
        assert!(entry.term_frequencies.contains_key("auth"));
        let auth_tf = &entry.term_frequencies["auth"];
        assert!(auth_tf.filename > 0);

        // "token" should appear in body field
        assert!(entry.term_frequencies.contains_key("token"));
        let token_tf = &entry.term_frequencies["token"];
        assert!(token_tf.body > 0);
    }

    #[test]
    fn index_extracts_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let content = "pub fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}\n\npub struct AuthConfig {\n    pub secret: String,\n}\n";
        fs::write(dir.path().join("auth.rs"), content).unwrap();

        let files = vec![make_file_info("auth.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        let entry = &index.files["auth.rs"];
        assert!(entry.chunks.len() >= 2);

        let fn_chunk = entry.chunks.iter().find(|c| c.kind == ChunkKind::Function);
        assert!(fn_chunk.is_some());
        assert_eq!(fn_chunk.unwrap().name, "authenticate");

        let struct_chunk = entry.chunks.iter().find(|c| c.kind == ChunkKind::Type);
        assert!(struct_chunk.is_some());
        assert_eq!(struct_chunk.unwrap().name, "AuthConfig");
    }

    #[test]
    fn index_doc_frequencies() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(
            dir.path().join("auth.rs"),
            "fn authenticate() {}\nfn verify() {}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("handler.rs"),
            "fn handle() {}\nfn authenticate() {}\n",
        )
        .unwrap();

        let files = vec![
            make_file_info("auth.rs", "fn authenticate() {}\nfn verify() {}\n"),
            make_file_info("handler.rs", "fn handle() {}\nfn authenticate() {}\n"),
        ];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        assert_eq!(index.total_docs, 2);
        // "authenticate" appears in both files
        assert_eq!(index.doc_frequencies.get("authenticate"), Some(&2));
    }

    #[test]
    fn index_empty_files() {
        let dir = tempfile::tempdir().unwrap();
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&[], None).unwrap().0;

        assert_eq!(index.total_docs, 0);
        assert!(index.files.is_empty());
    }

    #[test]
    fn index_symbol_term_frequencies() {
        let dir = tempfile::tempdir().unwrap();
        let content = "pub fn parseHTTPResponse() {}\n";
        fs::write(dir.path().join("parser.rs"), content).unwrap();

        let files = vec![make_file_info("parser.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        let entry = &index.files["parser.rs"];
        // "parse" should appear in symbols field from chunk name "parseHTTPResponse"
        let parse_tf = entry.term_frequencies.get("parse");
        assert!(parse_tf.is_some());
        assert!(parse_tf.unwrap().symbols > 0);
    }

    #[test]
    fn index_avg_doc_length() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("small.rs"), "fn a() {}").unwrap();
        fs::write(
            dir.path().join("large.rs"),
            "fn b() {\n    let x = 1;\n    let y = 2;\n    let z = x + y;\n    println!(\"{}\", z);\n}\n",
        )
        .unwrap();

        let files = vec![
            make_file_info("small.rs", "fn a() {}"),
            make_file_info(
                "large.rs",
                "fn b() {\n    let x = 1;\n    let y = 2;\n    let z = x + y;\n    println!(\"{}\", z);\n}\n",
            ),
        ];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        assert!(index.avg_doc_length > 0.0);
        assert_eq!(index.total_docs, 2);
    }

    #[test]
    fn extract_rust_chunks() {
        let content = r#"
use std::collections::HashMap;

pub struct Config {
    pub name: String,
}

pub enum Status {
    Active,
    Inactive,
}

impl Config {
    pub fn new() -> Self {
        Self { name: String::new() }
    }
}

pub trait Handler {
    fn handle(&self);
}
"#;
        let chunker = RegexChunker;
        let chunks = chunker.chunk(content, Language::Rust);
        let kinds: Vec<ChunkKind> = chunks.iter().map(|c| c.kind).collect();

        assert!(kinds.contains(&ChunkKind::Import));
        assert!(kinds.contains(&ChunkKind::Type));
        assert!(kinds.contains(&ChunkKind::Impl));
        assert!(kinds.contains(&ChunkKind::Function));

        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Config" && c.kind == ChunkKind::Type)
        );
    }

    #[test]
    fn extract_python_chunks() {
        let content = r#"
class UserService:
    def authenticate(self, token):
        return True

async def fetch_data(url):
    pass
"#;
        let chunker = RegexChunker;
        let chunks = chunker.chunk(content, Language::Python);
        assert!(chunks.iter().any(|c| c.name == "UserService"));
        assert!(chunks.iter().any(|c| c.name == "authenticate"));
        assert!(chunks.iter().any(|c| c.name == "fetch_data"));
    }

    #[test]
    fn extract_go_chunks() {
        let content = r#"
package main

func main() {
    fmt.Println("hello")
}

type Config struct {
    Name string
}
"#;
        let chunker = RegexChunker;
        let chunks = chunker.chunk(content, Language::Go);
        assert!(chunks.iter().any(|c| c.name == "main"));
        assert!(chunks.iter().any(|c| c.name == "Config"));
    }

    #[test]
    fn tokenize_path_splits_correctly() {
        let tokens = tokenize_path("src/auth/middleware.rs");
        assert!(tokens.contains(&"src".to_string()));
        assert!(tokens.contains(&"auth".to_string()));
        assert!(tokens.contains(&"middleware".to_string()));
    }

    #[test]
    fn tokenize_content_handles_code() {
        let tokens = tokenize_content("fn authenticate(token: &str) -> bool {}");
        assert!(tokens.contains(&"authenticate".to_string()));
        assert!(tokens.contains(&"token".to_string()));
        assert!(tokens.contains(&"bool".to_string()));
    }
}
