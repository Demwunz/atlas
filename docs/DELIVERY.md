# Topo — Delivery Plan

## Overview

Topo is delivered in 8 phases, 42 issues total. Each phase builds on the previous one. Issues within a phase can often be parallelized.

Phases 0-2 establish the foundation: workspace, CI, scanner, and scoring engine.
Phases 3-4 add the deep index and full CLI.
Phase 5 adds tree-sitter AST chunking.
Phases 6-7 polish, ensure compatibility, and ship.

## Phase 0: Repository Setup (3 issues)

Foundation: Cargo workspace, CI, release infrastructure.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #1 | Initialize Cargo workspace with crate structure | S | P0 | — | `phase-0`, `size-S`, `P0` |
| #2 | CI pipeline: cargo check, clippy, test, fmt | M | P0 | #1 | `phase-0`, `size-M`, `P0` |
| #3 | cargo-dist release workflow (macOS, Linux) | M | P1 | #2 | `phase-0`, `size-M`, `P1` |

**Exit criteria:** `cargo check` passes on empty workspace. CI runs on every PR. Release workflow produces binaries.

## Phase 1: Foundation Types & Scanner (7 issues)

Core domain types and the file scanner — the foundation everything else builds on.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #4 | Core domain types (FileInfo, Bundle, ScoredFile, Language, FileRole) | M | P0 | #1 | `phase-1`, `size-M`, `P0` |
| #5 | File scanner with `ignore` crate (full gitignore support) | M | P0 | #4 | `phase-1`, `size-M`, `P0` |
| #6 | SHA-256 content hashing for incremental updates | S | P0 | #5 | `phase-1`, `size-S`, `P0` |
| #7 | JSONL v0.3 output writer (header/file/footer) | M | P0 | #4 | `phase-1`, `size-M`, `P0` |
| #8 | Fingerprint generation (deterministic repo identity) | S | P0 | #5 | `phase-1`, `size-S`, `P0` |
| #9 | Bundle builder (scan -> hash -> bundle) | M | P0 | #5, #6, #8 | `phase-1`, `size-M`, `P0` |
| #10 | Integration test: scan real directory, verify JSONL output | M | P1 | #7, #9 | `phase-1`, `size-M`, `P1` |

**Exit criteria:** Can scan a real repo directory, produce a Bundle, and write valid JSONL v0.3 output. SHA-256 hashes match for unchanged files.

## Phase 2: Scoring Engine (8 issues)

The scoring engine — BM25F, heuristic, and fusion. This is the core intelligence.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #11 | Tokenizer: whitespace + camelCase splitting + stopwords | S | P0 | #4 | `phase-2`, `size-S`, `P0` |
| #12 | BM25F scorer: field-weighted (filename 5x, symbols 3x, body 1x) | L | P0 | #11 | `phase-2`, `size-L`, `P0` |
| #13 | Heuristic scorer (path-based, keyword matching) | M | P0 | #4 | `phase-2`, `size-M`, `P0` |
| #14 | Hybrid scorer (weighted blend of BM25F + heuristic) | M | P0 | #12, #13 | `phase-2`, `size-M`, `P0` |
| #15 | Import graph builder + PageRank | L | P1 | #5 | `phase-2`, `size-L`, `P1` |
| #16 | Git recency signal (git log commit frequency per file) | M | P1 | #5 | `phase-2`, `size-M`, `P1` |
| #17 | File role classifier (test/impl/config/docs/generated) | S | P1 | #4 | `phase-2`, `size-S`, `P1` |
| #18 | RRF fusion: combine all signals into final ranking | M | P0 | #12, #13 | `phase-2`, `size-M`, `P0` |

**Exit criteria:** Can score files against a query using BM25F + heuristic with RRF fusion. Explain command shows per-signal breakdown.

## Phase 3: Deep Index (4 issues)

rkyv zero-copy index for instant loading and incremental updates.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #19 | Deep index data model (chunks, term freqs, doc lengths) | M | P0 | #4 | `phase-3`, `size-M`, `P0` |
| #20 | rkyv serialization + memmap2 zero-copy loading | L | P0 | #19 | `phase-3`, `size-L`, `P0` |
| #21 | Incremental deep index updates (SHA-256 change detection) | M | P0 | #19, #6 | `phase-3`, `size-M`, `P0` |
| #22 | Deep index build: parallel file processing with rayon | M | P0 | #19, #5 | `phase-3`, `size-M`, `P0` |

**Exit criteria:** Deep index builds in parallel, saves as rkyv binary, loads via memmap in <10ms. Incremental updates skip unchanged files.

## Phase 4: CLI & Commands (9 issues)

Full CLI with all commands and presets.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #23 | CLI skeleton with clap (derive) + global flags | M | P0 | #1 | `phase-4`, `size-M`, `P0` |
| #24 | `topo index` command (shallow + deep modes) | M | P0 | #9, #22, #23 | `phase-4`, `size-M`, `P0` |
| #25 | `topo query` command (score + select + budget) | M | P0 | #14, #18, #23 | `phase-4`, `size-M`, `P0` |
| #26 | `topo quick` command (preset orchestration) | M | P0 | #24, #25 | `phase-4`, `size-M`, `P0` |
| #27 | `topo render` command (JSONL -> formatted context) | M | P0 | #7, #23 | `phase-4`, `size-M`, `P0` |
| #28 | `topo explain` command (score breakdown per file) | S | P1 | #25 | `phase-4`, `size-S`, `P1` |
| #29 | `topo describe` command (machine-readable capabilities) | S | P1 | #23 | `phase-4`, `size-S`, `P1` |
| #30 | Preset system: fast / balanced / deep / thorough | M | P0 | #26 | `phase-4`, `size-M`, `P0` |
| #31 | Feature scopes: include/exclude globs from YAML | M | P1 | #5, #23 | `phase-4`, `size-M`, `P1` |

**Exit criteria:** All 6 commands work end-to-end. Presets configure scoring depth correctly. `topo quick "task" --preset balanced` produces correct JSONL output.

## Phase 5: tree-sitter AST Chunking (3 issues)

Language-aware chunking for precise symbol extraction.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #32 | tree-sitter integration: grammar loading + chunk extraction | L | P0 | #19 | `phase-5`, `size-L`, `P0` |
| #33 | Language grammars: Go, Rust, Python, JS/TS, Java, Ruby, C/C++ | M | P0 | #32 | `phase-5`, `size-M`, `P0` |
| #34 | Fallback: regex chunker for unsupported languages | S | P1 | #19 | `phase-5`, `size-S`, `P1` |

**Exit criteria:** tree-sitter correctly extracts functions and types for all 9 target languages. Fallback regex chunker handles other languages. Deep index includes accurate chunk data.

## Phase 6: Polish & Compatibility (4 issues)

Ensure compatibility with repo-context and add polish.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #35 | Token budget enforcement (--max-bytes, --max-tokens, --min-score) | M | P0 | #25 | `phase-6`, `size-M`, `P0` |
| #36 | Pipe detection + output mode (human / json / jsonl) | S | P1 | #23 | `phase-6`, `size-S`, `P1` |
| #37 | Compatibility test suite: match Go CLI output format exactly | L | P1 | #26, #27 | `phase-6`, `size-L`, `P1` |
| #38 | Benchmark harness: wobot + kubernetes comparison vs Go baseline | M | P1 | #26 | `phase-6`, `size-M`, `P1` |

**Exit criteria:** Token budgets enforced correctly. Output format matches repo-context exactly for same inputs. Benchmarks show expected performance improvements.

## Phase 7: Distribution & Integration (4 issues)

Ship it.

| Issue | Title | Size | Priority | Depends On | Labels |
|-------|-------|------|----------|------------|--------|
| #39 | cargo-dist GitHub Actions release workflow | M | P1 | #3 | `phase-7`, `size-M`, `P1` |
| #40 | Homebrew formula (demwunz/homebrew-tap) | S | P1 | #39 | `phase-7`, `size-S`, `P1` |
| #41 | Shell install script (curl \| sh) | S | P2 | #39 | `phase-7`, `size-S`, `P2` |
| #42 | Wobot toolchain resolver: detect topo binary | S | P1 | #26 | `phase-7`, `size-S`, `P1` |

**Exit criteria:** Release workflow produces macOS and Linux binaries. Homebrew tap works. Wobot detects and prefers topo over repo-context.

## Summary

| Phase | Issues | P0 | P1 | P2 | Sizes |
|-------|--------|----|----|-----|-------|
| 0: Setup | 3 | 2 | 1 | 0 | 1S + 2M |
| 1: Foundation | 7 | 6 | 1 | 0 | 2S + 5M |
| 2: Scoring | 8 | 5 | 3 | 0 | 2S + 4M + 2L |
| 3: Deep Index | 4 | 4 | 0 | 0 | 3M + 1L |
| 4: CLI | 9 | 6 | 3 | 0 | 2S + 7M |
| 5: tree-sitter | 3 | 2 | 1 | 0 | 1S + 1M + 1L |
| 6: Polish | 4 | 1 | 3 | 0 | 1S + 2M + 1L |
| 7: Distribution | 4 | 0 | 3 | 1 | 3S + 1M |
| **Total** | **42** | **26** | **15** | **1** | **12S + 24M + 5L** |

## Critical Path

The longest dependency chain determines the minimum calendar time:

```
#1 -> #4 -> #5 -> #6 -> #9 -> #22 -> #24 -> #26 -> #30
      |          |              |
      #11 -> #12 -> #14 -> #18 -> #25 -> #35
                                    |
                                   #37
```

Phases 0-2 can be partially parallelized (scoring engine doesn't depend on scanner completion).
Phase 5 (tree-sitter) can start as soon as #19 (deep index data model) is done.
Phase 7 (distribution) can start as soon as #3 (cargo-dist setup) and #26 (quick command) are done.
