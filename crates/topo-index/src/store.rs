use std::collections::HashMap;
use std::fs;
use std::path::Path;
use topo_core::DeepIndex;

/// Default index file location relative to repo root.
const INDEX_DIR: &str = ".topo";
const INDEX_FILE: &str = "index.bin";

/// Save a DeepIndex to disk using rkyv binary serialization.
pub fn save(index: &DeepIndex, repo_root: &Path) -> anyhow::Result<()> {
    let dir = repo_root.join(INDEX_DIR);
    fs::create_dir_all(&dir)?;

    let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(index)
        .map_err(|e| anyhow::anyhow!("rkyv serialize: {e}"))?;
    fs::write(dir.join(INDEX_FILE), &bytes)?;

    // Remove legacy JSON index if present
    let legacy = dir.join("index.json");
    if legacy.exists() {
        let _ = fs::remove_file(legacy);
    }

    Ok(())
}

/// Load a DeepIndex from disk. Returns None if the index file doesn't exist.
pub fn load(repo_root: &Path) -> anyhow::Result<Option<DeepIndex>> {
    let path = repo_root.join(INDEX_DIR).join(INDEX_FILE);
    if !path.exists() {
        return Ok(None);
    }

    let bytes = fs::read(&path)?;
    let index = match rkyv::from_bytes::<DeepIndex, rkyv::rancor::Error>(&bytes) {
        Ok(idx) if idx.version >= 2 => idx,
        // Old version or deserialization failure — force rebuild
        _ => return Ok(None),
    };
    Ok(Some(index))
}

/// Get the path to the index file.
pub fn index_path(repo_root: &Path) -> std::path::PathBuf {
    repo_root.join(INDEX_DIR).join(INDEX_FILE)
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
        // PageRank is recomputed globally, always take from fresh index
        pagerank_scores: fresh.pagerank_scores.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::IndexBuilder;
    use topo_core::{ChunkKind, FileInfo, Language};

    fn make_file_info(path: &str, content: &str) -> FileInfo {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();

        FileInfo {
            path: path.to_string(),
            size: content.len() as u64,
            language: Language::from_path(Path::new(path)),
            role: topo_core::FileRole::from_path(Path::new(path)),
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
        let index = builder.build(&files, None).unwrap().0;

        save(&index, dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.version, 2);
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
    fn save_creates_topo_dir() {
        let dir = tempfile::tempdir().unwrap();
        let index = DeepIndex {
            version: 2,
            files: HashMap::new(),
            avg_doc_length: 0.0,
            total_docs: 0,
            doc_frequencies: HashMap::new(),
            pagerank_scores: HashMap::new(),
        };

        save(&index, dir.path()).unwrap();
        assert!(dir.path().join(".topo").exists());
        assert!(dir.path().join(".topo/index.bin").exists());
    }

    #[test]
    fn roundtrip_preserves_chunks() {
        let dir = tempfile::tempdir().unwrap();
        let content = "pub fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}\n";
        fs::write(dir.path().join("auth.rs"), content).unwrap();

        let files = vec![make_file_info("auth.rs", content)];
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        save(&index, dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();

        let entry = &loaded.files["auth.rs"];
        assert!(
            entry
                .chunks
                .iter()
                .any(|c| c.kind == ChunkKind::Function && c.name == "authenticate")
        );
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
        let existing = builder.build(&files, None).unwrap().0;

        // Build fresh index (same content)
        let fresh = builder.build(&files, None).unwrap().0;

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
        let existing = builder.build(&files_v1, None).unwrap().0;

        // Change file content
        let content_a2 = "fn a_updated() {}\n";
        fs::write(dir.path().join("a.rs"), content_a2).unwrap();

        let files_v2 = vec![make_file_info("a.rs", content_a2)];
        let fresh = builder.build(&files_v2, None).unwrap().0;

        let merged = merge_incremental(&existing, &fresh);
        assert_eq!(merged.total_docs, 1);
        // SHA should be different (fresh content)
        assert_eq!(merged.files["a.rs"].sha256, fresh.files["a.rs"].sha256);
    }

    #[test]
    fn removes_legacy_json_index() {
        let dir = tempfile::tempdir().unwrap();
        let topo_dir = dir.path().join(".topo");
        fs::create_dir_all(&topo_dir).unwrap();
        fs::write(topo_dir.join("index.json"), b"{}").unwrap();

        let index = DeepIndex {
            version: 2,
            files: HashMap::new(),
            avg_doc_length: 0.0,
            total_docs: 0,
            doc_frequencies: HashMap::new(),
            pagerank_scores: HashMap::new(),
        };

        save(&index, dir.path()).unwrap();
        assert!(!topo_dir.join("index.json").exists());
        assert!(topo_dir.join("index.bin").exists());
    }
}
