# Topo — Technical Specification

## 1. Architecture Overview

Single static binary. No runtime dependencies. No Python. No CGo equivalent.

Core pipeline:
```
Scanner -> Indexer -> Scorer -> Selector -> Renderer
```

Each stage is a separate module with clean interfaces. The CLI orchestrates the pipeline based on the command and preset.

### Crate Layout (planned)

```
topo/
├── Cargo.toml          (workspace root)
├── crates/
│   ├── topo-core/     (domain types, traits, errors)
│   ├── topo-scanner/  (file walking, gitignore, hashing)
│   ├── topo-index/    (deep index: chunks, rkyv serialization)
│   ├── topo-score/    (BM25F, heuristic, structural, RRF fusion)
│   ├── topo-render/   (JSONL v0.3, JSON, human output)
│   ├── topo-treesit/  (tree-sitter integration, grammar loading)
│   └── topo-cli/      (clap CLI, presets, commands)
└── tests/              (integration tests)
```

## 2. Data Model

### 2.1 FileInfo
```rust
pub struct FileInfo {
    pub path: String,        // Relative to repo root
    pub size: u64,           // Bytes
    pub language: Language,   // Detected language
    pub role: FileRole,      // impl/test/config/docs/generated
    pub sha256: [u8; 32],   // Content hash for incremental updates
}
```

### 2.2 Language (enum)
Detected from file extension. Covers 100+ languages. Used to select tree-sitter grammar.

### 2.3 FileRole (enum)
```rust
pub enum FileRole {
    Implementation,  // Source code
    Test,           // Test files (*_test.go, *_spec.rs, test_*.py)
    Config,         // YAML, TOML, JSON config files
    Documentation,  // Markdown, RST, docs/ directory
    Generated,      // Generated code (auto-detected patterns)
    Build,          // Makefiles, Cargo.toml, package.json
    Other,
}
```

### 2.4 Bundle
```rust
pub struct Bundle {
    pub fingerprint: String,    // Deterministic repo identity
    pub root: PathBuf,          // Repo root
    pub files: Vec<FileInfo>,   // All scanned files
    pub scanned_at: SystemTime,
}
```

### 2.5 DeepIndex
```rust
pub struct DeepIndex {
    pub version: u32,
    pub files: HashMap<String, FileEntry>,
    pub avg_doc_length: f64,      // For BM25 normalization
    pub total_docs: u32,
    pub doc_frequencies: HashMap<String, u32>,  // Term -> doc count
}

pub struct FileEntry {
    pub sha256: [u8; 32],
    pub chunks: Vec<Chunk>,
    pub term_frequencies: HashMap<String, TermFreqs>,
    pub doc_length: u32,  // Total terms
}

pub struct Chunk {
    pub kind: ChunkKind,     // Function, Type, Impl, Import, Other
    pub name: String,        // Symbol name
    pub start_line: u32,
    pub end_line: u32,
    pub content: String,
}

pub struct TermFreqs {
    pub filename: u32,   // Occurrences in filename
    pub symbols: u32,    // Occurrences in chunk names
    pub body: u32,       // Occurrences in chunk bodies
}
```

### 2.6 ScoredFile
```rust
pub struct ScoredFile {
    pub path: String,
    pub score: f64,
    pub signals: SignalBreakdown,
    pub tokens: u64,      // Estimated tokens (bytes / 4)
    pub language: Language,
    pub role: FileRole,
}

pub struct SignalBreakdown {
    pub bm25f: f64,
    pub heuristic: f64,
    pub pagerank: Option<f64>,
    pub git_recency: Option<f64>,
    pub embedding: Option<f64>,
}
```

## 3. Index Formats

### 3.1 Shallow Index (JSONL v0.3)
Used for fast heuristic-only queries. Same format as output.

```jsonl
{"Version":"0.3","Query":"","Preset":"index","Budget":{},"MinScore":0}
{"Path":"src/main.rs","Score":0,"Tokens":500,"Language":"rust","Role":"impl"}
{"TotalFiles":358,"TotalTokens":150000,"ScannedFiles":358}
```

Stored in `.topo-cache/<fingerprint>.jsonl`

### 3.2 Deep Index (rkyv binary)
Zero-copy deserialization via rkyv + memmap2. The index file is memory-mapped and accessed directly without parsing.

Stored in `.topo-cache/<fingerprint>.deep.rkyv`

Key design decisions:
- rkyv `Archive` derive on all index types
- `memmap2` for zero-copy file access
- Aligned allocation for rkyv safety
- Checksum header for integrity verification

### 3.3 Incremental Updates
On `topo index`:
1. Scan all files, compute SHA-256 hashes
2. Load existing deep index (if any)
3. For each file: if hash unchanged, keep existing entry; if changed/new, re-chunk and re-index
4. Remove entries for deleted files
5. Save updated index

## 4. Scoring Pipeline

### 4.1 Tokenizer
- Whitespace splitting
- camelCase / PascalCase splitting (insertBreak -> ["insert", "Break"])
- snake_case splitting
- Stop word removal (the, a, an, is, are, etc.)
- Lowercase normalization
- No stemming (preserves exact matches)

### 4.2 BM25F Scorer
Field-weighted BM25 (Robertson et al., 2004):

```
BM25F(q, d) = sum( IDF(t) * (tf_weighted / (tf_weighted + k1)) )
```

Where:
- `tf_weighted = w_filename * tf_filename + w_symbols * tf_symbols + w_body * tf_body`
- Field weights: filename=5.0, symbols=3.0, body=1.0
- k1 = 1.2 (term frequency saturation)
- b = 0.75 (document length normalization)
- IDF uses log((N - df + 0.5) / (df + 0.5) + 1)

### 4.3 Heuristic Scorer
Path-based scoring signals:
- Directory depth penalty (deeper = less relevant)
- Keyword match bonus (query terms in path segments)
- File role bonus (implementation > test > config > docs)
- Size penalty (very large files penalized)
- Well-known path bonus (src/, lib/, cmd/ get boost)

### 4.4 Structural Signals
- **Import graph PageRank**: Build directed graph from import/require/use statements. Run PageRank. Files imported by many others score higher.
- **Git recency**: `git log --format=%H --since=90.days -- <file>` commit count. More recent activity = higher score.

### 4.5 RRF Fusion
```rust
fn rrf_score(rankings: &[Vec<(String, f64)>], k: f64) -> HashMap<String, f64> {
    let mut scores: HashMap<String, f64> = HashMap::new();
    for ranking in rankings {
        for (rank, (file, _)) in ranking.iter().enumerate() {
            *scores.entry(file.clone()).or_default() += 1.0 / (k + rank as f64 + 1.0);
        }
    }
    scores
}
```

k=60 (standard RRF constant).

## 5. tree-sitter Integration

### 5.1 Grammar Loading
Grammars are statically linked at compile time. No runtime grammar downloads.

Supported languages (Phase 5):
Go, Rust, Python, JavaScript, TypeScript, Java, Ruby, C, C++

Additional languages added post-v1 via tree-sitter grammar crates.

### 5.2 Chunk Extraction
For each file, tree-sitter parses the AST and extracts:
- **Functions**: name, parameters, return type, body
- **Types/Structs/Classes**: name, fields/methods
- **Impl blocks** (Rust): associated functions
- **Imports**: module paths

Each chunk has a kind, name, start/end line, and content.

### 5.3 Fallback
For languages without tree-sitter grammars, a regex-based chunker extracts functions and types using language-specific patterns (same approach as repo-context's regex chunkers).

## 6. CLI Interface

### 6.1 Global Flags
```
--verbose, -v       Increase log verbosity
--quiet, -q         Suppress non-essential output
--format <fmt>      Output format: auto, json, jsonl, human (default: auto)
--no-color          Disable color output
--root <path>       Repository root (default: current directory)
```

### 6.2 Commands

```
topo index [--deep] [--force]
    Build or update the index. --deep includes AST chunking.
    --force rebuilds from scratch.

topo query <task> [--preset <p>] [--scoring <s>] [--max-bytes <n>]
                   [--max-tokens <n>] [--min-score <f>] [--rerank] [--top <n>]
    Score files against task description. Output JSONL selection.
    --preset: fast|balanced|deep|thorough (default: balanced)
    --scoring: heuristic|content|hybrid (default: hybrid)

topo quick <task> [--preset <p>] [flags...]
    One-shot: index + query. Accepts all query flags.

topo render <jsonl-file> [--max-tokens <n>]
    Convert JSONL selection to formatted LLM context.

topo explain <task> [--top <n>]
    Show score breakdown per file.

topo describe [--json]
    Machine-readable capabilities for agent discovery.
```

## 7. Crate Dependencies

| Crate | Purpose | Version |
|-------|---------|---------|
| `clap` (derive) | CLI argument parsing | 4.x |
| `serde` + `serde_json` | JSON/JSONL serialization | 1.x |
| `ignore` | File walking with gitignore support | 0.4.x |
| `rkyv` | Zero-copy serialization for deep index | 0.8.x |
| `memmap2` | Memory-mapped file access | 0.9.x |
| `tree-sitter` | AST parsing | 0.24.x |
| `rayon` | Parallel iteration for index building | 1.x |
| `sha2` | SHA-256 content hashing | 0.10.x |
| `anyhow` | Error handling | 1.x |
| `reqwest` | HTTP client for embedding APIs (optional) | 0.12.x |
| `tiktoken-rs` | Token counting (optional) | 0.6.x |

## 8. Output Formats

### 8.1 JSONL v0.3 (default for pipes)
```jsonl
{"Version":"0.3","Query":"auth middleware","Preset":"balanced","Budget":{"MaxBytes":100000},"MinScore":0.01}
{"Path":"src/auth/middleware.rs","Score":0.95,"Tokens":1200,"Language":"rust","Role":"impl"}
{"TotalFiles":2,"TotalTokens":2000,"ScannedFiles":358}
```

### 8.2 JSON (--format json)
```json
{
  "version": "0.3",
  "query": "auth middleware",
  "files": [...],
  "total_files": 2,
  "total_tokens": 2000
}
```

### 8.3 Human-readable (--format human, default for TTY)
Colored table output with scores, paths, and token counts.

## 9. Configuration

### 9.1 Feature Scopes
`.topo/features.yaml` (also reads `.repo-context/features.yaml`):
```yaml
features:
  auth:
    include:
      - "src/auth/**"
      - "src/middleware/auth*"
    exclude:
      - "**/*_test.*"
```

### 9.2 Presets
Built-in presets configure scoring depth:

| Preset | Index | Scoring | Signals |
|--------|-------|---------|---------|
| fast | shallow | heuristic | path only |
| balanced | deep (cached) | hybrid | BM25F + heuristic |
| deep | deep (fresh) | hybrid | BM25F + heuristic + structural |
| thorough | deep + rerank | all | BM25F + heuristic + structural + embeddings |

### 9.3 Environment Variables
- `TOPO_ROOT`: Override repository root
- `TOPO_CACHE_DIR`: Override cache directory
- `TOPO_PRESET`: Default preset
- `OLLAMA_HOST`: Ollama endpoint for embeddings (default: http://localhost:11434)
- `OPENAI_API_KEY`: OpenAI API key for embeddings

## 10. Integration

### 10.1 Wobot Toolchain Resolver
Wobot's toolchain routing (W4.3) will detect the `topo` binary and prefer it over `repo-context` when available. The `wobot context` command will delegate to whichever binary is found first.

### 10.2 Pipe Detection
When stdout is not a TTY, Topo automatically:
- Switches to JSONL output
- Disables color
- Suppresses progress bars

### 10.3 Agent Discovery
`topo describe --json` returns machine-readable capabilities:
```json
{
  "name": "topo",
  "version": "0.1.0",
  "commands": ["index", "query", "quick", "render", "explain", "describe"],
  "formats": ["jsonl", "json", "human"],
  "languages": ["go", "rust", "python", "javascript", "typescript", "java", "ruby", "c", "cpp"],
  "scoring": ["heuristic", "content", "hybrid"],
  "presets": ["fast", "balanced", "deep", "thorough"]
}
```
