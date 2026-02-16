use crate::hash;
use ignore::WalkBuilder;
use std::path::Path;
use topo_core::{FileInfo, FileRole, Language};

/// Walks a directory tree, respecting .gitignore rules, and produces `FileInfo` entries.
pub struct Scanner<'a> {
    root: &'a Path,
}

impl<'a> Scanner<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// Scan the directory tree and return metadata for all non-ignored files.
    pub fn scan(&self) -> anyhow::Result<Vec<FileInfo>> {
        let mut files = Vec::new();

        let walker = WalkBuilder::new(self.root)
            .hidden(false) // don't skip dotfiles by default
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .build();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            // Skip directories
            if entry.file_type().is_some_and(|ft| ft.is_dir()) {
                continue;
            }

            let path = entry.path();

            // Get relative path from root
            let rel_path = match path.strip_prefix(self.root) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Skip empty relative paths (the root itself)
            if rel_path.as_os_str().is_empty() {
                continue;
            }

            // Skip Topo's own data directory
            if rel_path.starts_with(".topo") {
                continue;
            }

            let rel_str = rel_path.to_string_lossy().to_string();

            // Get file metadata
            let metadata = match path.metadata() {
                Ok(m) => m,
                Err(_) => continue,
            };

            // Skip non-regular files
            if !metadata.is_file() {
                continue;
            }

            let size = metadata.len();
            let language = Language::from_path(rel_path);
            let role = FileRole::from_path(rel_path);

            let sha256 = match hash::sha256_file(path) {
                Ok(h) => h,
                Err(_) => continue,
            };

            files.push(FileInfo {
                path: rel_str,
                size,
                language,
                role,
                sha256,
            });
        }

        // Sort by path for deterministic output
        files.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(files)
    }
}
