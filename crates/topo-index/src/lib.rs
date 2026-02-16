//! Deep index with serialization and incremental updates.

mod builder;
mod store;

pub use builder::IndexBuilder;
pub use store::{index_path, load, merge_incremental, save};

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;
    use topo_core::{FileInfo, Language};

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
    fn full_index_pipeline() {
        let dir = tempfile::tempdir().unwrap();

        // Create source files
        fs::create_dir_all(dir.path().join("src")).unwrap();
        fs::write(
            dir.path().join("src/main.rs"),
            "use crate::auth;\n\nfn main() {\n    auth::check();\n}\n",
        )
        .unwrap();
        fs::write(
            dir.path().join("src/auth.rs"),
            "pub fn check() -> bool {\n    true\n}\n\npub struct Token {\n    pub value: String,\n}\n",
        )
        .unwrap();

        let files = vec![
            make_file_info(
                "src/main.rs",
                "use crate::auth;\n\nfn main() {\n    auth::check();\n}\n",
            ),
            make_file_info(
                "src/auth.rs",
                "pub fn check() -> bool {\n    true\n}\n\npub struct Token {\n    pub value: String,\n}\n",
            ),
        ];

        // Build index
        let builder = IndexBuilder::new(dir.path());
        let index = builder.build(&files, None).unwrap().0;

        assert_eq!(index.total_docs, 2);
        assert!(index.avg_doc_length > 0.0);

        // Save and reload
        save(&index, dir.path()).unwrap();
        let loaded = load(dir.path()).unwrap().unwrap();

        assert_eq!(loaded.total_docs, index.total_docs);
        assert_eq!(loaded.files.len(), index.files.len());

        // Verify chunks
        let auth_entry = &loaded.files["src/auth.rs"];
        let fn_names: Vec<&str> = auth_entry
            .chunks
            .iter()
            .filter(|c| c.kind == topo_core::ChunkKind::Function)
            .map(|c| c.name.as_str())
            .collect();
        assert!(fn_names.contains(&"check"));

        let type_names: Vec<&str> = auth_entry
            .chunks
            .iter()
            .filter(|c| c.kind == topo_core::ChunkKind::Type)
            .map(|c| c.name.as_str())
            .collect();
        assert!(type_names.contains(&"Token"));
    }

    #[test]
    fn incremental_update_pipeline() {
        let dir = tempfile::tempdir().unwrap();

        // Initial version
        fs::write(dir.path().join("a.rs"), "fn original() {}\n").unwrap();
        let files_v1 = vec![make_file_info("a.rs", "fn original() {}\n")];
        let builder = IndexBuilder::new(dir.path());
        let index_v1 = builder.build(&files_v1, None).unwrap().0;
        save(&index_v1, dir.path()).unwrap();

        // Update file
        fs::write(dir.path().join("a.rs"), "fn updated() {}\n").unwrap();
        let files_v2 = vec![make_file_info("a.rs", "fn updated() {}\n")];
        let index_v2 = builder.build(&files_v2, None).unwrap().0;

        // Load existing and merge
        let existing = load(dir.path()).unwrap().unwrap();
        let merged = merge_incremental(&existing, &index_v2);

        // SHA should be from fresh version (file changed)
        assert_eq!(merged.files["a.rs"].sha256, index_v2.files["a.rs"].sha256);
        assert_ne!(merged.files["a.rs"].sha256, index_v1.files["a.rs"].sha256);
    }
}
