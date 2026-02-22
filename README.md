<div align="center">

# Topo

**Smart file selection for LLMs. One command to go from codebase to context.**

  <img src="https://img.shields.io/badge/Rust-000000?style=flat-square&logo=rust&logoColor=white" alt="Rust" />
  <img src="https://img.shields.io/badge/CLI-000000?style=flat-square&logo=windowsterminal&logoColor=white" alt="CLI" />
  <img src="https://img.shields.io/badge/MCP-191919?style=flat-square&logo=anthropic&logoColor=white" alt="MCP" />
  <img src="https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white" alt="macOS" />
  <img src="https://img.shields.io/badge/Linux-FCC624?style=flat-square&logo=linux&logoColor=black" alt="Linux" />
  <img src="https://img.shields.io/badge/Windows-0078D4?style=flat-square&logo=windows&logoColor=white" alt="Windows" />

[Quickstart](#quickstart) · [MCP Server](#mcp-server) · [AI Setup](#ai-assistant-setup) · [Hooks](#claude-code-hooks) · [Commands](#commands) · [Scoring](#scoring-engine) · [Installation](#installation)

![Topo demo](vhs/hero.gif)

</div>

---

**Instant. Precise. Lean.**

Topo scores every file in your codebase against your task and returns ranked, token-budgeted context. Results arrive in milliseconds. Re-indexes skip unchanged files, so even large codebases stay fast. No API calls, no cloud.

Describe "auth middleware" and get back the implementation, its dependencies, and the config that wires it up — even when none of them match your search terms.

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Features

- ⚡ **Millisecond results** — small repos return instantly. 28,000-file codebases index in under 4 seconds. Changed files re-index automatically; unchanged files are skipped
- 🎯 **Finds files keywords miss** — scores every file by text relevance, path structure, import relationships, git history, and file role. A hub module imported by 40 files ranks high even if its name has nothing to do with your task
- 🎛️ **Precise token budgets** — `--max-tokens`, `--max-bytes`, `--top`, `--min-score`. Fill your context window with signal, not noise
- 🔌 **Works with every AI tool** — native hooks for Claude Code, rules for Cursor, instructions for Copilot, MCP server for everything else. `topo init` sets them all up
- 📦 **Single binary, zero config** — 18 languages out of the box. No runtime, no API keys. Download and run

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Quickstart

```bash
# Install
brew install demwunz/tap/topo

# Set up your AI assistants
topo init

# Get context for your task
topo quick "add health check endpoint"
```

`topo init` creates instruction files that teach your AI assistants to use Topo for file discovery. `topo quick` indexes your repo, scores every file against your task, and outputs exactly the context an LLM needs.

![topo quick demo](vhs/quick.gif)

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Installation

The standalone binary has no dependencies — download and run.

**Homebrew (macOS / Linux):**

```bash
brew install demwunz/tap/topo
```

**Shell script:**

```bash
curl -fsSL https://raw.githubusercontent.com/demwunz/topo/main/install.sh | bash
```

**From source (Rust 1.85+):**

```bash
git clone https://github.com/demwunz/topo.git
cd topo
cargo install --path crates/topo-cli
```

**Verify installation:**

```bash
topo --version
# topo 0.1.0
```

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## MCP Server

Use Topo as an [MCP](https://modelcontextprotocol.io/) server in Claude Desktop, Cursor, Cline, or any MCP client. Exposes `topo_query`, `topo_explain`, and `topo_index` as tools.

```json
{
  "mcpServers": {
    "topo": {
      "command": "topo",
      "args": ["--root", "/path/to/project", "mcp"]
    }
  }
}
```

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## AI Assistant Setup

Make every AI coding assistant use Topo for file discovery with one command:

```bash
topo init
```

This creates instruction files that tell AI assistants to run `topo quick` via shell before grep/find/glob, and installs [Claude Code hooks](#claude-code-hooks) for automatic context injection. It also checks that `topo` is on your PATH so assistants can actually call it:

| File | Purpose |
|------|---------|
| `AGENTS.md` | Cross-tool instructions (Codex, Claude Code, Jules, Cursor) |
| `CLAUDE.md` | Injects a topo-managed section (preserves your existing content) |
| `.cursor/rules/topo.md` | Cursor-specific rules (auto-applied) |
| `.github/copilot-instructions.md` | GitHub Copilot instructions (if `.github/` exists) |
| `.claude/hooks/topo-context.sh` | Claude Code hook: injects file suggestions on prompt submit |
| `.claude/hooks/topo-hint.sh` | Claude Code hook: hints when Glob/Grep are used |
| `.claude/hooks/topo-track.sh` | Claude Code hook: tracks file reads for `topo gain` |
| `.claude/settings.json` | Hook registration (merged into existing settings) |

Existing files are not overwritten. Use `topo init --force` to replace them. `CLAUDE.md` is special — it injects a marked section rather than overwriting, so your project instructions are preserved.

To skip hook installation: `topo init --hooks false`.

For tools without shell access, combine with the [MCP server](#mcp-server) config above.

![topo init demo](vhs/init.gif)

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Claude Code Hooks

When hooks are installed (default with `topo init`), Topo works automatically — no manual commands needed:

**`UserPromptSubmit` — automatic context injection.** When you submit a prompt, Topo reads it, runs `topo quick` with the fast preset, and returns the top 10 relevant files as additional context. Claude sees the right files before it starts working. Short prompts (<15 chars) are skipped to avoid noise on commands like `/help`.

**`PreToolUse` on Glob/Grep — discovery hints.** When Claude is about to search for files with Glob or Grep, Topo injects a lightweight hint with the top 5 files matching the search pattern. This doesn't block the tool call — it adds context.

**`PostToolUse` on Read — usage tracking.** When Claude reads a file, Topo logs it to `.topo/stats.jsonl` for the [`topo gain`](#gain--context-savings) analytics command.

All hooks are additive — they inject `additionalContext` and never block tool calls. The `UserPromptSubmit` hook uses `--preset fast` to keep latency under 2 seconds.

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Core Workflow

Topo has four main steps. Use `quick` to run them all at once, or each command individually for more control.

```bash
# 1. Index your repository
topo index                              # Shallow scan (fast)
topo index --deep                       # Deep index with AST chunking

# 2. Query for relevant files
topo query "refactor auth middleware"    # Score and select files

# 3. Render context for an LLM
topo render selection.jsonl             # Convert selection to formatted output

# 4. Understand scoring decisions
topo explain "auth middleware" --top 10  # Per-file score breakdown
```

Or do it all in one shot:

```bash
topo quick "refactor auth middleware" --preset balanced
```

![topo workflow](vhs/query.gif)

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Commands

### `quick` — One-command context (start here)

Indexes, queries, and renders in a single step. Best for most users.

```bash
# Default (balanced preset)
topo quick "refactor auth middleware"

# Fast preset (shallow index, heuristic scoring)
topo quick "fix login bug" --preset fast

# Human-readable output
topo quick "add retry logic" --format human

# With token budget
topo quick "update API" --max-tokens 8000
```

| Flag | Default | Description |
|------|---------|-------------|
| `task` | *(required)* | Plain-English task description |
| `--preset` | `balanced` | Preset: `fast`, `balanced`, `deep`, `thorough` |
| `--max-bytes` | from preset | Maximum bytes budget |
| `--max-tokens` | none | Token budget |
| `--min-score` | from preset | Minimum score threshold |
| `--top` | none | Maximum number of files |
| `--format` | `auto` | Output: `auto`, `json`, `jsonl`, `human`, `compact` |
| `--root` | `.` | Repository path |

<p align="right">(<a href="#topo">back to top</a>)</p>

### `index` — Build a cached index

Walks the repo, classifies every file, and optionally builds a deep index with AST chunks and term frequencies.

```bash
# Shallow index (fast, path-based metadata only)
topo index

# Deep index (adds AST chunks + term frequencies for BM25)
topo index --deep

# Force rebuild from scratch
topo index --deep --force
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
topo query "authentication middleware"

# With budget controls
topo query "fix rate limiter" --max-bytes 50000 --min-score 0.1 --top 20

# Different preset
topo query "add retry logic" --preset deep
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
topo render selection.jsonl --format human

# JSON output
topo render selection.jsonl --format json
```

| Flag | Default | Description |
|------|---------|-------------|
| `file` | *(required)* | Path to JSONL file |
| `--max-tokens` | none | Token budget for output |
| `--format` | `auto` | Output format |

### `explain` — Understand scoring decisions

Shows a table of selected files with their scores and signal breakdown.

```bash
topo explain "auth middleware" --top 10
topo explain "auth middleware" --preset deep --top 10  # includes PageRank
```

![explain output](vhs/render.gif)

Example output (with `--preset deep`):

```
Score breakdown for query: "auth middleware"
Showing top 10 of 186 files

PATH                                                  TOTAL    BM25F     HEUR       PR     ROLE
-----------------------------------------------------------------------------------------------
src/auth/middleware.rs                               0.9500   0.8200   0.7100   0.8340     impl
src/auth/handler.rs                                  0.8700   0.6300   0.6800   0.6210     impl
src/auth/mod.rs                                      0.7200   0.5100   0.5500   0.5080     impl
...
```

The `PR` column shows normalized PageRank scores (0–1) when using `deep` or `thorough` presets, or `-` otherwise.

| Flag | Default | Description |
|------|---------|-------------|
| `task` | *(required)* | Task description |
| `--top` | `10` | Number of files to show |
| `--preset` | `balanced` | Scoring preset (`deep`/`thorough` enable PageRank) |

### `inspect` — Index statistics

Shows metadata and statistics for the current index file.

```bash
topo inspect
```

Example output:

```
Index: .topo/index.bin
Format: rkyv binary
Size: 144.0 MB (150994944 bytes)
Version: 2
Files: 28358
Chunks: 142891
Unique terms: 89412
Terms (file-level): 312044
Avg doc length: 1523.4

Files by extension:
  .go             18923
  .json            3412
  .yaml            1205
  ...
```

### `init` — Set up AI assistants

Creates instruction files and installs Claude Code hooks. See [AI Assistant Setup](#ai-assistant-setup).

```bash
topo init              # Create files + install hooks
topo init --force      # Overwrite existing files
topo init --hooks false  # Skip hook installation
```

| Flag | Default | Description |
|------|---------|-------------|
| `--force` | `false` | Overwrite existing files |
| `--hooks` | `true` | Install Claude Code hooks |

### `gain` — Context savings

Shows how much context Topo has saved across Claude Code sessions. Reads tracking data from `.topo/stats.jsonl` written by the hooks.

```bash
topo gain
```

Example output:

```
Topo context savings:
  Sessions:         12
  Suggestions:      47
  Files suggested:  156
  Files opened:     89
  Tokens suggested: 847000
  Avg files/query:  3.3
```

### `describe` — Machine-readable capabilities

Outputs a JSON description of Topo's capabilities for agent discovery.

```bash
topo describe --format json
```

```json
{
  "name": "topo",
  "version": "0.1.0",
  "commands": ["index", "query", "quick", "render", "explain", "inspect", "describe"],
  "formats": ["jsonl", "json", "human"],
  "languages": ["rust", "go", "python", "javascript", "typescript", "java", "ruby", "c", "cpp"],
  "scoring": ["heuristic", "content", "hybrid"],
  "presets": ["fast", "balanced", "deep", "thorough"]
}
```

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Presets

Presets configure index depth, scoring strategy, and budget in one flag.

| Preset | Index | Scoring | Max Bytes | Min Score | Use Case |
|--------|-------|---------|-----------|-----------|----------|
| `fast` | Shallow | Heuristic only | 50 KB | 0.05 | Quick lookups |
| `balanced` | Deep (cached) | BM25F + heuristic | 100 KB | 0.01 | **Default — recommended** |
| `deep` | Deep (fresh) | BM25F + heuristic + PageRank (RRF) | 200 KB | 0.005 | Thorough analysis |
| `thorough` | Deep + all signals | BM25F + heuristic + PageRank + git recency (RRF) | 500 KB | 0.001 | Maximum relevance |

Explicit flags override preset values:

```bash
topo quick "auth" --preset fast --max-bytes 200000
```

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Scoring Engine

Topo combines multiple signals using Reciprocal Rank Fusion (RRF) to produce a single relevance score per file.

### Signals

| Signal | Weight | Description |
|--------|--------|-------------|
| **BM25F** | 60% | Field-weighted text relevance (filename 5x, symbols 3x, body 1x) |
| **Heuristic** | 40% | Path keywords, file role, depth penalty, well-known paths, file size |
| **Import graph** | RRF fusion | PageRank over import/require relationships (16 languages) |
| **Git recency** | structural | Commit frequency per file (90-day lookback) |
| **File role** | classification | Boosts impl, penalizes generated/vendor |

### How it works

1. **Scan** — Walk the repo respecting `.gitignore`, classify language and role
2. **Index** — Extract imports and compute PageRank scores at index time (stored in `.topo/index.bin`, zero query-time cost)
3. **Score** — BM25F content matching + heuristic path analysis, blended 60/40
4. **Fuse** — Structural signals (PageRank, git recency) combined with base ranking via RRF (`deep`/`thorough` presets)
5. **Budget** — Enforce `--max-bytes` / `--max-tokens`, greedily including top files
6. **Output** — Render as JSONL, JSON, compact, or human-readable table

### File roles

Topo classifies every file into a role that affects scoring:

| Role | Examples | Effect |
|------|----------|--------|
| `impl` | `src/main.rs`, `lib.py` | Boosted |
| `test` | `*_test.go`, `*.spec.ts` | Neutral |
| `config` | `.yaml`, `.env`, `.gitignore` | Slightly reduced |
| `docs` | `README.md`, `docs/` | Neutral |
| `build` | `Cargo.toml`, `Makefile` | Neutral |
| `generated` | `vendor/`, `node_modules/`, `*.pb.go` | Heavily penalized |

<p align="right">(<a href="#topo">back to top</a>)</p>

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
topo query "auth" --format json
```

Returns a single JSON object with all results.

### Human-readable (for terminals)

```bash
topo query "auth" --format human
```

Produces a formatted table. Auto-selected when stdout is a terminal.

### Compact (for hooks)

Minimal single-line-per-file format, designed for hook injection with minimal token overhead:

```
src/auth.rs (impl, 2494tok, 7.01)
src/commands/init.rs (impl, 2635tok, 6.92)
README.md (docs, 128tok, 6.54)
```

Auto-selected when `HOOK_EVENT_NAME` environment variable is set (Claude Code hooks set this). Use `--format compact` to select manually.

### Pipe detection

When stdout is not a TTY, Topo automatically switches to JSONL output and suppresses progress messages. When running inside a Claude Code hook, Topo auto-selects compact format. Override with `--format`.

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Deep Indexing

A deep index adds three capabilities on top of the shallow scan:

- **AST chunks** — Function, type, impl, and import declarations extracted per file with names and line ranges
- **Term frequencies** — Pre-computed word counts across filename, symbols, and body fields for BM25F scoring
- **PageRank scores** — Import graph built from source-level `import`/`use`/`require` statements, resolved to repo files via fuzzy file-stem matching, then scored with PageRank. Files imported by many others rank higher. Computed at index time for zero query-time cost.

Build one with:

```bash
topo index --deep
```

This creates `.topo/index.bin` in your repository root.

**Two-pass architecture:** Topo indexes thousands of files but typically selects ~30 for your context window. Parsing every file with a full AST is wasted work. Instead, indexing uses fast regex chunking to extract function names, types, and imports — the same data BM25F scoring consumes. Tree-sitter's 18 language grammars remain compiled and available for a future enrichment pass that deep-parses only the files that win scoring. This is the same pattern used by Sourcegraph (search-based vs precise navigation), IntelliJ (stub index vs full PSI), and rust-analyzer (lazy parsing). On Kubernetes (28k files), this cuts indexing time in half.

**Incremental updates:** When you re-run `topo index --deep`, only files whose SHA-256 has changed get re-indexed. Unchanged files carry forward from the existing index. File processing runs in parallel across all available cores via `rayon`.

**Supported languages for chunking (regex for indexing, tree-sitter for enrichment):**

| Language | Functions | Types | Imports | Impls |
|----------|-----------|-------|---------|-------|
| <img src="https://cdn.simpleicons.org/rust/DEA584" height="14" /> Rust | `fn` | `struct`, `enum`, `trait`, `type` | `use` | `impl` |
| <img src="https://cdn.simpleicons.org/go/00ADD8" height="14" /> Go | `func` | `type` | `import` | — |
| <img src="https://cdn.simpleicons.org/python/3776AB" height="14" /> Python | `def`, `async def` | `class` | `import`, `from` | — |
| <img src="https://cdn.simpleicons.org/javascript/F7DF1E" height="14" /> JavaScript | `function` | `class` | `import` | — |
| <img src="https://cdn.simpleicons.org/typescript/3178C6" height="14" /> TypeScript | `function` | `class`, `interface`, `type`, `enum` | `import` | — |
| <img src="https://cdn.simpleicons.org/openjdk/ED8B00" height="14" /> Java | methods | `class`, `interface`, `enum` | `import` | — |
| <img src="https://cdn.simpleicons.org/ruby/CC342D" height="14" /> Ruby | `def` | `class`, `module` | `require` | — |
| <img src="https://cdn.simpleicons.org/c/A8B9CC" height="14" /> C | functions | `struct`, `enum`, `union`, `typedef` | `#include` | — |
| <img src="https://cdn.simpleicons.org/cplusplus/00599C" height="14" /> C++ | functions | `class`, `struct`, `enum`, `namespace` | `#include` | — |
| <img src="https://cdn.simpleicons.org/gnubash/4EAA25" height="14" /> Shell | functions | — | — | — |
| <img src="https://cdn.simpleicons.org/swift/F05138" height="14" /> Swift | `func` | `class`, `struct`, `enum`, `protocol` | `import` | — |
| <img src="https://cdn.simpleicons.org/kotlin/7F52FF" height="14" /> Kotlin | `fun` | `class`, `object` | `import` | — |
| <img src="https://cdn.simpleicons.org/scala/DC322F" height="14" /> Scala | `def` | `class`, `trait`, `object` | `import` | — |
| <img src="https://cdn.simpleicons.org/haskell/5D4F85" height="14" /> Haskell | functions | `data`, `newtype`, `type`, `class` | `import` | — |
| <img src="https://cdn.simpleicons.org/elixir/4B275F" height="14" /> Elixir | `def` | — | — | — |
| <img src="https://cdn.simpleicons.org/lua/2C2D72" height="14" /> Lua | `function` | — | — | — |
| <img src="https://cdn.simpleicons.org/php/777BB4" height="14" /> PHP | functions | `class`, `interface`, `trait`, `enum` | `use` | — |
| <img src="https://cdn.simpleicons.org/r/276DC3" height="14" /> R | functions | — | — | — |

<p align="right">(<a href="#topo">back to top</a>)</p>

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
cargo bench -p topo-cli
```

### Polyglot and PageRank benchmarks

See **[BENCHMARKS.md](BENCHMARKS.md)** for detailed results across Kubernetes (28k Go files), Discourse (16k Ruby+JS files), and Mastodon (9k Ruby+TS files) — including before/after comparisons of PageRank scoring on polyglot repos.

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Architecture

Topo is a Cargo workspace with 7 focused crates:

```
                    +-------------+
                    |  CLI (clap) |
                    +------+------+
                           |
              +------------+------------+
              |            |            |
        +-----+-----+ +---+---+ +-----+-----+
        |  Scanner   | | Index | |  Scoring  |
        |  (ignore)  | | (rkyv)| |  Engine   |
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
| `topo-core` | Domain types, traits, errors, token budget |
| `topo-scanner` | File walking, gitignore, SHA-256 hashing |
| `topo-index` | Deep index builder, rkyv serialization, incremental merge |
| `topo-score` | BM25F, heuristic, hybrid, PageRank, git recency, RRF fusion |
| `topo-render` | JSONL v0.3, JSON, compact, human-readable output |
| `topo-treesit` | Code chunking (regex for indexing, tree-sitter for enrichment) |
| `topo-cli` | clap CLI, presets, commands |

### Built with

- [Rust](https://www.rust-lang.org) (2024 edition)
- [`clap`](https://docs.rs/clap) — CLI parsing
- [`ignore`](https://docs.rs/ignore) — Gitignore-respecting file walking (from ripgrep)
- [`tree-sitter`](https://docs.rs/tree-sitter) — AST-based code enrichment (18 language grammars, on-demand for selected files)
- [`rayon`](https://docs.rs/rayon) — Parallel file processing
- [`serde`](https://docs.rs/serde) + [`serde_json`](https://docs.rs/serde_json) — Serialization
- [`sha2`](https://docs.rs/sha2) — Content hashing

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Configuration Reference

### Global flags

| Flag | Default | Description |
|------|---------|-------------|
| `--root <path>` | `.` | Repository root (or set `TOPO_ROOT`) |
| `--format <fmt>` | `auto` | Output format: `auto`, `json`, `jsonl`, `human`, `compact` |
| `--no-color` | `false` | Disable color output |
| `-v` | `0` | Increase log verbosity (repeat for more) |
| `-q, --quiet` | `false` | Suppress non-essential output |

### Environment variables

| Variable | Description |
|----------|-------------|
| `TOPO_ROOT` | Default repository root path |
| `HOOK_EVENT_NAME` | Set by Claude Code hooks — auto-selects `compact` output format |

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Troubleshooting

| Problem | Cause | Fix |
|---------|-------|-----|
| Empty selection | No files matched the task | Broaden the task description or lower `--min-score` |
| Too many files selected | Budget too large | Use `--max-bytes` or `--top` to limit results |
| Stale results | Cached index from previous state | Run `topo index --force` to rebuild |
| Slow on large repos | First index builds from scratch | Subsequent runs use incremental updates |
| JSONL output in terminal | Pipe detection thinks stdout isn't a TTY | Use `--format human` explicitly |
| No deep index data | Ran `topo index` without `--deep` | Re-run with `--deep` flag |

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Contributing

Contributions are welcome. Topo follows these conventions:

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

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## Documentation

| Document | Description |
|----------|-------------|
| [BENCHMARKS](BENCHMARKS.md) | Performance and quality benchmarks across real-world repos |
| [PRD](docs/PRD.md) | Product requirements — what Topo is and who it's for |
| [SPEC](docs/SPEC.md) | Technical specification — architecture, data formats, APIs |
| [RESEARCH](docs/RESEARCH.md) | Rust migration analysis and crate evaluation |
| [DELIVERY](docs/DELIVERY.md) | Phased delivery plan with 42 issues across 8 phases |

<p align="right">(<a href="#topo">back to top</a>)</p>

---

## License

Distributed under the MIT License. See [`LICENSE`](LICENSE) for details.

---

<div align="center">

**[Report Bug](https://github.com/demwunz/topo/issues) · [Request Feature](https://github.com/demwunz/topo/issues)**

Made by [Fazal Khan](https://git.io/D)

</div>
