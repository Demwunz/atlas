//! Topo core domain types, traits, and errors.

mod error;
mod types;

pub use error::TopoError;
pub use types::{
    Bundle, Chunk, ChunkKind, DeepIndex, FileEntry, FileInfo, FileRole, Language, ScoredFile,
    SignalBreakdown, TermFreqs, TokenBudget,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    // --- Language::from_extension ---

    #[test]
    fn language_from_extension() {
        assert_eq!(Language::from_extension("rs"), Language::Rust);
        assert_eq!(Language::from_extension("py"), Language::Python);
        assert_eq!(Language::from_extension("js"), Language::JavaScript);
        assert_eq!(Language::from_extension("xyz"), Language::Other);
    }

    #[test]
    fn language_from_extension_cpp_variants() {
        assert_eq!(Language::from_extension("cpp"), Language::Cpp);
        assert_eq!(Language::from_extension("cc"), Language::Cpp);
        assert_eq!(Language::from_extension("hpp"), Language::Cpp);
    }

    #[test]
    fn language_from_extension_typescript_variants() {
        assert_eq!(Language::from_extension("ts"), Language::TypeScript);
        assert_eq!(Language::from_extension("tsx"), Language::TypeScript);
        assert_eq!(Language::from_extension("mts"), Language::TypeScript);
    }

    // --- Language::from_path ---

    #[test]
    fn language_from_path_rust() {
        assert_eq!(
            Language::from_path(Path::new("src/main.rs")),
            Language::Rust
        );
    }

    #[test]
    fn language_from_path_nested_typescript() {
        assert_eq!(
            Language::from_path(Path::new("src/components/App.tsx")),
            Language::TypeScript
        );
    }

    #[test]
    fn language_from_path_no_extension() {
        assert_eq!(Language::from_path(Path::new("Makefile")), Language::Other);
    }

    #[test]
    fn language_from_path_cpp_header() {
        assert_eq!(
            Language::from_path(Path::new("include/foo.hpp")),
            Language::Cpp
        );
    }

    // --- Language::Display ---

    #[test]
    fn language_display() {
        assert_eq!(format!("{}", Language::Rust), "rust");
        assert_eq!(format!("{}", Language::TypeScript), "typescript");
        assert_eq!(format!("{}", Language::Other), "other");
    }

    // --- Language::is_programming_language ---

    #[test]
    fn language_is_programming_language() {
        assert!(Language::Rust.is_programming_language());
        assert!(Language::Python.is_programming_language());
        assert!(!Language::Markdown.is_programming_language());
        assert!(!Language::Json.is_programming_language());
        assert!(!Language::Other.is_programming_language());
    }

    // --- FileRole::as_str ---

    #[test]
    fn file_role_as_str() {
        assert_eq!(FileRole::Implementation.as_str(), "impl");
        assert_eq!(FileRole::Test.as_str(), "test");
        assert_eq!(FileRole::Config.as_str(), "config");
        assert_eq!(FileRole::Documentation.as_str(), "docs");
        assert_eq!(FileRole::Generated.as_str(), "generated");
        assert_eq!(FileRole::Build.as_str(), "build");
        assert_eq!(FileRole::Other.as_str(), "other");
    }

    // --- FileRole::Display ---

    #[test]
    fn file_role_display() {
        assert_eq!(format!("{}", FileRole::Implementation), "impl");
        assert_eq!(format!("{}", FileRole::Test), "test");
        assert_eq!(format!("{}", FileRole::Generated), "generated");
        assert_eq!(format!("{}", FileRole::Build), "build");
        assert_eq!(format!("{}", FileRole::Documentation), "docs");
    }

    // --- FileRole::from_path: Test files ---

    #[test]
    fn role_test_by_suffix_go() {
        assert_eq!(
            FileRole::from_path(Path::new("pkg/handler_test.go")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_by_suffix_rs() {
        assert_eq!(
            FileRole::from_path(Path::new("src/parser_test.rs")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_by_spec_suffix() {
        assert_eq!(
            FileRole::from_path(Path::new("src/parser_spec.rs")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_by_prefix_py() {
        assert_eq!(
            FileRole::from_path(Path::new("test_utils.py")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_by_directory() {
        assert_eq!(
            FileRole::from_path(Path::new("tests/integration/scan.rs")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_by_jest_directory() {
        assert_eq!(
            FileRole::from_path(Path::new("src/__tests__/App.test.js")),
            FileRole::Test
        );
    }

    #[test]
    fn role_test_spec_ts() {
        assert_eq!(
            FileRole::from_path(Path::new("src/utils.spec.ts")),
            FileRole::Test
        );
    }

    // --- FileRole::from_path: Config files ---

    #[test]
    fn role_config_yaml() {
        assert_eq!(
            FileRole::from_path(Path::new("config/settings.yaml")),
            FileRole::Config
        );
    }

    #[test]
    fn role_config_dotenv() {
        assert_eq!(
            FileRole::from_path(Path::new(".env.production")),
            FileRole::Config
        );
    }

    #[test]
    fn role_config_gitignore() {
        assert_eq!(
            FileRole::from_path(Path::new(".gitignore")),
            FileRole::Config
        );
    }

    // --- FileRole::from_path: Documentation ---

    #[test]
    fn role_documentation_md() {
        assert_eq!(
            FileRole::from_path(Path::new("README.md")),
            FileRole::Documentation
        );
    }

    #[test]
    fn role_documentation_in_docs_dir() {
        assert_eq!(
            FileRole::from_path(Path::new("docs/architecture.rs")),
            FileRole::Documentation
        );
    }

    // --- FileRole::from_path: Generated ---

    #[test]
    fn role_generated_vendor() {
        assert_eq!(
            FileRole::from_path(Path::new("vendor/github.com/pkg/errors/errors.go")),
            FileRole::Generated
        );
    }

    #[test]
    fn role_generated_node_modules() {
        assert_eq!(
            FileRole::from_path(Path::new("node_modules/lodash/index.js")),
            FileRole::Generated
        );
    }

    #[test]
    fn role_generated_pb_go() {
        assert_eq!(
            FileRole::from_path(Path::new("api/service.pb.go")),
            FileRole::Generated
        );
    }

    #[test]
    fn role_generated_filename_pattern() {
        assert_eq!(
            FileRole::from_path(Path::new("src/schema.generated.ts")),
            FileRole::Generated
        );
    }

    #[test]
    fn role_generated_takes_priority_over_test() {
        // A test file inside vendor/ should be Generated, not Test
        assert_eq!(
            FileRole::from_path(Path::new("vendor/pkg/handler_test.go")),
            FileRole::Generated
        );
    }

    // --- FileRole::from_path: Build files ---

    #[test]
    fn role_build_makefile() {
        assert_eq!(FileRole::from_path(Path::new("Makefile")), FileRole::Build);
    }

    #[test]
    fn role_build_dockerfile() {
        assert_eq!(
            FileRole::from_path(Path::new("Dockerfile")),
            FileRole::Build
        );
    }

    #[test]
    fn role_build_cargo_toml() {
        assert_eq!(
            FileRole::from_path(Path::new("Cargo.toml")),
            FileRole::Build
        );
    }

    // --- FileRole::from_path: Implementation ---

    #[test]
    fn role_implementation_rust() {
        assert_eq!(
            FileRole::from_path(Path::new("src/main.rs")),
            FileRole::Implementation
        );
    }

    #[test]
    fn role_implementation_html() {
        assert_eq!(
            FileRole::from_path(Path::new("templates/index.html")),
            FileRole::Implementation
        );
    }

    // --- FileRole::from_path: Other ---

    #[test]
    fn role_other_unknown_ext() {
        assert_eq!(
            FileRole::from_path(Path::new("data/blob.xyz")),
            FileRole::Other
        );
    }

    // --- FileInfo ---

    #[test]
    fn file_info_token_estimate() {
        let info = FileInfo {
            path: "src/main.rs".to_string(),
            size: 400,
            language: Language::Rust,
            role: FileRole::Implementation,
            sha256: [0u8; 32],
        };
        assert_eq!(info.estimated_tokens(), 100);
    }

    // --- Bundle ---

    #[test]
    fn bundle_is_empty_when_no_files() {
        let bundle = Bundle {
            fingerprint: "test".to_string(),
            root: std::path::PathBuf::from("/tmp"),
            files: vec![],
            scanned_at: std::time::SystemTime::now(),
        };
        assert!(bundle.is_empty());
        assert_eq!(bundle.total_tokens(), 0);
        assert_eq!(bundle.file_count(), 0);
    }

    #[test]
    fn bundle_with_files() {
        let bundle = Bundle {
            fingerprint: "test".to_string(),
            root: std::path::PathBuf::from("/tmp"),
            files: vec![
                FileInfo {
                    path: "a.rs".to_string(),
                    size: 400,
                    language: Language::Rust,
                    role: FileRole::Implementation,
                    sha256: [0u8; 32],
                },
                FileInfo {
                    path: "b.rs".to_string(),
                    size: 800,
                    language: Language::Rust,
                    role: FileRole::Implementation,
                    sha256: [0u8; 32],
                },
            ],
            scanned_at: std::time::SystemTime::now(),
        };
        assert!(!bundle.is_empty());
        assert_eq!(bundle.file_count(), 2);
        assert_eq!(bundle.total_tokens(), 300); // 100 + 200
    }

    // --- ScoredFile ---

    #[test]
    fn scored_file_ordering() {
        let a = ScoredFile {
            path: "a.rs".to_string(),
            score: 0.8,
            signals: SignalBreakdown::default(),
            tokens: 100,
            language: Language::Rust,
            role: FileRole::Implementation,
        };
        let b = ScoredFile {
            path: "b.rs".to_string(),
            score: 0.5,
            signals: SignalBreakdown::default(),
            tokens: 200,
            language: Language::Rust,
            role: FileRole::Implementation,
        };
        assert!(a.score > b.score);
    }

    // --- TopoError ---

    #[test]
    fn topo_error_display() {
        let err = TopoError::Io("test".to_string());
        assert!(err.to_string().contains("test"));
    }

    #[test]
    fn topo_error_from_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "not found");
        let topo_err: TopoError = io_err.into();
        assert!(topo_err.to_string().contains("not found"));
    }

    // --- ChunkKind ---

    #[test]
    fn chunk_kind_variants() {
        let kind = ChunkKind::Function;
        assert_eq!(format!("{kind:?}"), "Function");
    }

    // --- TokenBudget ---

    fn make_scored(path: &str, tokens: u64, score: f64) -> ScoredFile {
        ScoredFile {
            path: path.to_string(),
            score,
            signals: SignalBreakdown::default(),
            tokens,
            language: Language::Rust,
            role: FileRole::Implementation,
        }
    }

    #[test]
    fn budget_no_limits_returns_all() {
        let files = vec![make_scored("a.rs", 100, 0.9), make_scored("b.rs", 200, 0.8)];
        let budget = TokenBudget {
            max_bytes: None,
            max_tokens: None,
        };
        assert_eq!(budget.enforce(&files).len(), 2);
    }

    #[test]
    fn budget_max_bytes_truncates() {
        let files = vec![
            make_scored("a.rs", 100, 0.9), // 400 bytes
            make_scored("b.rs", 200, 0.8), // 800 bytes — cumulative 1200
            make_scored("c.rs", 300, 0.7), // 1200 bytes — cumulative 2400
        ];
        let budget = TokenBudget {
            max_bytes: Some(1000),
            max_tokens: None,
        };
        let result = budget.enforce(&files);
        // First file: 400 bytes (under 1000) ✓
        // Second file: cumulative 1200 (over 1000) — stop
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn budget_max_tokens_truncates() {
        let files = vec![
            make_scored("a.rs", 100, 0.9),
            make_scored("b.rs", 200, 0.8),
            make_scored("c.rs", 300, 0.7),
        ];
        let budget = TokenBudget {
            max_bytes: None,
            max_tokens: Some(250),
        };
        let result = budget.enforce(&files);
        // First: 100 tokens ✓, second: cumulative 300 > 250 — stop
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn budget_always_includes_first_file() {
        let files = vec![make_scored("huge.rs", 10000, 0.9)];
        let budget = TokenBudget {
            max_bytes: Some(100),
            max_tokens: None,
        };
        // First file always included even if it exceeds the budget
        assert_eq!(budget.enforce(&files).len(), 1);
    }

    #[test]
    fn budget_empty_input() {
        let budget = TokenBudget {
            max_bytes: Some(100),
            max_tokens: Some(100),
        };
        assert!(budget.enforce(&[]).is_empty());
    }
}
