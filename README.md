# Atlas

Fast codebase indexer and file selector for LLMs. Written in Rust.

## What is Atlas?

Atlas scans a codebase, indexes its structure and content, then selects the most relevant files for a given task — purpose-built for LLM context windows.

LLM agents need codebase context but can't read entire repositories. Atlas solves this by scoring every file against your query using multiple signals (BM25F text relevance, structural analysis, git history, import graphs) and returning a token-budgeted selection in a single pass.

## Key Features

- **Fast**: <1s balanced query on 30k-file repos. rkyv zero-copy index loading.
- **Smart scoring**: BM25F field-weighted text search + structural signals + Reciprocal Rank Fusion.
- **100+ languages**: tree-sitter AST chunking with exact function/type boundaries.
- **Correct gitignore**: Full `.gitignore` spec via the `ignore` crate (from ripgrep).
- **Token budgets**: `--max-bytes`, `--max-tokens`, `--min-score` for precise context control.
- **Drop-in replacement**: Same commands, flags, and JSONL v0.3 output as repo-context.
- **Zero dependencies**: Single static binary, no Python, no runtime deps.

## Installation

```bash
# Homebrew (macOS / Linux)
brew install demwunz/tap/atlas

# Shell script
curl -fsSL https://raw.githubusercontent.com/demwunz/atlas/main/install.sh | sh

# From source
cargo install atlas-cli
```

## Quick Start

```bash
# Index and query in one shot
atlas quick "authentication middleware" --preset balanced

# Step by step
atlas index                          # Build shallow + deep index
atlas query "auth middleware"         # Score and select files
atlas render selection.jsonl         # Format for LLM context

# Explain scoring decisions
atlas explain "auth middleware" --top 10
```

## Commands

| Command | Description |
|---------|-------------|
| `atlas index` | Build shallow and/or deep index |
| `atlas query` | Score files against a task, output JSONL selection |
| `atlas quick` | One-shot: index + query + render |
| `atlas render` | Convert JSONL selection to formatted context |
| `atlas explain` | Show score breakdown per file |
| `atlas describe` | Machine-readable capabilities (for agent discovery) |

## Presets

| Preset | Index | Scoring | Use Case |
|--------|-------|---------|----------|
| `fast` | Shallow only | Heuristic | Quick lookups, <100ms |
| `balanced` | Deep (cached) | BM25F + heuristic | Default, <1s |
| `deep` | Deep (fresh) | BM25F + structural | Thorough analysis |
| `thorough` | Deep + rerank | All signals + embeddings | Maximum relevance |

## Output Format

Atlas outputs JSONL v0.3, compatible with repo-context:

```jsonl
{"Version":"0.3","Query":"auth middleware","Preset":"balanced","Budget":{"MaxBytes":100000},"MinScore":0.01}
{"Path":"src/auth/middleware.rs","Score":0.95,"Tokens":1200,"Language":"rust","Role":"impl"}
{"Path":"src/auth/mod.rs","Score":0.87,"Tokens":800,"Language":"rust","Role":"impl"}
{"TotalFiles":2,"TotalTokens":2000,"ScannedFiles":358}
```

## Scoring Architecture

Atlas combines multiple signals using Reciprocal Rank Fusion (RRF):

1. **BM25F** — Field-weighted text relevance (filename 5x, symbols 3x, body 1x)
2. **Heuristic** — Path-based scoring, keyword matching
3. **Import graph** — PageRank over import/require relationships
4. **Git recency** — Recent commit frequency per file
5. **File role** — Classification (test/impl/config/docs/generated)
6. **Embeddings** — Optional reranking via Ollama/OpenAI (`--rerank`)

## Architecture

```
                    ┌─────────────┐
                    │   CLI (clap) │
                    └──────┬──────┘
                           │
              ┌────────────┼────────────┐
              │            │            │
        ┌─────┴─────┐ ┌───┴───┐ ┌─────┴─────┐
        │  Scanner   │ │ Index │ │  Scoring  │
        │  (ignore)  │ │(rkyv) │ │  Engine   │
        └─────┬─────┘ └───┬───┘ └─────┬─────┘
              │            │            │
              │     ┌──────┴──────┐    │
              │     │ tree-sitter │    │
              │     │  Chunking   │    │
              │     └─────────────┘    │
              │                        │
              └────────┬───────────────┘
                       │
                 ┌─────┴─────┐
                 │   JSONL   │
                 │  Output   │
                 └───────────┘
```

## Documentation

- [Product Requirements](docs/PRD.md) — What Atlas is, who it's for, what it does
- [Technical Specification](docs/SPEC.md) — Architecture, data formats, APIs
- [Research](docs/RESEARCH.md) — Rust migration analysis and crate evaluation
- [Delivery Plan](docs/DELIVERY.md) — Phased delivery with all 42 issues

## Project Status

Atlas is in active development. See the [delivery plan](docs/DELIVERY.md) and the [GitHub project board](https://github.com/demwunz/atlas/projects) for current progress.

## Background

Atlas is a Rust rewrite of [repo-context](https://github.com/demwunz/wobot), which was written in Go. The rewrite unlocks:

- **~100x faster index loading** via rkyv zero-copy deserialization (vs gob decode)
- **100+ language support** via tree-sitter (vs 8 regex-based chunkers)
- **Correct gitignore** via the `ignore` crate (vs custom glob matching)
- **BM25F field-weighted scoring** (vs flat BM25)
- **2-3x smaller binary** with no CGo
- **WASM deployment** potential for browser-based indexing

## License

MIT
