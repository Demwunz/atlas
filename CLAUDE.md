# Claude Instructions for Topo

## Project Overview

Topo is a Rust CLI that indexes codebases and selects the most relevant files for LLM context windows. It is a rewrite of [repo-context](https://github.com/demwunz/wobot) (Go).

## Canonical References

- **What to build**: [docs/PRD.md](docs/PRD.md)
- **How to build it**: [docs/SPEC.md](docs/SPEC.md)
- **Build order**: [docs/DELIVERY.md](docs/DELIVERY.md) — 42 issues across 8 phases
- **Research**: [docs/RESEARCH.md](docs/RESEARCH.md) — full Rust migration analysis
- **Issues**: https://github.com/demwunz/topo/issues
- **Project board**: https://github.com/users/Demwunz/projects/8

## Rust Conventions

- Edition: 2024
- `cargo clippy -- -D warnings` must pass
- `cargo fmt -- --check` must pass
- Prefer standard library over external crates when reasonable
- Error handling: `anyhow` for applications, `thiserror` for libraries
- No `unsafe` without justification
- No `unwrap()` in library code — use `?` or explicit error handling
- Tests live alongside source (`#[cfg(test)] mod tests`)
- Integration tests in `tests/` directory

## Crate Layout

```
crates/
├── topo-core/     (domain types, traits, errors)
├── topo-scanner/  (file walking, gitignore, hashing)
├── topo-index/    (deep index: chunks, rkyv serialization)
├── topo-score/    (BM25F, heuristic, structural, RRF fusion)
├── topo-render/   (JSONL v0.3, JSON, human output)
├── topo-treesit/  (tree-sitter integration, grammar loading)
└── topo-cli/      (clap CLI, presets, commands)
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `clap` (derive) | CLI parsing |
| `serde` + `serde_json` | Serialization |
| `ignore` | File walking (gitignore) |
| `rkyv` + `memmap2` | Zero-copy index |
| `tree-sitter` | AST chunking |
| `rayon` | Parallelism |
| `sha2` | Content hashing |
| `anyhow` | Error handling |

## Prompt Classification

Classify every user message before responding:

| Class | Rules |
|-------|-------|
| **EXPLORATION** | No code generation, no execution |
| **DECISION** | Structured output, declare risks, stop at planning |
| **EXECUTION** | Follow spec exactly, no scope expansion |

## Current Phase

**Phase 0: Repository Setup** — issues #1, #2, #3

Next action: Issue #1 — Initialize Cargo workspace with crate structure.
