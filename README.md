<div align="center">

# Atlas

**Smart file selection for LLMs. One command to go from codebase to context.**

[![Rust](https://img.shields.io/badge/Rust-2024_edition-000000?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![CI](https://img.shields.io/github/actions/workflow/status/demwunz/atlas/ci.yml?branch=main&style=for-the-badge&label=CI)](https://github.com/demwunz/atlas/actions)
[![MIT License](https://img.shields.io/github/license/demwunz/atlas?style=for-the-badge)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-212_passing-brightgreen?style=for-the-badge)](#)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Linux-blue?style=for-the-badge)](#-installation)

![Atlas demo](vhs/hero.gif)

[Quickstart](#-quickstart) · [Commands](#-commands) · [Scoring](#-scoring-engine) · [Installation](#-installation) · [Docs](docs/)

</div>

---

<details>
<summary>Table of Contents</summary>

- [The Problem](#-the-problem)
- [How Atlas Helps](#-how-atlas-helps)
- [Quickstart](#-quickstart)
- [Installation](#-installation)
- [Workflow](#-core-workflow)
- [Commands](#-commands)
- [Presets](#-presets)
- [Scoring Engine](#-scoring-engine)
- [Output Formats](#-output-formats)
- [Deep Indexing](#-deep-indexing)
- [Performance](#-performance)
- [Architecture](#-architecture)
- [Configuration Reference](#-configuration-reference)
- [Troubleshooting](#-troubleshooting)
- [Contributing](#-contributing)
- [License](#-license)

</details>

---

## The Problem

LLMs need context to be useful. But most codebases are too large to fit in a prompt.

You end up manually selecting files, guessing what's relevant, and hoping you didn't miss something important. Too little context and the LLM hallucinates. Too much and it loses focus.

**Atlas fixes this.** One command. Indexes your repo, scores every file against your task, outputs exactly what the LLM needs — within your token budget.

### Who is this for?

- **LLM agent builders** who need deterministic, machine-readable file selection
- **Developers using AI assistants** who want better context without manual cherry-picking
- **Teams building AI workflows** who need a fast, scriptable context pipeline

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## How Atlas Helps

- **Automatic file selection** — no manual cherry-picking
- **Multi-signal scoring** — BM25F text search, heuristics, import graphs, git history, all fused with RRF
- **Any file, any language** — indexes every file in your repo; regex chunking for fast indexing across 18 languages, tree-sitter available for on-demand enrichment
- **Token budgets** — `--max-bytes`, `--max-tokens`, `--min-score` for precise context control
- **Incremental indexing** — only re-processes changed files via SHA-256 change detection
- **Three output formats** — JSONL (pipes), JSON (APIs), human-readable (terminals)
- **Zero dependencies** — single static binary, no runtime deps

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Quickstart

```bash
# Install
brew install demwunz/tap/atlas

# Get context for your task
atlas quick "add health check endpoint"
```

That's it. Atlas indexes your repo, scores every file against your task, and outputs exactly the context an LLM needs.

![atlas quick demo](vhs/quick.gif)

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Installation

The standalone binary has no dependencies — download and run.

**Homebrew (macOS / Linux):**

```bash
brew install demwunz/tap/atlas
```

**Shell script:**

```bash
curl -fsSL https://raw.githubusercontent.com/demwunz/atlas/main/install.sh | bash
```

**From source (Rust 1.85+):**

```bash
git clone https://github.com/demwunz/atlas.git
cd atlas
cargo install --path crates/atlas-cli
```

**Verify installation:**

```bash
atlas --version
# atlas 0.1.0
```

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Core Workflow

Atlas has four main steps. Use `quick` to run them all at once, or each command individually for more control.

```bash
# 1. Index your repository
atlas index                              # Shallow scan (fast)
atlas index --deep                       # Deep index with AST chunking

# 2. Query for relevant files
atlas query "refactor auth middleware"    # Score and select files

# 3. Render context for an LLM
atlas render selection.jsonl             # Convert selection to formatted output

# 4. Understand scoring decisions
atlas explain "auth middleware" --top 10  # Per-file score breakdown
```

Or do it all in one shot:

```bash
atlas quick "refactor auth middleware" --preset balanced
```

![atlas workflow](vhs/query.gif)

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Commands

### `quick` — One-command context (start here)

Indexes, queries, and renders in a single step. Best for most users.

```bash
# Default (balanced preset)
atlas quick "refactor auth middleware"

# Fast preset (shallow index, heuristic scoring)
atlas quick "fix login bug" --preset fast

# Human-readable output
atlas quick "add retry logic" --format human

# With token budget
atlas quick "update API" --max-tokens 8000
```

| Flag | Default | Description |
|------|---------|-------------|
| `task` | *(required)* | Plain-English task description |
| `--preset` | `balanced` | Preset: `fast`, `balanced`, `deep`, `thorough` |
| `--max-bytes` | from preset | Maximum bytes budget |
| `--max-tokens` | none | Token budget |
| `--min-score` | from preset | Minimum score threshold |
| `--top` | none | Maximum number of files |
| `--format` | `auto` | Output: `auto`, `json`, `jsonl`, `human` |
| `--root` | `.` | Repository path |

<p align="right">(<a href="#atlas">back to top</a>)</p>

### `index` — Build a cached index

Walks the repo, classifies every file, and optionally builds a deep index with AST chunks and term frequencies.

```bash
# Shallow index (fast, path-based metadata only)
atlas index

# Deep index (adds AST chunks + term frequencies for BM25)
atlas index --deep

# Force rebuild from scratch
atlas index --deep --force
```

**Shallow vs deep:** A shallow index records file paths, sizes, languages, roles, and SHA-256 hashes. A deep index also parses source files into function-level chunks and pre-computes term frequencies. Deep mode is required for BM25F content scoring.

| Flag | Default | Description |
|------|---------|-------------|
| `--deep` | `false` | Enable AST chunking and term frequency extraction |
| `--force` | `false` | Rebuild index from scratch (ignore cache) |
| `--root` | `.` | Repository path |

### `query` — Select files for a task

Takes a task description, scores every file, and outputs a selection within your token budget.

```bash
# Basic query
atlas query "authentication middleware"

# With budget controls
atlas query "fix rate limiter" --max-bytes 50000 --min-score 0.1 --top 20

# Different preset
atlas query "add retry logic" --preset deep
```

| Flag | Default | Description |
|------|---------|-------------|
| `task` | *(required)* | Task description |
| `--preset` | `balanced` | Scoring preset |
| `--max-bytes` | from preset | Max total bytes |
| `--max-tokens` | none | Max total tokens |
| `--min-score` | from preset | Minimum score threshold |
| `--top` | none | Max files to select |

### `render` — Format output for LLMs

Converts a JSONL selection file into human-readable or structured output.

```bash
# Human-readable summary
atlas render selection.jsonl --format human

# JSON output
atlas render selection.jsonl --format json
```

| Flag | Default | Description |
|------|---------|-------------|
| `file` | *(required)* | Path to JSONL file |
| `--max-tokens` | none | Token budget for output |
| `--format` | `auto` | Output format |

### `explain` — Understand scoring decisions

Shows a table of selected files with their scores and signal breakdown.

```bash
atlas explain "auth middleware" --top 10
```

![explain output](vhs/render.gif)

Example output:

```
Score breakdown for query: "auth middleware"
Showing top 10 of 186 files

PATH                                               TOTAL    BM25F     HEUR     ROLE
--------------------------------------------------------------------------------------
src/auth/middleware.rs                             0.9500   0.8200   0.7100     impl
src/auth/handler.rs                               0.8700   0.6300   0.6800     impl
src/auth/mod.rs                                   0.7200   0.5100   0.5500     impl
...
```

| Flag | Default | Description |
|------|---------|-------------|
| `task` | *(required)* | Task description |
| `--top` | `10` | Number of files to show |

### `describe` — Machine-readable capabilities

Outputs a JSON description of Atlas's capabilities for agent discovery.

```bash
atlas describe --format json
```

```json
{
  "name": "atlas",
  "version": "0.1.0",
  "commands": ["index", "query", "quick", "render", "explain", "describe"],
  "formats": ["jsonl", "json", "human"],
  "languages": ["rust", "go", "python", "javascript", "typescript", "java", "ruby", "c", "cpp"],
  "scoring": ["heuristic", "content", "hybrid"],
  "presets": ["fast", "balanced", "deep", "thorough"]
}
```

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Presets

Presets configure index depth, scoring strategy, and budget in one flag.

| Preset | Index | Scoring | Max Bytes | Min Score | Use Case |
|--------|-------|---------|-----------|-----------|----------|
| `fast` | Shallow | Heuristic only | 50 KB | 0.05 | Quick lookups |
| `balanced` | Deep (cached) | BM25F + heuristic | 100 KB | 0.01 | **Default — recommended** |
| `deep` | Deep (fresh) | BM25F + structural | 200 KB | 0.005 | Thorough analysis |
| `thorough` | Deep + all signals | Everything | 500 KB | 0.001 | Maximum relevance |

Explicit flags override preset values:

```bash
atlas quick "auth" --preset fast --max-bytes 200000
```

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Scoring Engine

Atlas combines multiple signals using Reciprocal Rank Fusion (RRF) to produce a single relevance score per file.

### Signals

| Signal | Weight | Description |
|--------|--------|-------------|
| **BM25F** | 60% | Field-weighted text relevance (filename 5x, symbols 3x, body 1x) |
| **Heuristic** | 40% | Path keywords, file role, depth penalty, well-known paths, file size |
| **Import graph** | structural | PageRank over import/require relationships |
| **Git recency** | structural | Commit frequency per file (90-day lookback) |
| **File role** | classification | Boosts impl, penalizes generated/vendor |

### How it works

1. **Scan** — Walk the repo respecting `.gitignore`, classify language and role
2. **Score** — BM25F content matching + heuristic path analysis, blended 60/40
3. **Fuse** — Optional structural signals (PageRank, git recency) combined via RRF
4. **Budget** — Enforce `--max-bytes` / `--max-tokens`, greedily including top files
5. **Output** — Render as JSONL, JSON, or human-readable table

### File roles

Atlas classifies every file into a role that affects scoring:

| Role | Examples | Effect |
|------|----------|--------|
| `impl` | `src/main.rs`, `lib.py` | Boosted |
| `test` | `*_test.go`, `*.spec.ts` | Neutral |
| `config` | `.yaml`, `.env`, `.gitignore` | Slightly reduced |
| `docs` | `README.md`, `docs/` | Neutral |
| `build` | `Cargo.toml`, `Makefile` | Neutral |
| `generated` | `vendor/`, `node_modules/`, `*.pb.go` | Heavily penalized |

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Output Formats

### JSONL v0.3 (default for pipes)

Streaming format with header/body/footer. Each line is a self-contained JSON object.

```jsonl
{"Version":"0.3","Query":"auth middleware","Preset":"balanced","Budget":{"MaxBytes":100000},"MinScore":0.01}
{"Path":"src/auth/middleware.rs","Score":0.95,"Tokens":1200,"Language":"rust","Role":"impl"}
{"Path":"src/auth/handler.rs","Score":0.87,"Tokens":800,"Language":"rust","Role":"impl"}
{"TotalFiles":2,"TotalTokens":2000,"ScannedFiles":358}
```

### JSON (for APIs)

```bash
atlas query "auth" --format json
```

Returns a single JSON object with all results.

### Human-readable (for terminals)

```bash
atlas query "auth" --format human
```

Produces a formatted table. Auto-selected when stdout is a terminal.

### Pipe detection

When stdout is not a TTY, Atlas automatically switches to JSONL output and suppresses progress messages. Override with `--format`.

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Deep Indexing

A deep index adds two capabilities on top of the shallow scan:

- **AST chunks** — Function, type, impl, and import declarations extracted per file with names and line ranges
- **Term frequencies** — Pre-computed word counts across filename, symbols, and body fields for BM25F scoring

Build one with:

```bash
atlas index --deep
```

This creates `.atlas/index.json` in your repository root.

**Two-pass architecture:** Atlas indexes thousands of files but typically selects ~30 for your context window. Parsing every file with a full AST is wasted work. Instead, indexing uses fast regex chunking to extract function names, types, and imports — the same data BM25F scoring consumes. Tree-sitter's 18 language grammars remain compiled and available for a future enrichment pass that deep-parses only the files that win scoring. This is the same pattern used by Sourcegraph (search-based vs precise navigation), IntelliJ (stub index vs full PSI), and rust-analyzer (lazy parsing). On Kubernetes (28k files), this cuts indexing time in half.

**Incremental updates:** When you re-run `atlas index --deep`, only files whose SHA-256 has changed get re-indexed. Unchanged files carry forward from the existing index. File processing runs in parallel across all available cores via `rayon`.

**Supported languages for chunking (regex for indexing, tree-sitter for enrichment):**

| Language | Functions | Types | Imports | Impls |
|----------|-----------|-------|---------|-------|
| Rust | `fn` | `struct`, `enum`, `trait`, `type` | `use` | `impl` |
| Go | `func` | `type` | `import` | — |
| Python | `def`, `async def` | `class` | `import`, `from` | — |
| JavaScript | `function` | `class` | `import` | — |
| TypeScript | `function` | `class`, `interface`, `type`, `enum` | `import` | — |
| Java | methods | `class`, `interface`, `enum` | `import` | — |
| Ruby | `def` | `class`, `module` | `require` | — |
| C | functions | `struct`, `enum`, `union`, `typedef` | `#include` | — |
| C++ | functions | `class`, `struct`, `enum`, `namespace` | `#include` | — |
| Shell | functions | — | — | — |
| Swift | `func` | `class`, `struct`, `enum`, `protocol` | `import` | — |
| Kotlin | `fun` | `class`, `object` | `import` | — |
| Scala | `def` | `class`, `trait`, `object` | `import` | — |
| Haskell | functions | `data`, `newtype`, `type`, `class` | `import` | — |
| Elixir | `def` | — | — | — |
| Lua | `function` | — | — | — |
| PHP | functions | `class`, `interface`, `trait`, `enum` | `use` | — |
| R | functions | — | — | — |

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Performance

Benchmarked on Apple Silicon (M-series, release build).

### Synthetic repos

| Repo Size | Scan | Score | Render | Total |
|-----------|------|-------|--------|-------|
| 50 files | 1.0 ms | <0.1 ms | <0.1 ms | **1.0 ms** |
| 200 files | 3.4 ms | 0.2 ms | <0.1 ms | **3.6 ms** |
| 1,000 files | 18.4 ms | 1.6 ms | 0.2 ms | **20.2 ms** |

### Real-world: Kubernetes (28,358 files)

| Operation | Wall Clock | Peak RSS |
|-----------|-----------|----------|
| Shallow index | **1.8 s** | 72 MB |
| Deep index (fresh build) | **3.6 s** | 805 MB |
| End-to-end `quick` (cold) | **5.7 s** | 805 MB |
| End-to-end `quick` (cached) | **4.8 s** | 1.2 GB |

Deep indexing processes **27,827 source files** across all 18 supported languages in under 4 seconds — leveraging `rayon` for parallel file I/O and chunking. The generated index is 144 MB (rkyv binary) for the full Kubernetes codebase. Incremental updates skip unchanged files via SHA-256 comparison and avoid re-serializing when nothing changed.

Scoring and rendering are negligible — the bottleneck is file I/O.

Run benchmarks yourself:

```bash
cargo bench -p atlas-cli
```

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Architecture

Atlas is a Cargo workspace with 7 focused crates:

```
                    +-------------+
                    |  CLI (clap) |
                    +------+------+
                           |
              +------------+------------+
              |            |            |
        +-----+-----+ +---+---+ +-----+-----+
        |  Scanner   | | Index | |  Scoring  |
        |  (ignore)  | | (JSON)| |  Engine   |
        +-----+-----+ +---+---+ +-----+-----+
              |            |            |
              |     +------+------+    |
              |     |  Chunker    |    |
              |     | (regex/AST) |    |
              |     +-------------+    |
              |                        |
              +--------+---------------+
                       |
                 +-----+-----+
                 |   Render   |
                 | JSONL/JSON |
                 +-----------+
```

| Crate | Purpose |
|-------|---------|
| `atlas-core` | Domain types, traits, errors, token budget |
| `atlas-scanner` | File walking, gitignore, SHA-256 hashing |
| `atlas-index` | Deep index builder, JSON serialization, incremental merge |
| `atlas-score` | BM25F, heuristic, hybrid, PageRank, git recency, RRF fusion |
| `atlas-render` | JSONL v0.3, JSON, human-readable output |
| `atlas-treesit` | Code chunking (regex for indexing, tree-sitter for enrichment) |
| `atlas-cli` | clap CLI, presets, commands |

### Built with

- [Rust](https://www.rust-lang.org) (2024 edition)
- [`clap`](https://docs.rs/clap) — CLI parsing
- [`ignore`](https://docs.rs/ignore) — Gitignore-respecting file walking (from ripgrep)
- [`tree-sitter`](https://docs.rs/tree-sitter) — AST-based code enrichment (18 language grammars, on-demand for selected files)
- [`rayon`](https://docs.rs/rayon) — Parallel file processing
- [`serde`](https://docs.rs/serde) + [`serde_json`](https://docs.rs/serde_json) — Serialization
- [`sha2`](https://docs.rs/sha2) — Content hashing

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Configuration Reference

### Global flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root <path>` | `.` | Repository root (or set `ATLAS_ROOT`) |
| `--format <fmt>` | `auto` | Output format: `auto`, `json`, `jsonl`, `human` |
| `--no-color` | `false` | Disable color output |
| `-v` | `0` | Increase log verbosity (repeat for more) |
| `-q, --quiet` | `false` | Suppress non-essential output |

### Environment variables

| Variable | Description |
|----------|-------------|
| `ATLAS_ROOT` | Default repository root path |

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| Empty selection | No files matched the task | Broaden the task description or lower `--min-score` |
| Too many files selected | Budget too large | Use `--max-bytes` or `--top` to limit results |
| Stale results | Cached index from previous state | Run `atlas index --force` to rebuild |
| Slow on large repos | First index builds from scratch | Subsequent runs use incremental updates |
| JSONL output in terminal | Pipe detection thinks stdout isn't a TTY | Use `--format human` explicitly |
| No deep index data | Ran `atlas index` without `--deep` | Re-run with `--deep` flag |

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Contributing

Contributions are welcome. Atlas follows these conventions:

- `cargo clippy -- -D warnings` must pass
- `cargo fmt -- --check` must pass
- Tests live alongside source (`#[cfg(test)] mod tests`)
- `anyhow` for application errors, `thiserror` for library errors
- No `unsafe` without justification
- No `unwrap()` in library code

```bash
# Run all checks
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
cargo test --workspace
```

See [DELIVERY.md](docs/DELIVERY.md) for the full roadmap and [SPEC.md](docs/SPEC.md) for the technical specification.

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## Documentation

| Document | Description |
|----------|-------------|
| [PRD](docs/PRD.md) | Product requirements — what Atlas is and who it's for |
| [SPEC](docs/SPEC.md) | Technical specification — architecture, data formats, APIs |
| [RESEARCH](docs/RESEARCH.md) | Rust migration analysis and crate evaluation |
| [DELIVERY](docs/DELIVERY.md) | Phased delivery plan with 42 issues across 8 phases |

<p align="right">(<a href="#atlas">back to top</a>)</p>

---

## License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for details.

---

<div align="center">

**[Report Bug](https://github.com/demwunz/atlas/issues) · [Request Feature](https://github.com/demwunz/atlas/issues)**

Made by [Fazal Majid](https://github.com/demwunz)

</div>
