use crate::fingerprint;
use crate::scanner::Scanner;
use atlas_core::Bundle;
use std::path::Path;
use std::time::SystemTime;

/// Orchestrates scan -> hash -> fingerprint -> Bundle.
pub struct BundleBuilder<'a> {
    root: &'a Path,
}

impl<'a> BundleBuilder<'a> {
    pub fn new(root: &'a Path) -> Self {
        Self { root }
    }

    /// Build a complete Bundle from the repository root.
    pub fn build(&self) -> anyhow::Result<Bundle> {
        let scanner = Scanner::new(self.root);
        let files = scanner.scan()?;
        let fp = fingerprint::generate(&files);

        Ok(Bundle {
            fingerprint: fp,
            root: self.root.to_path_buf(),
            files,
            scanned_at: SystemTime::now(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn bundle_builder_creates_bundle() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();

        let bundle = BundleBuilder::new(dir.path()).build().unwrap();

        assert_eq!(bundle.file_count(), 2);
        assert!(!bundle.fingerprint.is_empty());
        assert_eq!(bundle.root, dir.path());
    }

    #[test]
    fn bundle_builder_fingerprint_is_deterministic() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let b1 = BundleBuilder::new(dir.path()).build().unwrap();
        let b2 = BundleBuilder::new(dir.path()).build().unwrap();

        assert_eq!(b1.fingerprint, b2.fingerprint);
    }

    #[test]
    fn bundle_builder_fingerprint_changes_on_new_file() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let b1 = BundleBuilder::new(dir.path()).build().unwrap();

        fs::write(dir.path().join("lib.rs"), "pub fn hello() {}").unwrap();

        let b2 = BundleBuilder::new(dir.path()).build().unwrap();

        assert_ne!(b1.fingerprint, b2.fingerprint);
    }

    #[test]
    fn bundle_builder_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let bundle = BundleBuilder::new(dir.path()).build().unwrap();

        assert!(bundle.is_empty());
        assert!(!bundle.fingerprint.is_empty());
    }

    #[test]
    fn bundle_builder_includes_hashes() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();

        let bundle = BundleBuilder::new(dir.path()).build().unwrap();
        let file = &bundle.files[0];
        assert_ne!(file.sha256, [0u8; 32]);
    }

    #[test]
    fn bundle_builder_token_count() {
        let dir = tempfile::tempdir().unwrap();
        // 400 bytes -> 100 tokens
        let content = "x".repeat(400);
        fs::write(dir.path().join("main.rs"), &content).unwrap();

        let bundle = BundleBuilder::new(dir.path()).build().unwrap();
        assert_eq!(bundle.total_tokens(), 100);
    }
}
