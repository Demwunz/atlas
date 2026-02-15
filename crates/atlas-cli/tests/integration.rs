//! Integration test: scan a real directory, build a Bundle, render JSONL v0.3.

use atlas_core::{FileRole, Language, ScoredFile, SignalBreakdown};
use atlas_render::JsonlWriter;
use atlas_scanner::BundleBuilder;
use std::fs;

fn create_test_project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    fs::create_dir_all(root.join("src/auth")).unwrap();
    fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    println!(\"Hello, world!\");\n}\n",
    )
    .unwrap();
    fs::write(root.join("src/lib.rs"), "pub mod auth;\npub mod handler;\n").unwrap();
    fs::write(
        root.join("src/auth/mod.rs"),
        "pub fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}\n",
    )
    .unwrap();

    fs::create_dir_all(root.join("tests")).unwrap();
    fs::write(
        root.join("tests/auth_test.rs"),
        "#[test]\nfn test_auth() {\n    assert!(true);\n}\n",
    )
    .unwrap();

    fs::write(root.join("Cargo.toml"), "[package]\nname = \"demo\"").unwrap();
    fs::write(root.join("README.md"), "# Demo Project").unwrap();

    dir
}

#[test]
fn scan_and_bundle() {
    let dir = create_test_project();
    let bundle = BundleBuilder::new(dir.path()).build().unwrap();

    assert!(bundle.file_count() >= 5);
    assert!(!bundle.fingerprint.is_empty());
    assert_eq!(bundle.fingerprint.len(), 64);

    let main_rs = bundle.files.iter().find(|f| f.path.ends_with("main.rs"));
    assert!(main_rs.is_some());
    assert_eq!(main_rs.unwrap().language, Language::Rust);

    let readme = bundle.files.iter().find(|f| f.path == "README.md");
    assert!(readme.is_some());
    assert_eq!(readme.unwrap().role, FileRole::Documentation);

    let test_file = bundle.files.iter().find(|f| f.path.contains("auth_test"));
    assert!(test_file.is_some());
    assert_eq!(test_file.unwrap().role, FileRole::Test);
}

#[test]
fn bundle_to_jsonl_roundtrip() {
    let dir = create_test_project();
    let bundle = BundleBuilder::new(dir.path()).build().unwrap();

    let scored: Vec<ScoredFile> = bundle
        .files
        .iter()
        .map(|f| ScoredFile {
            path: f.path.clone(),
            score: 0.5,
            signals: SignalBreakdown::default(),
            tokens: f.estimated_tokens(),
            language: f.language,
            role: f.role,
        })
        .collect();

    let output = JsonlWriter::new("auth middleware", "balanced")
        .max_bytes(Some(100_000))
        .min_score(0.01)
        .render(&scored, bundle.file_count())
        .unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    assert_eq!(lines.len(), scored.len() + 2);

    // Every line is valid JSON
    for line in &lines {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Invalid JSON: {line}");
    }

    // Header
    let header: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(header["Version"], "0.3");
    assert_eq!(header["Query"], "auth middleware");
    assert_eq!(header["Preset"], "balanced");

    // Footer
    let footer: serde_json::Value = serde_json::from_str(lines[lines.len() - 1]).unwrap();
    assert_eq!(footer["TotalFiles"], scored.len());
    assert_eq!(footer["ScannedFiles"], bundle.file_count());
}

#[test]
fn incremental_fingerprint_unchanged() {
    let dir = create_test_project();
    let b1 = BundleBuilder::new(dir.path()).build().unwrap();
    let b2 = BundleBuilder::new(dir.path()).build().unwrap();
    assert_eq!(b1.fingerprint, b2.fingerprint);
}

#[test]
fn incremental_hash_changes_on_edit() {
    let dir = create_test_project();
    let b1 = BundleBuilder::new(dir.path()).build().unwrap();

    fs::write(
        dir.path().join("src/main.rs"),
        "fn main() { println!(\"changed!\"); }",
    )
    .unwrap();

    let b2 = BundleBuilder::new(dir.path()).build().unwrap();

    let main1 = b1
        .files
        .iter()
        .find(|f| f.path.contains("main.rs"))
        .unwrap();
    let main2 = b2
        .files
        .iter()
        .find(|f| f.path.contains("main.rs"))
        .unwrap();
    assert_ne!(main1.sha256, main2.sha256);
}
