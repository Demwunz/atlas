use atlas_core::{Chunk, ChunkKind, DeepIndex, FileEntry, TermFreqs};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Default index file location relative to repo root.
const INDEX_DIR: &str = ".atlas";
const INDEX_FILE: &str = "index.json";

/// Serializable representation of the deep index.
#[derive(Serialize, Deserialize)]
struct StoredIndex {
    version: u32,
    total_docs: u32,
    avg_doc_length: f64,
    doc_frequencies: HashMap<String, u32>,
    files: HashMap<String, StoredFileEntry>,
}

#[derive(Serialize, Deserialize)]
struct StoredFileEntry {
    sha256: Vec<u8>,
    doc_length: u32,
    term_frequencies: HashMap<String, StoredTermFreqs>,
    chunks: Vec<StoredChunk>,
}

#[derive(Serialize, Deserialize)]
struct StoredTermFreqs {
    filename: u32,
    symbols: u32,
    body: u32,
}

#[derive(Serialize, Deserialize)]
struct StoredChunk {
    kind: String,
    name: String,
    start_line: u32,
    end_line: u32,
    content: String,
}

/// Save a DeepIndex to disk.
pub fn save(index: &DeepIndex, repo_root: &Path) -> anyhow::Result<()> {
    let stored = to_stored(index);
    let dir = repo_root.join(INDEX_DIR);
    fs::create_dir_all(&dir)?;

    let path = dir.join(INDEX_FILE);
    let json = serde_json::to_vec(&stored)?;
    fs::write(&path, json)?;

    Ok(())
}

/// Load a DeepIndex from disk. Returns None if the index file doesn't exist.
pub fn load(repo_root: &Path) -> anyhow::Result<Option<DeepIndex>> {
    let path = repo_root.join(INDEX_DIR).join(INDEX_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let stored: StoredIndex = serde_json::from_slice(&bytes)?;
    Ok(Some(from_stored(stored)))
}

/// Get the path to the index file.
pub fn index_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(INDEX_DIR).join(INDEX_FILE)
}

fn to_stored(index: &DeepIndex) -> StoredIndex {
    let files = index
        .files
        .iter()
        .map(|(path, entry)| {
            let stored_entry = StoredFileEntry {
                sha256: entry.sha256.to_vec(),
                doc_length: entry.doc_length,
                term_frequencies: entry
                    .term_frequencies
                    .iter()
                    .map(|(term, tf)| {
                        (
                            term.clone(),
                            StoredTermFreqs {
                                filename: tf.filename,
                                symbols: tf.symbols,
                                body: tf.body,
                            },
                        )
                    })
                    .collect(),
                chunks: entry
                    .chunks
                    .iter()
                    .map(|c| StoredChunk {
                        kind: chunk_kind_to_str(c.kind),
                        name: c.name.clone(),
                        start_line: c.start_line,
                        end_line: c.end_line,
                        content: c.content.clone(),
                    })
                    .collect(),
            };
            (path.clone(), stored_entry)
        })
        .collect();

    StoredIndex {
        version: index.version,
        total_docs: index.total_docs,
        avg_doc_length: index.avg_doc_length,
        doc_frequencies: index.doc_frequencies.clone(),
        files,
    }
}

fn from_stored(stored: StoredIndex) -> DeepIndex {
    let files = stored
        .files
        .into_iter()
        .map(|(path, entry)| {
            let sha256: [u8; 32] = entry.sha256.try_into().unwrap_or([0u8; 32]);
            let file_entry = FileEntry {
                sha256,
                doc_length: entry.doc_length,
                term_frequencies: entry
                    .term_frequencies
                    .into_iter()
                    .map(|(term, tf)| {
                        (
                            term,
                            TermFreqs {
                                filename: tf.filename,
                                symbols: tf.symbols,
                                body: tf.body,
                            },
                        )
                    })
                    .collect(),
                chunks: entry
                    .chunks
                    .into_iter()
                    .map(|c| Chunk {
                        kind: str_to_chunk_kind(&c.kind),
                        name: c.name,
                        start_line: c.start_line,
                        end_line: c.end_line,
                        content: c.content,
                    })
                    .collect(),
            };
            (path, file_entry)
        })
        .collect();

    DeepIndex {
        version: stored.version,
        total_docs: stored.total_docs,
        avg_doc_length: stored.avg_doc_length,
        doc_frequencies: stored.doc_frequencies,
        files,
    }
}

fn chunk_kind_to_str(kind: ChunkKind) -> String {
    match kind {
        ChunkKind::Function => "function".to_string(),
        ChunkKind::Type => "type".to_string(),
        ChunkKind::Impl => "impl".to_string(),
        ChunkKind::Import => "import".to_string(),
        ChunkKind::Other => "other".to_string(),
    }
}

fn str_to_chunk_kind(s: &str) -> ChunkKind {
    match s {
        "function" => ChunkKind::Function,
        "type" => ChunkKind::Type,
        "impl" => ChunkKind::Impl,
        "import" => ChunkKind::Import,
        _ => ChunkKind::Other,
    }
}

/// Perform an incremental update: merge new index data with an existing index.
///
/// Files whose SHA-256 hasn't changed keep their existing entries.
/// New or changed files get entries from the fresh index.
pub fn merge_incremental(existing: &DeepIndex, fresh: &DeepIndex) -> DeepIndex {
    let mut merged_files = HashMap::new();

    // Start with all fresh entries
    for (path, entry) in &fresh.files {
        // Check if the file exists in the old index with the same hash
        if let Some(old_entry) = existing.files.get(path)
            && old_entry.sha256 == entry.sha256
        {
            // File unchanged — keep existing entry
            merged_files.insert(path.clone(), old_entry.clone());
            continue;
        }
        // File is new or changed — use fresh entry
        merged_files.insert(path.clone(), entry.clone());
    }

    // Recompute corpus stats from merged data
    let total_docs = merged_files.len() as u32;
    let total_length: u32 = merged_files.values().map(|e| e.doc_length).sum();
    let avg_doc_length = if total_docs > 0 {
        total_length as f64 / total_docs as f64
    } else {
        1.0
    };

    let mut doc_frequencies: HashMap<String, u32> = HashMap::new();
    for entry in merged_files.values() {
        for term in entry.term_frequencies.keys() {
            *doc_frequencies.entry(term.clone()).or_default() += 1;
        }
    }

    DeepIndex {
        version: fresh.version,
        files: merged_files,
        avg_doc_length,
        total_docs,
        doc_frequencies,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::IndexBuilder;
    use atlas_core::{FileInfo, Language};

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
    fn save_and_load_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let content = "fn main() {}\n";
        fs::write(dir.path().join("main.rs"), content).unwrap();

        let files = vec![make_file_info("main.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files).unwrap();

        save(&index, dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.version, index.version);
        assert_eq!(loaded.total_docs, index.total_docs);
        assert!(loaded.files.contains_key("main.rs"));
        assert_eq!(
            loaded.files["main.rs"].sha256,
            index.files["main.rs"].sha256
        );
    }

    #[test]
    fn load_nonexistent_returns_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = load(dir.path()).unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn save_creates_atlas_dir() {
        let dir = tempfile::tempdir().unwrap();
        let index = DeepIndex {
            version: 1,
            files: HashMap::new(),
            avg_doc_length: 0.0,
            total_docs: 0,
            doc_frequencies: HashMap::new(),
        };

        save(&index, dir.path()).unwrap();
        assert!(dir.path().join(".atlas").exists());
        assert!(dir.path().join(".atlas/index.json").exists());
    }

    #[test]
    fn merge_incremental_keeps_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let content_a = "fn a() {}\n";
        let content_b = "fn b() {}\n";
        fs::write(dir.path().join("a.rs"), content_a).unwrap();
        fs::write(dir.path().join("b.rs"), content_b).unwrap();

        let files = vec![
            make_file_info("a.rs", content_a),
            make_file_info("b.rs", content_b),
        ];
        let builder = IndexBuilder::new(dir.path());
        let existing = builder.build(&files).unwrap();

        // Build fresh index (same content)
        let fresh = builder.build(&files).unwrap();

        let merged = merge_incremental(&existing, &fresh);
        assert_eq!(merged.total_docs, 2);
    }

    #[test]
    fn merge_incremental_updates_changed() {
        let dir = tempfile::tempdir().unwrap();
        let content_a = "fn a() {}\n";
        fs::write(dir.path().join("a.rs"), content_a).unwrap();

        let files_v1 = vec![make_file_info("a.rs", content_a)];
        let builder = IndexBuilder::new(dir.path());
        let existing = builder.build(&files_v1).unwrap();

        // Change file content
        let content_a2 = "fn a_updated() {}\n";
        fs::write(dir.path().join("a.rs"), content_a2).unwrap();

        let files_v2 = vec![make_file_info("a.rs", content_a2)];
        let fresh = builder.build(&files_v2).unwrap();

        let merged = merge_incremental(&existing, &fresh);
        assert_eq!(merged.total_docs, 1);
        // SHA should be different (fresh content)
        assert_eq!(merged.files["a.rs"].sha256, fresh.files["a.rs"].sha256);
    }

    #[test]
    fn chunk_kind_roundtrip() {
        for kind in [
            ChunkKind::Function,
            ChunkKind::Type,
            ChunkKind::Impl,
            ChunkKind::Import,
            ChunkKind::Other,
        ] {
            let s = chunk_kind_to_str(kind);
            assert_eq!(str_to_chunk_kind(&s), kind);
        }
    }
}
