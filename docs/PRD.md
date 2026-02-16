# Topo — Product Requirements Document

## 1. Problem Statement

LLM agents (Claude Code, Cursor, Copilot, custom frameworks) need relevant codebase context to perform tasks effectively, but they cannot read entire repositories. A 30,000-file repo like Kubernetes has ~239MB of source code — far beyond any context window.

Current approaches:
- **Manual file selection**: Slow, error-prone, misses relevant files
- **Grep/ripgrep**: Finds exact matches but misses semantically related files
- **Embedding-based search**: Requires external infrastructure (vector DBs, embedding APIs)
- **repo-context (Go)**: Works well but hits performance ceiling at scale — gob deserialization takes 500ms+ on large repos, supports only 8 languages for AST chunking, binary is 15MB+

Topo solves this by providing a fast, standalone binary that indexes any codebase and selects the most relevant files using multiple scoring signals — all without external dependencies.

## 2. Product Vision

Topo is the fastest way to answer: "Which files in this codebase are most relevant to this task?"

It is purpose-built for LLM context windows: it scores, selects, and formats codebase context in a single pass, respecting token budgets and producing structured output that LLM agents can consume directly.

## 3. Target Users

**Primary:**
- LLM agent frameworks (wobot, Claude Code, aider, continue.dev)
- AI coding assistants that need automated file selection
- Developers using Claude Code, Cursor, or Copilot who want better context

**Secondary:**
- Code review tools that need to identify relevant files for a change
- Documentation generators that need to understand code structure
- Onboarding tools that help developers navigate unfamiliar codebases

## 4. Core Capabilities

### 4.1 Index
- Shallow index: file paths, sizes, languages, roles — built in <100ms
- Deep index: AST chunks, term frequencies, document lengths — built in <5s for 30k files
- Incremental updates: SHA-256 change detection, only re-index modified files
- Storage: rkyv zero-copy binary format, memory-mapped for instant loading

### 4.2 Query
- Multi-signal scoring: BM25F text relevance, heuristic path scoring, import graph PageRank, git recency, file role classification
- Reciprocal Rank Fusion: combines all signals into a single ranking
- Token budget enforcement: --max-bytes, --max-tokens, --min-score
- Presets: fast, balanced, deep, thorough — each configures index depth and scoring complexity

### 4.3 Render
- JSONL v0.3 output: header with query metadata, one line per selected file, footer with totals
- Human-readable output: formatted for terminal display
- JSON output: structured for programmatic consumption
- Pipe detection: automatically switches output format based on whether stdout is a TTY

### 4.4 Explain
- Score breakdown per file: shows contribution of each signal
- Useful for debugging scoring behavior and understanding why files were selected or excluded

## 5. Non-Goals

- **Not an IDE plugin** — Topo is a CLI tool; IDE integration is for consumers to build
- **Not a code search engine** — It selects files, not lines. Use ripgrep for line-level search.
- **Not a replacement for grep/ripgrep** — Different purpose: selection vs search
- **No LLM inference in the binary** — Scoring is algorithmic. Embeddings are optional via external HTTP.
- **No vector database** — All indices are local files. No infrastructure required.
- **No Python dependencies** — Zero Python anywhere in the build or runtime.

## 6. Success Metrics

| Metric | Target | Current (Go) |
|--------|--------|---------------|
| Balanced query (30k files) | <1s | 2.4s |
| Deep index load (30k files) | <10ms | 498ms (gob) |
| Deep index build (30k files) | <5s | 4.4s (parallel) |
| Language support | 100+ | 8 |
| Binary size | <4MB | ~15MB |
| Memory (30k file query) | <50MB | ~200MB |
| JSONL compatibility | 100% | N/A (baseline) |

## 7. Scoring Architecture

### 7.1 Signals

| Signal | Source | Weight | Implementation |
|--------|--------|--------|----------------|
| BM25F | Deep index | High | Field-weighted: filename 5x, symbols 3x, body 1x |
| Heuristic | File paths | Medium | Path patterns, keywords, directory structure |
| Import graph | Source analysis | Medium | PageRank over import/require/use relationships |
| Git recency | git log | Low | Commit frequency per file in recent history |
| File role | Filename patterns | Low | test/impl/config/docs/generated classification |
| Embeddings | External HTTP | Optional | Ollama/OpenAI via --rerank flag |

### 7.2 Fusion

Reciprocal Rank Fusion (RRF) combines ranked lists from each signal:

```
RRF_score(file) = sum( 1 / (k + rank_i(file)) )
```

Where k=60 (standard constant) and rank_i is the file's rank in signal i's sorted list.

### 7.3 Budget Enforcement

After scoring, files are selected greedily by score until the token budget is exhausted:
- `--max-bytes N`: Total bytes of selected files
- `--max-tokens N`: Total estimated tokens (bytes / 4)
- `--min-score F`: Minimum score threshold

## 8. Compatibility

Topo is a drop-in replacement for repo-context:
- Same CLI commands: index, query, render, explain, quick, describe
- Same flags: --preset, --max-bytes, --max-tokens, --min-score, --rerank, --scoring
- Same JSONL v0.3 output format: header/file/footer structure
- Same feature scopes: .repo-context/features.yaml (also reads .topo/features.yaml)
- Same cache directory structure: .repo-context-cache/ (also uses .topo-cache/)

## 9. Constraints

- Single static binary, no runtime dependencies
- Must work offline (embeddings are optional)
- Must handle repos up to 100k files
- Must support macOS (ARM + x86) and Linux (x86_64)
- Index files must be backward-compatible within major versions
- JSONL output must be byte-identical to repo-context for the same input
