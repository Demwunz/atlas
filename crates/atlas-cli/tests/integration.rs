//! Integration tests: scan, bundle, render JSONL v0.3, compatibility checks.

use atlas_core::{FileRole, Language, ScoredFile, SignalBreakdown, TokenBudget};
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

// ── Compatibility tests: JSONL v0.3 format matches spec ────────────

fn make_scored(path: &str, score: f64, tokens: u64, lang: Language, role: FileRole) -> ScoredFile {
    ScoredFile {
        path: path.to_string(),
        score,
        signals: SignalBreakdown {
            bm25f: score * 0.6,
            heuristic: score * 0.4,
            ..Default::default()
        },
        tokens,
        language: lang,
        role,
    }
}

#[test]
fn compat_jsonl_header_format() {
    let output = JsonlWriter::new("auth middleware", "balanced")
        .max_bytes(Some(100_000))
        .min_score(0.01)
        .render(&[], 42)
        .unwrap();

    let header: serde_json::Value = serde_json::from_str(output.lines().next().unwrap()).unwrap();

    // Required header fields per JSONL v0.3 spec
    assert_eq!(header["Version"], "0.3");
    assert!(header["Query"].is_string());
    assert!(header["Preset"].is_string());
    assert!(header["Budget"].is_object());
    assert!(header["MinScore"].is_number());
}

#[test]
fn compat_jsonl_file_entry_format() {
    let files = vec![make_scored(
        "src/auth.rs",
        0.95,
        300,
        Language::Rust,
        FileRole::Implementation,
    )];

    let output = JsonlWriter::new("auth", "balanced")
        .render(&files, 100)
        .unwrap();

    let lines: Vec<&str> = output.trim().lines().collect();
    let entry: serde_json::Value = serde_json::from_str(lines[1]).unwrap();

    // Required fields per spec
    assert!(entry["Path"].is_string());
    assert!(entry["Score"].is_number());
    assert!(entry["Tokens"].is_number());
    assert!(entry["Language"].is_string());
    assert!(entry["Role"].is_string());

    // Values are correct
    assert_eq!(entry["Path"], "src/auth.rs");
    assert_eq!(entry["Tokens"], 300);
    assert_eq!(entry["Language"], "rust");
    assert_eq!(entry["Role"], "impl");
}

#[test]
fn compat_jsonl_footer_format() {
    let files = vec![
        make_scored("a.rs", 0.9, 100, Language::Rust, FileRole::Implementation),
        make_scored("b.rs", 0.8, 200, Language::Rust, FileRole::Implementation),
    ];

    let output = JsonlWriter::new("test", "balanced")
        .render(&files, 500)
        .unwrap();

    let footer: serde_json::Value =
        serde_json::from_str(output.trim().lines().last().unwrap()).unwrap();

    assert_eq!(footer["TotalFiles"], 2);
    assert_eq!(footer["TotalTokens"], 300);
    assert_eq!(footer["ScannedFiles"], 500);
}

#[test]
fn compat_each_jsonl_line_is_valid_json() {
    let files = vec![
        make_scored("a.rs", 0.9, 100, Language::Rust, FileRole::Implementation),
        make_scored("b.py", 0.7, 200, Language::Python, FileRole::Implementation),
        make_scored("c.go", 0.5, 150, Language::Go, FileRole::Test),
    ];

    let output = JsonlWriter::new("mixed query", "deep")
        .max_bytes(Some(50_000))
        .min_score(0.1)
        .render(&files, 1000)
        .unwrap();

    for (i, line) in output.trim().lines().enumerate() {
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(line);
        assert!(parsed.is_ok(), "Line {i} is not valid JSON: {line}");
    }
}

#[test]
fn compat_jsonl_line_count() {
    let files = vec![
        make_scored("a.rs", 0.9, 100, Language::Rust, FileRole::Implementation),
        make_scored("b.rs", 0.8, 200, Language::Rust, FileRole::Implementation),
        make_scored("c.rs", 0.7, 150, Language::Rust, FileRole::Implementation),
    ];

    let output = JsonlWriter::new("test", "fast").render(&files, 50).unwrap();

    let line_count = output.trim().lines().count();
    // header + N files + footer = N + 2
    assert_eq!(line_count, 5);
}

// ── Token budget integration tests ─────────────────────────────────

#[test]
fn budget_enforcement_end_to_end() {
    let dir = create_test_project();
    let bundle = BundleBuilder::new(dir.path()).build().unwrap();

    let scored: Vec<ScoredFile> = bundle
        .files
        .iter()
        .enumerate()
        .map(|(i, f)| ScoredFile {
            path: f.path.clone(),
            score: 1.0 - (i as f64 * 0.1),
            signals: SignalBreakdown::default(),
            tokens: f.estimated_tokens(),
            language: f.language,
            role: f.role,
        })
        .collect();

    // Very small budget should still include at least one file
    let budget = TokenBudget {
        max_bytes: Some(1),
        max_tokens: None,
    };
    let result = budget.enforce(&scored);
    assert_eq!(result.len(), 1);

    // Large budget should include all
    let budget = TokenBudget {
        max_bytes: Some(1_000_000),
        max_tokens: None,
    };
    let result = budget.enforce(&scored);
    assert_eq!(result.len(), scored.len());
}

#[test]
fn budget_max_tokens_integration() {
    let files = vec![
        make_scored("a.rs", 0.9, 100, Language::Rust, FileRole::Implementation),
        make_scored("b.rs", 0.8, 200, Language::Rust, FileRole::Implementation),
        make_scored("c.rs", 0.7, 300, Language::Rust, FileRole::Implementation),
    ];

    let budget = TokenBudget {
        max_bytes: None,
        max_tokens: Some(250),
    };
    let result = budget.enforce(&files);
    // a.rs: 100 tokens, b.rs: cumulative 300 > 250 → only a.rs
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].path, "a.rs");
}

// ── Score pipeline integration ─────────────────────────────────────

#[test]
fn score_pipeline_end_to_end() {
    let dir = create_test_project();
    let bundle = BundleBuilder::new(dir.path()).build().unwrap();

    let scorer = atlas_score::HybridScorer::new("authenticate");
    let scored = scorer.score(&bundle.files);

    // Should return results sorted by score
    assert!(!scored.is_empty());
    for window in scored.windows(2) {
        assert!(window[0].score >= window[1].score);
    }

    // The auth module should rank highly
    let top5: Vec<&str> = scored.iter().take(5).map(|f| f.path.as_str()).collect();
    assert!(
        top5.iter().any(|p| p.contains("auth")),
        "auth file should be in top 5 for 'authenticate' query, got: {top5:?}"
    );
}
