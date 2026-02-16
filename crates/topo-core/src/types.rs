use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Metadata for a single scanned file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub size: u64,
    pub language: Language,
    pub role: FileRole,
    pub sha256: [u8; 32],
}

impl FileInfo {
    /// Estimate token count as bytes / 4 (rough heuristic).
    pub fn estimated_tokens(&self) -> u64 {
        self.size / 4
    }
}

/// Detected programming language.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Rust,
    Go,
    Python,
    JavaScript,
    TypeScript,
    Java,
    Ruby,
    C,
    Cpp,
    Shell,
    Markdown,
    Yaml,
    Toml,
    Json,
    Html,
    Css,
    Swift,
    Kotlin,
    Scala,
    Haskell,
    Elixir,
    Lua,
    Php,
    R,
    Other,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext {
            "rs" => Self::Rust,
            "go" => Self::Go,
            "py" | "pyi" => Self::Python,
            "js" | "mjs" | "cjs" => Self::JavaScript,
            "ts" | "tsx" | "mts" | "cts" => Self::TypeScript,
            "java" => Self::Java,
            "rb" => Self::Ruby,
            "c" | "h" => Self::C,
            "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => Self::Cpp,
            "sh" | "bash" | "zsh" => Self::Shell,
            "md" | "mdx" => Self::Markdown,
            "yml" | "yaml" => Self::Yaml,
            "toml" => Self::Toml,
            "json" => Self::Json,
            "html" | "htm" => Self::Html,
            "css" | "scss" | "sass" | "less" => Self::Css,
            "swift" => Self::Swift,
            "kt" | "kts" => Self::Kotlin,
            "scala" | "sc" => Self::Scala,
            "hs" => Self::Haskell,
            "ex" | "exs" => Self::Elixir,
            "lua" => Self::Lua,
            "php" => Self::Php,
            "r" | "R" => Self::R,
            _ => Self::Other,
        }
    }

    /// Detect language from a file path by extracting its extension.
    pub fn from_path(path: &Path) -> Self {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(Self::from_extension)
            .unwrap_or(Self::Other)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::Go => "go",
            Self::Python => "python",
            Self::JavaScript => "javascript",
            Self::TypeScript => "typescript",
            Self::Java => "java",
            Self::Ruby => "ruby",
            Self::C => "c",
            Self::Cpp => "cpp",
            Self::Shell => "shell",
            Self::Markdown => "markdown",
            Self::Yaml => "yaml",
            Self::Toml => "toml",
            Self::Json => "json",
            Self::Html => "html",
            Self::Css => "css",
            Self::Swift => "swift",
            Self::Kotlin => "kotlin",
            Self::Scala => "scala",
            Self::Haskell => "haskell",
            Self::Elixir => "elixir",
            Self::Lua => "lua",
            Self::Php => "php",
            Self::R => "r",
            Self::Other => "other",
        }
    }

    /// Returns true if this language is a programming language
    /// (as opposed to markup/config/data format).
    pub fn is_programming_language(&self) -> bool {
        matches!(
            self,
            Self::Rust
                | Self::Go
                | Self::Python
                | Self::JavaScript
                | Self::TypeScript
                | Self::Java
                | Self::Ruby
                | Self::C
                | Self::Cpp
                | Self::Shell
                | Self::Swift
                | Self::Kotlin
                | Self::Scala
                | Self::Haskell
                | Self::Elixir
                | Self::Lua
                | Self::Php
                | Self::R
        )
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Classification of a file's role in the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FileRole {
    Implementation,
    Test,
    Config,
    Documentation,
    Generated,
    Build,
    Other,
}

impl FileRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Implementation => "impl",
            Self::Test => "test",
            Self::Config => "config",
            Self::Documentation => "docs",
            Self::Generated => "generated",
            Self::Build => "build",
            Self::Other => "other",
        }
    }

    /// Classify a file's role based on its path.
    ///
    /// Priority order: Generated > Test > Documentation > Build > Config > Implementation > Other
    pub fn from_path(path: &Path) -> Self {
        let path_str = path.to_string_lossy();
        let file_name = path
            .file_name()
            .map(|f| f.to_string_lossy().into_owned())
            .unwrap_or_default();
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Generated directories (highest priority)
        if Self::path_contains_component(&path_str, "vendor")
            || Self::path_contains_component(&path_str, "node_modules")
            || Self::path_contains_component(&path_str, "generated")
        {
            return Self::Generated;
        }

        // Generated filename patterns
        if Self::is_generated_filename(&file_name) {
            return Self::Generated;
        }

        // Test directories
        if Self::path_contains_component(&path_str, "tests")
            || Self::path_contains_component(&path_str, "__tests__")
            || Self::path_contains_component(&path_str, "spec")
        {
            return Self::Test;
        }

        // Test filename patterns
        if Self::is_test_filename(&file_name) {
            return Self::Test;
        }

        // Documentation directory
        if Self::path_contains_component(&path_str, "docs") {
            return Self::Documentation;
        }

        // Build files (exact filenames)
        if Self::is_build_filename(&file_name) {
            return Self::Build;
        }

        // Config files
        if Self::is_config_extension(ext) || Self::is_config_filename(&file_name) {
            return Self::Config;
        }

        // Documentation by extension
        if matches!(ext, "md" | "mdx" | "rst") {
            return Self::Documentation;
        }

        // Implementation: known programming languages
        let lang = Language::from_extension(ext);
        if lang.is_programming_language() || matches!(lang, Language::Html | Language::Css) {
            return Self::Implementation;
        }

        Self::Other
    }

    fn path_contains_component(path_str: &str, component: &str) -> bool {
        path_str.split(['/', '\\']).any(|c| c == component)
    }

    fn is_test_filename(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.ends_with("_test.go")
            || lower.ends_with("_test.rs")
            || lower.ends_with("_spec.rs")
            || lower.ends_with("_spec.rb")
            || lower.ends_with("_test.py")
            || lower.ends_with(".test.js")
            || lower.ends_with(".test.ts")
            || lower.ends_with(".test.tsx")
            || lower.ends_with(".test.jsx")
            || lower.ends_with(".spec.js")
            || lower.ends_with(".spec.ts")
            || lower.ends_with(".spec.tsx")
            || lower.ends_with(".spec.jsx")
            || (lower.starts_with("test_") && (lower.ends_with(".py") || lower.ends_with(".rb")))
    }

    fn is_generated_filename(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.contains(".generated.") || lower.ends_with(".pb.go") || lower.ends_with(".g.dart")
    }

    fn is_build_filename(file_name: &str) -> bool {
        matches!(
            file_name,
            "Makefile"
                | "makefile"
                | "GNUmakefile"
                | "Cargo.toml"
                | "package.json"
                | "build.rs"
                | "build.gradle"
                | "build.gradle.kts"
                | "pom.xml"
                | "CMakeLists.txt"
                | "Dockerfile"
                | "docker-compose.yml"
                | "docker-compose.yaml"
                | "Rakefile"
                | "Gemfile"
                | "Justfile"
                | "justfile"
                | "go.mod"
                | "go.sum"
                | "setup.py"
                | "setup.cfg"
                | "pyproject.toml"
                | "Pipfile"
                | "Cargo.lock"
                | "package-lock.json"
                | "yarn.lock"
                | "pnpm-lock.yaml"
                | "flake.nix"
        )
    }

    fn is_config_extension(ext: &str) -> bool {
        matches!(
            ext,
            "yaml" | "yml" | "toml" | "json" | "ini" | "cfg" | "env"
        )
    }

    fn is_config_filename(file_name: &str) -> bool {
        let lower = file_name.to_lowercase();
        lower.starts_with(".env")
            || matches!(
                file_name,
                ".gitignore"
                    | ".gitattributes"
                    | ".editorconfig"
                    | ".prettierrc"
                    | ".eslintrc"
                    | ".babelrc"
                    | "tsconfig.json"
                    | "rustfmt.toml"
                    | "clippy.toml"
                    | ".rustfmt.toml"
                    | ".clippy.toml"
                    | "deny.toml"
            )
    }
}

impl fmt::Display for FileRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A collection of scanned files from a repository.
#[derive(Debug, Clone)]
pub struct Bundle {
    pub fingerprint: String,
    pub root: PathBuf,
    pub files: Vec<FileInfo>,
    pub scanned_at: SystemTime,
}

impl Bundle {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn total_tokens(&self) -> u64 {
        self.files.iter().map(|f| f.estimated_tokens()).sum()
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// A file with its computed relevance score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoredFile {
    pub path: String,
    pub score: f64,
    pub signals: SignalBreakdown,
    pub tokens: u64,
    pub language: Language,
    pub role: FileRole,
}

/// Per-signal score breakdown for explainability.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SignalBreakdown {
    pub bm25f: f64,
    pub heuristic: f64,
    pub pagerank: Option<f64>,
    pub git_recency: Option<f64>,
    pub embedding: Option<f64>,
}

/// The deep index containing pre-computed term frequencies and chunks.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct DeepIndex {
    pub version: u32,
    pub files: std::collections::HashMap<String, FileEntry>,
    pub avg_doc_length: f64,
    pub total_docs: u32,
    pub doc_frequencies: std::collections::HashMap<String, u32>,
    /// Normalized PageRank scores per file path (0.0–1.0).
    pub pagerank_scores: std::collections::HashMap<String, f64>,
}

/// Per-file entry in the deep index.
#[derive(Debug, Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileEntry {
    pub sha256: [u8; 32],
    pub chunks: Vec<Chunk>,
    pub term_frequencies: std::collections::HashMap<String, TermFreqs>,
    pub doc_length: u32,
}

/// A code chunk extracted by tree-sitter or regex fallback.
#[derive(
    Debug, Clone, Serialize, Deserialize, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize,
)]
pub struct Chunk {
    pub kind: ChunkKind,
    pub name: String,
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}

/// The kind of code chunk.
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    Serialize,
    Deserialize,
    rkyv::Archive,
    rkyv::Serialize,
    rkyv::Deserialize,
)]
pub enum ChunkKind {
    Function,
    Type,
    Impl,
    Import,
    Other,
}

/// Term frequency counts across different fields.
#[derive(Debug, Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct TermFreqs {
    pub filename: u32,
    pub symbols: u32,
    pub body: u32,
}

/// Token budget configuration for query results.
#[derive(Debug, Clone)]
pub struct TokenBudget {
    pub max_bytes: Option<u64>,
    pub max_tokens: Option<u64>,
}

impl TokenBudget {
    /// Enforce the token budget on a scored file list.
    ///
    /// Walks the sorted list in order, accumulating bytes and tokens.
    /// Stops including files once either limit is exceeded.
    /// Files are assumed to already be sorted by score (highest first).
    pub fn enforce(&self, files: &[ScoredFile]) -> Vec<ScoredFile> {
        let mut result = Vec::new();
        let mut total_bytes: u64 = 0;
        let mut total_tokens: u64 = 0;

        for file in files {
            let file_bytes = file.tokens * 4; // tokens = bytes / 4, so bytes = tokens * 4
            let file_tokens = file.tokens;

            if let Some(max_bytes) = self.max_bytes
                && total_bytes + file_bytes > max_bytes
                && !result.is_empty()
            {
                break;
            }
            if let Some(max_tokens) = self.max_tokens
                && total_tokens + file_tokens > max_tokens
                && !result.is_empty()
            {
                break;
            }

            total_bytes += file_bytes;
            total_tokens += file_tokens;
            result.push(file.clone());
        }

        result
    }
}
