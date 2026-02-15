//! File walking with gitignore support and content hashing.

mod bundle;
pub(crate) mod fingerprint;
pub(crate) mod hash;
mod scanner;

pub use bundle::BundleBuilder;
pub use scanner::Scanner;

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn create_test_dir() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();

        // Create source files
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn hello() {}").unwrap();

        // Create a test file
        fs::create_dir_all(root.join("tests")).unwrap();
        fs::write(
            root.join("tests/integration.rs"),
            "#[test] fn it_works() {}",
        )
        .unwrap();

        // Create config
        fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"").unwrap();

        // Create docs
        fs::write(root.join("README.md"), "# Test").unwrap();

        // Use .ignore file (respected by the ignore crate without requiring git init)
        fs::write(root.join(".ignore"), "target/\n*.tmp\n").unwrap();

        // Create files that should be ignored
        fs::create_dir_all(root.join("target")).unwrap();
        fs::write(root.join("target/debug"), "binary").unwrap();
        fs::write(root.join("temp.tmp"), "temporary").unwrap();

        dir
    }

    #[test]
    fn scanner_finds_files() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        // Should find files but not those in .gitignore
        assert!(!files.is_empty());

        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"src/main.rs"));
        assert!(paths.contains(&"src/lib.rs"));
        assert!(paths.contains(&"README.md"));
    }

    #[test]
    fn scanner_respects_gitignore() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        // target/ and *.tmp should be excluded
        assert!(!paths.iter().any(|p| p.contains("target")));
        assert!(!paths.iter().any(|p| p.ends_with(".tmp")));
    }

    #[test]
    fn scanner_detects_languages() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let rs_file = files.iter().find(|f| f.path == "src/main.rs").unwrap();
        assert_eq!(rs_file.language, atlas_core::Language::Rust);

        let md_file = files.iter().find(|f| f.path == "README.md").unwrap();
        assert_eq!(md_file.language, atlas_core::Language::Markdown);
    }

    #[test]
    fn scanner_classifies_roles() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let main_rs = files.iter().find(|f| f.path == "src/main.rs").unwrap();
        assert_eq!(main_rs.role, atlas_core::FileRole::Implementation);

        let readme = files.iter().find(|f| f.path == "README.md").unwrap();
        assert_eq!(readme.role, atlas_core::FileRole::Documentation);

        let test_file = files
            .iter()
            .find(|f| f.path == "tests/integration.rs")
            .unwrap();
        assert_eq!(test_file.role, atlas_core::FileRole::Test);
    }

    #[test]
    fn scanner_computes_hashes() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let file = files.iter().find(|f| f.path == "src/main.rs").unwrap();
        // Hash should not be all zeros (it was computed)
        assert_ne!(file.sha256, [0u8; 32]);
    }

    #[test]
    fn scanner_records_file_sizes() {
        let dir = create_test_dir();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let file = files.iter().find(|f| f.path == "src/main.rs").unwrap();
        assert_eq!(file.size, "fn main() {}".len() as u64);
    }

    #[test]
    fn scanner_same_content_same_hash() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "same content").unwrap();
        fs::write(dir.path().join("b.rs"), "same content").unwrap();

        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let a = files.iter().find(|f| f.path == "a.rs").unwrap();
        let b = files.iter().find(|f| f.path == "b.rs").unwrap();
        assert_eq!(a.sha256, b.sha256);
    }

    #[test]
    fn scanner_different_content_different_hash() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.rs"), "content a").unwrap();
        fs::write(dir.path().join("b.rs"), "content b").unwrap();

        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();

        let a = files.iter().find(|f| f.path == "a.rs").unwrap();
        let b = files.iter().find(|f| f.path == "b.rs").unwrap();
        assert_ne!(a.sha256, b.sha256);
    }

    #[test]
    fn scanner_empty_directory() {
        let dir = tempfile::tempdir().unwrap();
        let scanner = Scanner::new(dir.path());
        let files = scanner.scan().unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn hash_sha256_deterministic() {
        let hash1 = hash::sha256_bytes(b"hello world");
        let hash2 = hash::sha256_bytes(b"hello world");
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn hash_sha256_different_input() {
        let hash1 = hash::sha256_bytes(b"hello");
        let hash2 = hash::sha256_bytes(b"world");
        assert_ne!(hash1, hash2);
    }

    #[test]
    fn scanner_nonexistent_path() {
        let scanner = Scanner::new(Path::new("/nonexistent/path/that/does/not/exist"));
        let files = scanner.scan().unwrap();
        assert!(files.is_empty());
    }
}
