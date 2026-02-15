# Atlas: Rust Migration Research

> Research conducted February 14-15, 2026. This document preserves the full analysis from plan mode sessions investigating whether repo-context (a Go codebase indexer) should be rewritten in Rust. The conclusion: **rewrite in Rust as "Atlas"**.

---

## Table of Contents

1. [Background: Go Performance Baseline](#background-go-performance-baseline)
2. [Go-Only Optimization Analysis](#go-only-optimization-analysis)
3. [Go-to-Rust Migration Analysis (13 Dimensions)](#go-to-rust-migration-analysis-13-dimensions)
4. [Scoring and Ranking Algorithms Beyond BM25](#scoring-and-ranking-algorithms-beyond-bm25)
5. [Rust Alternatives to LiteLLM (Python-Free LLM/Embedding Access)](#rust-alternatives-to-litelm-python-free-llmembedding-access)
6. [Final Recommendation and Architecture Decisions](#final-recommendation-and-architecture-decisions)

---

## Background: Go Performance Baseline

### Current Performance Measurements (wobot repo, 356 files, 66MB)

| Operation | Time | CPU% | Notes |
|-----------|------|------|-------|
| Shallow index | 98ms | ~100% | Path-only metadata |
| Deep index (cold) | 5.0s | 128% | AST chunking + TF computation for all 356 files |
| Fast preset (quick) | 67ms | 99% | Shallow index + heuristic scoring (cached) |
| Balanced preset (quick) | 4.9s | 119% | Deep index + hybrid BM25 scoring |
| Hybrid query (cached deep index) | 4.9s | 122% | The bottleneck is loading the 209MB deep.json |

Key observation: the balanced/deep path takes ~5 seconds and the bottleneck is the 209MB deep index JSON file (load + parse), not the BM25 math itself.

### Codebase Profile

| Metric | Value |
|--------|-------|
| Total Go LOC | 8,739 (4,689 non-test) |
| Binary size | 9.9 MB |
| External deps | 1 (`gopkg.in/yaml.v3`) |
| CGo usage | None |
| Concurrency | None (fully single-threaded) |
| Go `Benchmark` functions | 0 -- no microbenchmarks exist |
| Go version | 1.21 |

### Where Time Is Spent

1. **Deep index build** (~5s): Reading all 356 files from disk, tokenizing each, AST-chunking Go files, regex-chunking others, writing 209MB JSON
2. **Deep index load** (~4s): `os.ReadFile` + `json.Unmarshal` on 209MB `deep.json`
3. **BM25 scoring** (<100ms): Pure in-memory map lookups + floating-point math -- trivially fast once data is in memory
4. **Heuristic scoring** (<10ms): String matching on file paths
5. **Sorting** (selector.go): Bubble sort on scored results (O(n^2)) -- works fine at n=356 but would hurt at scale

---

## Go-Only Optimization Analysis

Before considering Rust, we analyzed whether Go-level optimizations could solve the performance issues. These were implemented and benchmarked.

### What Would Actually Make repo-context Faster (in Go)

1. **Switch deep index from JSON to a binary format** (MessagePack/gob) -- expected 3-5x faster load, 2-3x smaller file
2. **Add goroutine parallelism to deep index build** -- file reads + tokenization are embarrassingly parallel. `sync.WaitGroup` + bounded worker pool would cut deep build time by 2-4x on multi-core
3. **Replace bubble sort with `sort.Slice`** -- already imported but not used in selector.go's hot path (lines 39-45, 83-90)
4. **Add Go `Benchmark` functions** -- zero microbenchmarks exist. Can't optimize what you can't measure.
5. **Lazy deep index loading** -- only load TF data for files that pass heuristic pre-filter, not all 356 files

### Expected vs Actual Impact

| Optimization | Before | Expected | Actual | Effort |
|--------------|--------|----------|--------|--------|
| Binary format (gob) for deep index | 4.9s query | ~1-2s | 2.04s | Medium |
| Parallel deep index build | 5.0s build | ~1.5-2s | 1.1s | Small |
| Lazy TF loading | loads 209MB | loads ~20MB | Not implemented | Medium |
| sort.Slice instead of bubble sort | O(n^2) | O(n log n) | Trivial, measured | Trivial |
| Combined | ~5s | <1s | ~2s balanced query | ~1 day |

### Post-Optimization Benchmarks (M2 Max, 12 cores)

#### wobot repo (358 files, 63MB)

- **Deep index format**: gob 92MB vs JSON 200MB (54% smaller)
- **Deep index load**: gob 244ms vs JSON 1,791ms (7.3x faster)
- **Deep index save**: gob 183ms vs JSON 1,095ms (6.0x faster)
- **Deep index build**: 1.1s parallel (485% CPU) vs ~5.0s sequential (4.5x faster)
- **Balanced query e2e**: 2.04s (gob) vs 3.57s (JSON) (43% faster)
- **Fast query e2e**: 0.33s (shallow index, heuristic)
- **Scoring**: heuristic 52us, BM25+deep 163us, BM25 from disk 2,760ms (17,000x slower)
- **Full pipeline** (score + sort + budget): 206us for 50 selected files

#### kubernetes/kubernetes (28,027 files, 239MB)

- **Deep index format**: gob 186MB vs JSON 230MB (19% smaller)
- **Deep index load**: gob 498ms vs JSON 2,246ms (4.5x faster)
- **Deep index save**: gob 429ms vs JSON 1,251ms (2.9x faster)
- **Deep index build**: 4.4s parallel (78x files, only 4x time)
- **Balanced query e2e**: 2.40s (only 18% slower than wobot despite 78x more files)
- **Fast query e2e**: 1.72s
- **Scoring**: heuristic 9.4ms (26k files), BM25+deep 46ms (14k files), full pipeline 57ms (50 selected)

#### Scaling Factors (wobot to kubernetes, 78x files)

| Metric | Factor |
|--------|--------|
| Deep build | 4x |
| Gob size | 2x |
| Gob load | 2x |
| BM25+deep | 282x |
| Full pipeline | 277x |
| Balanced e2e | 1.2x |

Key insight: end-to-end time is dominated by gob deserialization, which scales sub-linearly.

### Verdict on Go Optimizations

The Go optimizations delivered significant speedup (5s to 2s for balanced queries). However, the remaining bottleneck -- gob deserialization at 244-498ms -- is a fundamental limitation of Go's serialization model. Rust's rkyv zero-copy approach eliminates this entirely (<5ms projected).

---

## Go-to-Rust Migration Analysis (13 Dimensions)

### 1. Binary Size Comparison

#### Real-world Rust CLI binary sizes (Arch Linux x86_64, dynamically linked, stripped)

| Tool | Installed Size | Package (compressed) | What it replaces |
|------|---------------|---------------------|------------------|
| ripgrep | 5.1 MB | 1.5 MB | grep |
| fd | 3.9 MB | 1.3 MB | find |
| bat | 5.8 MB | 2.5 MB | cat |
| delta | 6.1 MB | 2.4 MB | diff-so-fancy |
| eza | 1.5 MB | 579 KB | ls |

Sources: [Arch ripgrep](https://archlinux.org/packages/extra/x86_64/ripgrep/), [Arch fd](https://archlinux.org/packages/extra/x86_64/fd/), [Arch bat](https://archlinux.org/packages/extra/x86_64/bat/), [Arch delta](https://archlinux.org/packages/extra/x86_64/git-delta/), [Arch eza](https://archlinux.org/packages/extra/x86_64/eza/)

#### Go baseline

A Go "Hello, World!" compiles to ~1.6 MB (Go 1.7+). A typical Go CLI with a couple of modules is ~11 MB; with `go build -ldflags "-s -w"` (strip debug + DWARF) it drops to ~7.7 MB. repo-context at 10 MB is consistent with this pattern.

Source: [Why Go binaries are large](https://www.codestudy.net/blog/how-to-reduce-go-compiled-file-size/), [Go binary sizes growing](https://donatstudios.com/Golang-Binary-Sizes)

#### Rust optimization techniques and impact

Using the [size-optimization-rust-binaries](https://github.com/Rust-Trends/size-optimization-rust-binaries) benchmark:

| Configuration | Size | Reduction |
|---|---|---|
| Default `--release` | 1.2 MB | baseline |
| `strip` + `opt-level=z` | 804 KB | -33% |
| + `lto=true` | 668 KB | -44% |
| + `codegen-units=1` | 652 KB | -46% |

Real-world examples: An 18 MB Rust binary was cut to 7 MB with zero dependency changes. An 8.1 MB binary went to 3.0 MB with `panic=abort` + `opt-level=z` + `lto` + `codegen-units=1` + `strip`.

**Realistic estimate for repo-context in Rust**: 2-4 MB stripped (vs 10 MB Go), potentially under 2 MB with aggressive optimization. If you match ripgrep's complexity profile, expect ~5 MB installed.

Sources: [min-sized-rust](https://github.com/johnthagen/min-sized-rust), [Rust binary size optimization 2025](https://markaicode.com/binary-size-optimization-techniques/), [Rust Project Primer](https://rustprojectprimer.com/building/size.html)

---

### 2. Serde vs encoding/gob

#### Rust serialization benchmarks (from [rust_serialization_benchmark](https://github.com/djkoloski/rust_serialization_benchmark))

On the **Log dataset** (representative of structured data like the deep index):

| Framework | Version | Serialize | Deserialize |
|---|---|---|---|
| rkyv | 0.8.10 | 249.15 us | 1.54 ms (zero-copy) |
| bincode | 2.0.1 | 382.28 us | 2.35 ms |
| postcard | 1.1.1 | 430.08 us | 2.24 ms |

On the **Mesh dataset** (larger structures):

| Framework | Serialize | Deserialize |
|---|---|---|
| rkyv | 1.12 ms | 1.96 ms |
| bincode | 7.03 ms | N/A |

#### Go encoding/gob performance

Go gob encoding runs at approximately 134,500-176,500 ns/op for small structs. Gob is designed for streams of a single type, not individual values, so single-value benchmarks understate its batch performance. However, gob includes significant overhead for its self-describing format.

Source: [Go serialization benchmarks](https://github.com/alecthomas/go_serialization_benchmarks), [Gob vs JSON benchmark](https://rsheremeta.medium.com/benchmarking-gob-vs-json-xml-yaml-48b090b097e8)

#### Realistic speedup estimate

- **bincode**: 2-5x faster than gob for typical struct serialization, dramatically smaller wire format
- **rkyv** (zero-copy): 10-50x faster for deserialization because it accesses data in-place without copying
- **postcard**: Similar to bincode but 70% of the wire size, designed for embedded/constrained environments

For the deep index (pre-computed chunks + term frequencies), **rkyv** is the strongest choice: zero-copy deserialization means loading the index is essentially free (mmap + pointer cast).

Source: [rkyv is faster than everything](https://david.kolo.ski/blog/rkyv-is-faster-than/), [Rust serialization benchmark](https://david.kolo.ski/rust_serialization_benchmark/)

---

### 3. Tree-sitter for AST Chunking

#### Capabilities vs regex

Tree-sitter provides incremental, error-tolerant parsing with full AST output. It supports 100+ language grammars. Incremental re-parsing after edits takes sub-millisecond. Initial full parse is 2-3x slower than hand-written parsers (like rustc's), but produces a complete CST.

For the repo-context use case (extracting function/type boundaries from source files), tree-sitter gives:
- Exact symbol boundaries (vs regex heuristics that miss edge cases)
- Language-agnostic API (one code path vs per-language regex patterns)
- Nested structure awareness (closures, inner functions, etc.)

#### Binary size cost

Each tree-sitter grammar compiles from JavaScript to a large C file (~3.7 MB for C grammar source). The compiled `.so` per grammar is approximately 0.5-1.5 MB. A WASM build of the JavaScript grammar is ~616 KB; the tree-sitter runtime itself is ~252 KB in WASM.

For 10 languages statically linked: estimate **5-15 MB** additional binary size. This is significant.

#### Dynamic loading

Tree-sitter supports WASM-based dynamic grammar loading via `WasmStore`. Grammars can be distributed as separate `.wasm` files (~200-600 KB each) and loaded at runtime, keeping the core binary small. This is the approach used by editors like Zed and Helix.

Source: [tree-sitter docs](https://tree-sitter.github.io/tree-sitter/using-parsers/1-getting-started.html), [tree-sitter WASM optimization](https://github.com/tree-sitter/tree-sitter/issues/410), [Using tree-sitter parsers in Rust](https://rfdonnelly.github.io/posts/using-tree-sitter-parsers-in-rust/)

---

### 4. Tantivy for BM25

#### Does tantivy provide BM25?

Yes. Tantivy uses BM25 scoring (same as Lucene) for relevance ranking. The implementation is in [`bm25.rs`](https://docs.rs/tantivy/latest/src/tantivy/query/bm25.rs.html) and supports configurable `Bm25StatisticsProvider`.

#### Performance

Tantivy is ~2x faster than Lucene in search latency benchmarks and starts up in under 10 ms. It uses FSTs (Finite State Transducers) for term dictionary storage and sophisticated integer compression.

Source: [tantivy GitHub](https://github.com/quickwit-oss/tantivy), [search-benchmark-game](https://github.com/turbopuffer/search-benchmark-game)

#### Is it overkill for in-memory scoring?

**Likely yes for this use case.** Tantivy is a full inverted-index search engine with disk-based segments, merging, and concurrent readers. For in-memory BM25 scoring of a pre-computed term frequency index, a lightweight alternative is better:

- **[`bm25` crate](https://crates.io/crates/bm25)**: A dedicated lightweight in-memory BM25 embedder, scorer, and search engine. Supports multilingual tokenization with stemming and stop words. Minimal dependencies. This is a much better fit.
- **[`probly-search`](https://crates.io/crates/probly-search)**: Full-text search with BM25 ranking, gives you full control over scoring.
- **Hand-rolled BM25**: The algorithm is ~30 lines of Rust. If you already have term frequencies pre-computed (as repo-context does), a hand-rolled scorer is trivial and zero-dependency.

**Recommendation**: Start with the `bm25` crate or hand-roll. Only reach for tantivy if you need persistent indexes, phrase queries, or fuzzy matching.

Source: [bm25 crate](https://github.com/michael-jb/bm25), [tantivy architecture](https://github.com/quickwit-oss/tantivy/blob/main/ARCHITECTURE.md)

---

### 5. Cross-Compilation

#### Go: GOOS/GOARCH

Go's cross-compilation is famously trivial:
```bash
GOOS=linux GOARCH=amd64 go build
GOOS=darwin GOARCH=arm64 go build
GOOS=windows GOARCH=amd64 go build
```
No additional toolchain installation needed. Pure Go code works out of the box. CGo disables this simplicity.

#### Rust: `cross` and `rustup target add`

Rust requires installing target-specific standard libraries (`rustup target add x86_64-unknown-linux-musl`) plus a cross-linker. The **[`cross`](https://github.com/cross-rs/cross)** tool simplifies this by using Docker containers with pre-configured toolchains:
```bash
cross build --target x86_64-unknown-linux-musl --release
cross build --target aarch64-unknown-linux-gnu --release
```

Alternatively, **cargo-zigbuild** uses Zig as a cross-compiler (no Docker needed), which is what GoReleaser uses for Rust projects.

#### cargo-dist vs goreleaser

| Feature | cargo-dist | goreleaser |
|---------|-----------|------------|
| Language focus | Rust-native | Go-native, Rust/Zig added |
| Config location | `Cargo.toml` metadata | `.goreleaser.yaml` |
| CI generation | Auto-generates `release.yml` | Manual or template |
| Installers | Shell, PowerShell, Homebrew, MSI, npm | Archives, Homebrew, Snap, Docker, Scoop, AUR |
| cargo-binstall | Automatic integration | N/A |
| Setup | `dist init` | `goreleaser init` |
| Maturity | Newer (~2023) | Established (~2017) |

**Verdict**: Go wins on cross-compilation simplicity. Rust's `cross` + `cargo-dist` achieves parity in CI automation but requires more initial setup. GoReleaser now supports Rust, so you could even use GoReleaser for the Rust binary.

Source: [GoReleaser Rust support](https://goreleaser.com/customization/builds/rust/), [cargo-dist](https://axodotdev.github.io/cargo-dist/), [cross-compilation in Rust](https://fpira.com/blog/2025/01/cross-compilation-in-rust/)

---

### 6. Compilation Time

#### Clean build estimates for a 5-10k line Rust project

Key bottlenecks for the dependency set:
- **clap** (derive): ~11 seconds alone, ~81 transitive dependencies
- **serde** (derive): proc-macro compilation is a bottleneck, limits CPU utilization
- **regex**: regex-syntax is ~52K lines, regex-automata ~40K lines
- **tree-sitter**: C code compilation (grammar-dependent)

**Realistic clean `--release` build time**: 30-90 seconds on a modern machine (Apple M-series or recent x86_64). Debug builds: 15-45 seconds.

**Incremental builds** (after initial): 2-5 seconds for changes to your code only (dependencies cached).

#### 2025 improvements

- Rust compiler got 6x speed improvements in 2025 through parallel front-end compilation
- Cranelift backend (becoming production-ready for 2025H2) offers ~20% faster code generation for debug builds
- Parallel front-end delivers 20-30% faster builds

**Comparison**: Go compiles ~5K lines in 1-3 seconds. Rust's clean build is 10-30x slower. Incremental Rust builds narrow the gap to 2-5x.

Source: [Tips for faster Rust compile times](https://corrode.dev/blog/tips-for-faster-rust-compile-times/), [clap compile time issue](https://github.com/clap-rs/clap/issues/2037), [serde compile time issue](https://github.com/serde-rs/serde/issues/1146)

---

### 7. WASM Target

#### Maturity

- **wasm32-wasip1**: Tier 2 target, tested in CI, ships with rustup. Production-ready.
- **wasm32-wasip2**: Supported since Rust 1.82 (late 2024). Tier 2.
- **wasm32-wasip3**: Not yet building as of 2025-10-01.

#### Size implications

- A Rust WASM binary can be under 1 MB with optimization
- `wasm-opt` reduces WASM binary size by 10-20% additionally
- Default allocator adds ~10 KB; can be replaced with `wee_alloc` for smaller footprint
- `--release` mode drastically reduces WASM output vs debug

#### Feasibility for repo-context

A WASM-compiled repo-context could run in:
- **Browser**: Via web-tree-sitter + WASI filesystem shims (experimental)
- **Edge/serverless**: Cloudflare Workers, Fastly Compute, Fermyon Spin all support WASI
- **CLI via Wasmtime/Wasmer**: Functional but ~2-5x slower than native

**Go comparison**: Go can compile to WASM but the runtime (GC, goroutine scheduler) adds significant overhead (~2-5 MB minimum). Rust WASM binaries are fundamentally smaller and faster because there is no runtime to embed.

Source: [Rust WASI targets](https://blog.rust-lang.org/2024/04/09/updates-to-rusts-wasi-targets.html), [Shrinking WASM](https://rustwasm.github.io/book/reference/code-size.html), [Fermyon Rust WASM](https://developer.fermyon.com/wasm-languages/rust)

---

### 8. Memory Usage

#### Rust vs Go for large codebases

- In a 2025 JSON processing benchmark (1M documents, 100 fields, 1000 concurrent users), Rust achieved 1.5x faster throughput and 20% lower memory usage than Go
- Go's GC typically consumes ~10% of processing time
- Rust's ownership model eliminates GC pauses entirely; memory usage is deterministic
- For 1 GB log file processing, both languages finish in ~2 seconds, but Rust uses less peak memory

#### Estimate for 28K files / 239 MB repo

The workload (directory traversal + file reading + tokenization + BM25 scoring) is allocation-heavy. Key differences:
- **Go**: Each `os.ReadFile` allocates; GC pressure scales with file count. Peak RSS likely 200-400 MB for 239 MB repo with in-memory index.
- **Rust**: `mmap` or streaming reads avoid allocation. With rkyv zero-copy index, RSS could be close to file sizes on disk. Estimate 50-150 MB peak.

Source: [Rust vs Go 2025](https://blog.jetbrains.com/rust/2025/06/12/rust-vs-go/), [benchmarks game](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/rust-go.html)

---

### 9. Startup Time

#### Benchmark data (from startup-time)

| Language | Intel i5 | Raspberry Pi 3 |
|----------|----------|----------------|
| Rust (rustc 1.21) | 0.51 ms | 4.42 ms |
| Go (go 1.8) | 0.41 ms | 4.10 ms |

Both are in the "Fast" category. Go is marginally faster (~0.1 ms) on desktop due to simpler binary layout.

#### Real-world CLI startup

For actual CLI tools with argument parsing and config loading:
- Rust CLIs (ripgrep, fd): typically 1-5 ms startup
- Go CLIs: typically 2-10 ms startup (runtime initialization + GC setup)
- AWS Lambda: Rust cold start ~30 ms, Go cold start ~45 ms

**Verdict**: Startup time is a wash for CLI tools. Both are sub-10 ms. Neither will be user-perceptible.

Source: [startup-time benchmark](https://github.com/bdrung/startup-time), [Lambda cold starts](https://maxday.github.io/lambda-perf/)

---

### 10. Dependency Count

#### Estimated transitive dependencies for the Rust port

| Direct crate | Approximate transitive deps | Notes |
|---|---|---|
| `clap` (derive) | ~81 crates | Heavy proc-macro tree |
| `serde` + `serde_derive` | ~15 crates | proc-macro overhead |
| `walkdir` | ~3 crates | minimal |
| `ignore` | ~30 crates (includes regex) | from ripgrep |
| `regex` | ~5 crates | regex-syntax + regex-automata |
| `tree-sitter` + grammars | ~5-10 per grammar | C code compiled via cc |
| `bincode` or `rkyv` | ~5-15 crates | rkyv has more |
| `serde_yml` | ~5 crates | yaml-rust2 based |

**Total estimate**: ~150-200 transitive crate dependencies.

**Go comparison**: The current Go binary has 1 external dependency. This is a massive delta. However, Rust's crate dependencies are:
- Compiled from source (auditable)
- Statically linked (no DLL hell)
- Managed by Cargo.lock (reproducible)
- Auditable via `cargo audit` / `cargo vet`

**Mitigation**: Use `clap` in builder mode (not derive) to drop proc-macro deps. Use `ignore` (which bundles regex) instead of separate regex crate. Consider `lexopt` or `pico-args` for minimal argument parsing (~0 deps).

Source: [Removing dependency bloat](https://acha.ninja/blog/removing_rust_dependency_bloat/), [Managing Rust deps](https://notgull.net/rust-dependencies/), [Effective Rust dep graph](https://effective-rust.com/dep-graph.html)

---

### 11. Notable Rust Rewrites

| Rust tool | Replaces | Key outcome |
|---|---|---|
| ripgrep | grep, ag, git-grep | 2-10x faster than GNU grep; wins on Unicode support; parallel by default |
| fd | find | Order of magnitude faster for common use cases; sane defaults; respects .gitignore |
| bat | cat | Syntax highlighting, git integration, line numbers; not a performance play |
| delta | diff-so-fancy | Syntax-highlighted diffs, side-by-side, line numbers |
| eza | ls, exa | Git status integration, icons, tree view; smallest binary (1.5 MB) |

The key pattern: these tools are not just faster -- they provide better defaults, better Unicode handling, and parallelism out of the box. Performance is a feature, not the only selling point.

Source: [ripgrep benchmarks](https://burntsushi.net/ripgrep/), [ripgrep GitHub](https://github.com/BurntSushi/ripgrep)

---

### 12. cargo-binstall / cargo-dist

#### cargo-dist

- Purpose: End-to-end release automation for Rust projects
- Run `dist init` to generate config in `Cargo.toml` and a `release.yml` GitHub Actions workflow
- Generates: tarballs (.tar.xz, .zip), shell installers, PowerShell installers, Homebrew formulae, MSI installers, npm packages
- Automatic cargo-binstall integration (users can `cargo binstall your-tool` and get prebuilt binaries)
- Tag-triggered: push a git tag, CI handles everything

#### cargo-binstall

- Drop-in replacement for `cargo install` that downloads pre-built binaries
- Checks GitHub Releases first, falls back to quickinstall, then `cargo install`
- v1.0 released; supports `--git` for installing from repositories
- If you use cargo-dist, binstall integration is automatic

#### vs goreleaser

GoReleaser is more mature (2017 vs 2023), supports more packaging targets (Docker, Snap, AUR, Scoop), and now handles Rust via cargo-zigbuild. cargo-dist is Rust-native, generates its own CI, and integrates with cargo-binstall. For a Rust-only project, cargo-dist is the natural choice.

Source: [cargo-dist](https://axodotdev.github.io/cargo-dist/), [cargo-binstall](https://github.com/cargo-bins/cargo-binstall), [GoReleaser Rust](https://goreleaser.com/blog/rust-zig/), [Automated Rust releases](https://blog.orhun.dev/automated-rust-releases/)

---

### 13. Rust Ecosystem: Required Crates

#### Core

| Crate | Purpose | Downloads | Notes |
|---|---|---|---|
| [`clap`](https://crates.io/crates/clap) | CLI argument parsing | 300M+ | Use derive or builder mode |
| [`serde`](https://crates.io/crates/serde) + `serde_derive` | Serialization framework | 500M+ | Universal derive macros |
| [`serde_json`](https://crates.io/crates/serde_json) | JSON (JSONL output) | 400M+ | |
| [`serde_yml`](https://github.com/sebastienrousseau/serde_yml) | YAML parsing | Active fork | Replaces deprecated `serde_yaml` |

#### File walking + gitignore

| Crate | Purpose | Notes |
|---|---|---|
| [`ignore`](https://crates.io/crates/ignore) | Dir walker with .gitignore, parallel | From ripgrep; includes `walkdir` + `regex` |
| [`walkdir`](https://crates.io/crates/walkdir) | Simple recursive dir walker | Only if not using `ignore` |

#### Scoring + search

| Crate | Purpose | Notes |
|---|---|---|
| [`bm25`](https://crates.io/crates/bm25) | In-memory BM25 scoring | Lightweight, multilingual tokenizer |
| [`regex`](https://crates.io/crates/regex) | Pattern matching | Included transitively via `ignore` |

#### AST chunking

| Crate | Purpose | Notes |
|---|---|---|
| [`tree-sitter`](https://crates.io/crates/tree-sitter) | Parser runtime | Core library |
| `tree-sitter-{lang}` | Per-language grammars | `tree-sitter-rust`, `tree-sitter-python`, `tree-sitter-javascript`, etc. |

#### Serialization (index format)

| Crate | Purpose | Notes |
|---|---|---|
| [`rkyv`](https://crates.io/crates/rkyv) | Zero-copy deserialization | Best for index loading performance |
| [`bincode`](https://crates.io/crates/bincode) | Compact binary format | Simpler API than rkyv |

#### Utilities

| Crate | Purpose | Notes |
|---|---|---|
| [`sha2`](https://crates.io/crates/sha2) | SHA-256 hashing | For incremental index updates |
| [`rayon`](https://crates.io/crates/rayon) | Data parallelism | Parallel file processing |
| [`memmap2`](https://crates.io/crates/memmap2) | Memory-mapped files | For large file/index access |
| [`anyhow`](https://crates.io/crates/anyhow) | Error handling | Or `eyre` for richer errors |
| [`tracing`](https://crates.io/crates/tracing) | Structured logging | Industry standard |

#### Minimal alternative (fewer deps)

If dependency count is a concern, a minimal stack would be:
- `pico-args` or `lexopt` (0-dep argument parsing) instead of `clap`
- `serde` + `serde_json` (unavoidable for JSONL)
- `ignore` (bundles walkdir + regex)
- Hand-rolled BM25 (~30 lines) instead of `bm25` crate
- Regex-based chunking (current approach) instead of tree-sitter
- `bincode` instead of `rkyv` (simpler)

This minimal stack would have ~50-80 transitive dependencies instead of ~150-200.

Sources: [ignore crate](https://docs.rs/ignore), [serde_yml](https://github.com/sebastienrousseau/serde_yml), [bm25 crate](https://github.com/michael-jb/bm25), [rkyv](https://rkyv.org/)

---

### Summary Scorecard

| Dimension | Go (current) | Rust (projected) | Winner |
|---|---|---|---|
| Binary size | ~10 MB | 2-5 MB | **Rust** (2-5x smaller) |
| Serialization | encoding/gob ~135-177 us | rkyv ~249 us serialize, zero-copy deser | **Rust** (10-50x deser) |
| BM25 | Hand-rolled | `bm25` crate or hand-rolled | Tie |
| AST chunking | Regex (8 langs) | tree-sitter (100+ langs) | **Rust** (ecosystem) |
| Cross-compilation | `GOOS=x GOARCH=y` | `cross` + Docker/Zig | **Go** (simpler) |
| Compile time | 1-3 sec | 30-90 sec clean, 2-5 sec incremental | **Go** (10-30x faster clean) |
| WASM | Possible but heavy | wasm32-wasip1/p2, sub-1MB | **Rust** |
| Memory | ~200-400 MB peak (est.) | ~50-150 MB peak (est.) | **Rust** (2-4x less) |
| Startup | ~2-10 ms | ~1-5 ms | Tie |
| Dependencies | 1 external | ~150-200 transitive crates | **Go** (dramatically simpler) |
| Distribution | goreleaser | cargo-dist + cargo-binstall | Tie (both mature) |

---

## Scoring and Ranking Algorithms Beyond BM25

### 1. BM25 Variants

**BM25+ (Lv & Zhai, 2011):** Adds a lower-bound constant (delta, typically 1.0) to the term frequency normalization. This prevents long documents that contain query terms from being scored similarly to short documents that do not. In benchmarks on TREC collections, BM25+ shows marginal improvement over standard BM25, particularly on collections with high document length variance. One medical retrieval study reported P@10 of 0.297 for BM25+ vs 0.298 for BM25L.

**BM25L (Lv & Zhai, 2011):** Designed specifically for long document retrieval. It "shifts" the TF normalization by adding a constant to prevent over-penalization of long documents. In the same medical study, BM25L achieved nDCG@10 of 0.433 vs 0.426 for BM25+. BM25L is the strongest variant when your corpus has wide length variation (which code files do -- config files are 10 lines, main modules are 1000+).

**BM25F (Robertson et al., 2004):** The most relevant variant for code search. Treats a document as composed of multiple fields with per-field boost weights. **Sourcegraph's Zoekt uses BM25F** with three fields: filename, symbol definitions, and file content. Their implementation applies a **5x boost** to term frequencies in filename and symbol fields before computing the unified BM25 score. This delivered **~20% improvement** in search quality metrics. The boost value was selected via grid search and proved "not too sensitive to the exact choice." They also compute line-level BM25F for within-file ranking.

**Practical verdict:** BM25F is the variant worth pursuing for code search. BM25+ and BM25L offer marginal gains. A large-scale reproducibility study (Kamphuis et al., 2020) found that "once trained there is very little difference in performance between these functions."

Sources:
- [Which BM25 Do You Mean? A Large-Scale Reproducibility Study (PMC)](https://pmc.ncbi.nlm.nih.gov/articles/PMC7148026/)
- [BM25L: Enhancing Long Document Retrieval](https://www.academia.edu/2785747/When_documents_are_very_long_BM25_fails_)
- [Keeping it boring (and relevant) with BM25F - Sourcegraph](https://sourcegraph.com/blog/keeping-it-boring-and-relevant-with-bm25f)
- [Okapi BM25 - Wikipedia](https://en.wikipedia.org/wiki/Okapi_BM25)

---

### 2. TF-IDF Alternatives

**BM25 vs TF-IDF:** BM25 is strictly better than raw TF-IDF because of (a) term frequency saturation (parameter k1 prevents a term appearing 100 times from dominating), and (b) document length normalization (parameter b). repo-context already has BM25, so TF-IDF is a step backward.

**DFR (Divergence From Randomness):** A family of models in Apache Lucene/Terrier that computes term weights by measuring how much a term's actual distribution diverges from a random distribution. DFR is claimed to perform "definitely better than BM25 for short queries." However, Lucene's documentation notes that DFR implementations are "not optimized to the same extent as BM25Similarity" so expect slower performance. DFR requires selecting three components (BasicModel, AfterEffect, Normalization) -- many combinations, hard to tune.

**LM (Language Model) Scoring:** Jelinek-Mercer or Dirichlet smoothing language models. These estimate the probability of generating the query from a document's language model. Comparable performance to BM25 on standard benchmarks. Dirichlet smoothing works well for short documents. Available in Lucene.

**DFI (Divergence From Independence):** Said to give comparable results to BM25/DFR with less sensitivity to parameter tuning, making it an interesting "set and forget" alternative.

**Practical verdict for code search:** BM25 remains the best starting point. DFR is theoretically interesting but harder to tune and slower. LM-Dirichlet could be worth testing if documents (code files) are generally short.

Sources:
- [Search Relevance - Solr & Elasticsearch Similarities](https://sematext.com/blog/search-relevance-solr-elasticsearch-similarity/)
- [DFR Framework - Terrier](http://terrier.org/docs/v3.5/dfr_description.html)

---

### 3. Learned Sparse Retrieval

**SPLADE (Formal et al., SIGIR 2021/2022):** Uses a BERT-based encoder to produce sparse representations where each dimension corresponds to a vocabulary term. The encoder learns to "expand" documents with semantically related terms not present in the original text. The bi-encoder version requires neural inference on both queries and documents.

**SPLADE-doc (document-only variant):** This is the critical variant for this use case. SPLADE-doc applies neural expansion **only at index time** on documents. At query time, the query is treated as a plain bag-of-words -- **no neural inference, zero GPU at query time**. Performance characteristics:
- Query latency competitive with BM25 (10-100ms range on BEIR benchmarks)
- Index sizes 2-3x smaller than full SPLADE via thresholding
- Performance remains "competitive with state-of-the-art dense bi-encoders"
- Less effective than full SPLADE (where the query is also encoded), but the zero-query-cost tradeoff is compelling
- Pre-computation is a one-time cost at indexing

**DeepImpact (Mallia et al., 2021):** Expands documents via DocT5Query, then estimates a single scalar impact score per token per document using BERT. Fully pre-computed -- stored as sparse vectors. At query time, you just sum the impact scores of matching terms. No neural inference needed at query time.

**uniCOIL (Lin et al., 2021):** A degenerate version of COIL where contextualized vectors are collapsed to scalar weights. Pre-computes sparse vectors for all documents. At query time, retrieval uses a standard inverted index with pre-computed term weights -- no neural inference required.

**Practical verdict:** SPLADE-doc and uniCOIL are both viable for pre-computation at index time with bag-of-words queries at runtime. However, all of these require a one-time neural inference pass over every document during indexing, which means you need a BERT-class model available at index time (not query time). For a code search tool, this is acceptable if you can afford the initial indexing cost.

Sources:
- [SPLADE for Sparse Vector Search - Pinecone](https://www.pinecone.io/learn/splade/)
- [SPLADE v2 (arXiv 2109.10086)](https://arxiv.org/abs/2109.10086)
- [SPLADE-v3 (arXiv 2403.06789)](https://arxiv.org/pdf/2403.06789)
- [Modern Sparse Neural Retrieval - Qdrant](https://qdrant.tech/articles/modern-sparse-neural-retrieval/)

---

### 4. Code-Specific Scoring Signals

These are the structural signals that go beyond bag-of-words. All can be pre-computed at index time.

**Import Graph / Dependency Analysis (PageRank):** Sourcegraph applies a PageRank variant over the code symbol graph. A file's rank is based on inbound references from other files -- function calls, imports, type references. This measures "code reuse," which is a strong relevance signal: heavily-imported utility modules rank higher than dead code. The graph is built from their code intelligence platform (cross-repo symbol resolution). For a simpler implementation, build a file-level import graph from static analysis and compute PageRank on it.

**Symbol Reference Counting:** Count how many times each file's exported symbols (functions, types, constants) are referenced by other files. This is a simplified version of PageRank -- just in-degree rather than recursive importance. Cheaper to compute, still very useful. A file whose exports are referenced by 50 other files is more likely relevant than one referenced by 2.

**File Change Frequency (git log):** Files with more commits are "hotter" and more likely to be relevant for active development tasks. Compute `git log --format='%H' -- <file> | wc -l` per file. Weight recent changes more heavily (exponential decay).

**Co-Change Analysis (Temporal Coupling):** Files that frequently change together in the same commits are likely related. This captures implicit dependencies not visible in import graphs (e.g., a test file and its implementation, a handler and its route registration). Compute a co-change matrix from git history. When a query matches file A, boost files that historically co-change with A. Research from the mining software repositories community validates this signal for change prediction and impact analysis.

**Directory Structure Proximity:** Files in the same directory or nearby directories are more likely related to the same feature. A simple signal: given a query that matches `pkg/handler/user.go`, boost `pkg/handler/user_test.go` and `pkg/model/user.go`.

**File Type / Role Weighting:** Implementation files are generally more relevant than test files for "add feature" queries, while test files are more relevant for "fix bug" or "add test" queries. Config files (YAML, JSON, TOML) are relevant for infrastructure queries. Assign per-role base weights and adjust based on query classification.

Sources:
- [Rethinking search results ranking on Sourcegraph.com](https://sourcegraph.com/blog/new-search-ranking)
- [Ranking in a week - Eric Fritz](https://www.eric-fritz.com/articles/ranking-in-a-week/)
- [Indexed ranking - Sourcegraph docs](https://docs.sourcegraph.com/dev/background-information/architecture/indexed-ranking)

---

### 5. Hybrid Scoring Without LLMs

The standard architecture:

1. **BM25 (or BM25F)** produces a lexical relevance score per file
2. **Pre-computed embeddings** produce a semantic similarity score via cosine similarity (just vector math, no LLM at query time)
3. **Structural signals** (PageRank, change frequency, co-change, etc.) produce additional scores
4. **Fusion** combines all scores into a final ranking

For fusion, two main approaches:

**Reciprocal Rank Fusion (RRF):** `score(d) = sum_i(1 / (k + rank_i(d)))` where k=60 is the standard constant. RRF ignores raw scores and only uses rank positions. Advantages: no normalization needed, robust across disparate score distributions, minimal tuning. Used by Elasticsearch, OpenSearch, Azure AI Search, Chroma.

**Weighted Linear Combination:** Normalize each signal to [0,1] (min-max or z-score), then compute `score(d) = w1*bm25(d) + w2*cosine(d) + w3*pagerank(d) + ...`. Advantages: can precisely tune relative importance. Disadvantages: requires score normalization (tricky when distributions differ), more hyperparameters to tune. Elasticsearch's research found that "when weights are carefully calibrated, linear combination can outperform RRF."

**Practical recommendation:** Start with RRF for its simplicity and robustness. Graduate to weighted linear combination if you have evaluation data to tune weights against.

**Pre-computed embeddings with local inference (ONNX Runtime):** Compute embeddings at index time using a small model (all-MiniLM-L6-v2 at 22M params, or nomic-embed-text at 137M params). Store the vectors. At query time, embed the query string (one inference call, ~15-70ms for small models) and compute cosine similarity against all stored vectors. The query embedding is the only inference needed -- everything else is dot products. If you pre-normalize embeddings to unit length, cosine similarity reduces to a dot product, which is trivially fast.

Sources:
- [Reciprocal Rank Fusion (RRF) - ParadeDB](https://www.paradedb.com/learn/search-concepts/reciprocal-rank-fusion)
- [Hybrid search revisited: linear retriever - Elasticsearch Labs](https://www.elastic.co/search-labs/blog/linear-retriever-hybrid-search)
- [Weighted RRF - Elasticsearch Labs](https://www.elastic.co/search-labs/blog/weighted-reciprocal-rank-fusion-rrf)
- [Hybrid Search Scoring (RRF) - Azure AI Search](https://learn.microsoft.com/en-us/azure/search/hybrid-search-ranking)

---

### 6. ONNX Runtime in Rust (the `ort` crate)

**`ort` (pykeio/ort):** The primary Rust binding for ONNX Runtime. Actively maintained. Used in production by Bloop (semantic code search), SurrealDB, Google's Magika (file type detection), and Hugging Face Text Embeddings Inference (TEI).

**`fastembed-rs` (Anush008/fastembed-rs):** Higher-level crate built on `ort` specifically for embeddings. Features:
- Text embeddings (dense and sparse)
- Reranking
- Auto-downloads models from HuggingFace
- Quantized model variants (Q8, Q4) for smaller size
- Uses `ort` internally + HuggingFace tokenizers

**Performance numbers:**
- all-MiniLM-L6-v2: ~14.7ms per 1K tokens, ~68ms end-to-end latency per query
- On AWS Lambda with ONNX Runtime: ~280ms inference latency for embedding models
- For Q8/Q4 models under 500MB, single-threaded inference is optimal

**Recommended models for code search (via ONNX):**
- `all-MiniLM-L6-v2`: 22M params, 384-dim, blazing fast, but lower accuracy (56% Top-5 on MTEB)
- `nomic-embed-text` (v1.5 or v2): 137M params, 768-dim, 86.2% Top-5, supports 8192 token context
- `voyage-code-3`: Code-specific embedding model, but proprietary (API only)

Sources:
- [ort - GitHub (pykeio/ort)](https://github.com/pykeio/ort)
- [fastembed-rs - GitHub](https://github.com/Anush008/fastembed-rs)

---

### 7. What Modern AI Coding Tools Use

**Cursor:**
- Indexes the entire codebase into a vector store using "Cursor's own embedding model" (proprietary)
- Uses AST-based chunking (tree-sitter) at logical boundaries (functions, classes) for supported languages; falls back to regex-based splitting
- Retrieval: vector similarity search via Turbopuffer (nearest-neighbor)
- Re-ranking: uses an AI model (likely a cross-encoder) to re-rank candidate snippets
- Uses Merkle tree for incremental re-indexing (only changed files re-embedded)

**GitHub Copilot:**
- Context Retriever considers: recently edited files, symbols, embeddings, imports, prior completions
- Prompt Assembler ranks and fits snippets within token limits using heuristics + token estimation
- RAG combined with "non-neural code search capabilities" (likely trigram/BM25)
- September 2025: 37.6% better retrieval accuracy, 8x smaller index, ~190ms retrieval latency

**Sourcegraph (code search, powers Cody AI):**
- Zoekt: trigram-based code search engine
- BM25F with 5x boost for filename/symbol matches
- PageRank over the code symbol graph (inbound references as "links")
- Plans for symbol-level PageRank (finer granularity than file-level)

**Continue (open source):**
- @codebase context provider with embedding + re-ranking
- Indexing via embedding models (default: all-MiniLM-L6-v2 locally, or nomic-embed-text via Ollama)
- Re-ranking models for relevance scoring
- Uses LanceDB for vector storage (local)
- Fully local option with Ollama embeddings

**Common pattern across all tools:** Two-stage retrieval (fast candidate selection via embeddings or BM25, then neural re-ranking), AST-aware chunking, and a mix of lexical + semantic signals.

Sources:
- [How Cursor Works - BitPeak Deep Dive](https://bitpeak.com/how-cursor-works-deep-dive-into-vibe-coding/)
- [GitHub Copilot Chat: The Life of a Prompt](https://devblogs.microsoft.com/all-things-azure/github-copilot-chat-explained-the-life-of-a-prompt/)
- [Continue.dev Documentation](https://docs.continue.dev)
- [Keeping it boring with BM25F - Sourcegraph](https://sourcegraph.com/blog/keeping-it-boring-and-relevant-with-bm25f)

---

### Recommended Scoring Evolution Path

| Priority | Signal | Effort | Impact | Query-time cost |
|----------|--------|--------|--------|-----------------|
| 1 | BM25F (boost filename + symbol matches) | Low | High (~20% per Sourcegraph) | Zero additional |
| 2 | File role weighting (test vs impl vs config) | Low | Medium | Zero additional |
| 3 | Import graph PageRank | Medium | High | Zero (pre-computed) |
| 4 | RRF fusion of BM25 + structural signals | Low | Medium | Trivial |
| 5 | Pre-computed embeddings (fastembed-rs/ort + nomic-embed-text) | Medium | High | ~15-70ms for query embedding |
| 6 | Co-change analysis from git history | Medium | Medium | Zero (pre-computed) |
| 7 | SPLADE-doc expansion at index time | High | Medium | Zero at query time |

---

## Rust Alternatives to LiteLLM (Python-Free LLM/Embedding Access)

### 1. ONNX Runtime in Rust (`ort` crate)

**Maturity**: High. Most production-proven Rust ML inference option. Wraps Microsoft's ONNX Runtime C++ library via FFI. Used by Bloop, SurrealDB, Google's Magika, and HuggingFace TEI.

**Embedding model support**: Runs `all-MiniLM-L6-v2`, `nomic-embed-text-v1.5`, and any ONNX-exported model.

**Python dependency**: None. Links against ONNX Runtime C++ shared library (~30-50 MB).

**Performance**: ~40-100 sentences/sec on CPU single-threaded for `all-MiniLM-L6-v2`. With ONNX quantization (QInt8), inference drops to ~15ms per document.

### 2. Candle (HuggingFace Rust ML Framework)

**Maturity**: Medium-high. Designed for serverless inference -- lightweight binaries without Python. Supports CPU (with optional MKL/Accelerate), CUDA, Metal, and WASM backends.

**Python dependency**: None. Pure Rust (optional C linkage for MKL/CUDA).

**Drawback**: Must implement or port model architecture yourself. More effort than "load an ONNX file."

### 3. Tract (Sonos)

**Maturity**: High for its niche. Pure Rust, no C++ dependencies. Passes ~85% of ONNX backend tests. Designed for embedded/edge deployment.

**Python dependency**: None. Zero non-Rust dependencies.

**Performance**: Optimized for small models on CPU. Slower than `ort` since it lacks heavily optimized C++ kernels.

**Drawback**: 85% ONNX operator coverage means some embedding models may fail to load. CPU-only.

### 4. FastEmbed-rs

**Maturity**: High for embeddings specifically. Rust rewrite of Qdrant's FastEmbed Python library. Used by SurrealDB. Uses `ort` under the hood.

**Supported models**: all-MiniLM-L6-v2, nomic-embed-text-v1/v1.5, bge-small/base/large-en-v1.5, multilingual-e5, mxbai-embed-large-v1, ModernBERT-embed-large, and more.

**Python dependency**: None. Downloads ONNX models from HuggingFace on first use.

**Key advantage**: Drop-in solution. `TextEmbedding::try_new(EmbeddingModel::AllMiniLML6V2)` and you are embedding text.

### 5. Model2Vec-rs (Static Embeddings)

**Standout finding.** Model2Vec distills large sentence transformers into static embedding models:
- 500x faster than the original transformer on CPU
- Model sizes: 7-30 MB (vs 80-260 MB for full transformers)
- Crate size: ~1.7 MB
- No ONNX, no C++, just Rust + safetensors loading + tokenizer
- Pre-trained models: `potion-base-32M` and `potion-retrieval-32M` (optimized for retrieval)

**Python dependency**: None.

**Drawback**: Lower quality than full transformer embeddings. The 500x speedup comes from using static (lookup-based) embeddings rather than contextual transformer inference.

### 6. Ollama HTTP API from Rust

The simplest option. The Ollama `/api/embed` endpoint is trivial:
```
POST http://localhost:11434/api/embed
{"model": "nomic-embed-text", "input": ["text here"]}
```
A dozen lines of Rust with `reqwest` + `serde_json`. No Python. Ollama itself is a Go binary.

**Drawback**: Requires Ollama running as a separate process. Adds latency (HTTP round-trip).

**Advantage**: Already working in Go. Supports every model Ollama supports (hundreds).

### 7. llama.cpp Rust Bindings

Crates: `llama-cpp-2`, `llama_cpp`, `embellama`. Embedding support via GGUF models. Q4_K_M quantization reduces model size by ~4x with minimal quality loss. Metal acceleration on macOS.

**Drawback**: Compiles llama.cpp from source (adds build time, needs CMake).

### Best Embedding Models for Code

| Model | Params | Dims | Size on Disk | Best For |
|-------|--------|------|-------------|----------|
| all-MiniLM-L6-v2 | 33M | 384 | ~80 MB (ONNX) | General text, fast baseline |
| nomic-embed-text-v1.5 | 137M | 768 (Matryoshka: 64-768) | ~262 MB (FP16) | Text + code, open-source |
| bge-small-en-v1.5 | 33M | 384 | ~80 MB | General text, very fast |
| e5-small-v2 | 33M | 384 | ~80 MB | Fastest published (16ms/1K tokens) |
| voyage-code-3 | Unknown | Unknown | API only | Best code benchmarks (proprietary) |

### Do We Even Need Embeddings?

**Short answer**: For file-level relevance ranking in a CLI tool, embeddings are likely overkill. Enhanced BM25 + structural signals can match or beat them.

**Evidence from Sourcegraph**: Their production code search saw a 20% improvement across all key metrics from adding BM25F (BM25 with field boosting) to their baseline ranking.

**Key insight from Sourcegraph's blog**: "Matches on file names and symbol definitions tend to be much more meaningful than those in the middle of a statement or comment." This is a structural signal, not a semantic one. BM25 with field weighting handles this well.

**When embeddings DO help**: Semantic queries ("find the authentication logic"), cross-language matching, synonym resolution.

**When embeddings DON'T help much**: File-level relevance where filenames and paths are strong signals, exact identifier matching.

### Recommendation Summary

| Option | Python-Free | Pure Rust | Binary Size Impact | Best For |
|--------|------------|-----------|-------------------|----------|
| fastembed-rs | Yes | No (C++ via ort) | +30-50 MB runtime | Drop-in embedding solution |
| ort (raw) | Yes | No (C++ FFI) | +30-50 MB runtime | Custom ONNX models |
| candle | Yes | Yes* | Minimal | Custom architectures |
| tract | Yes | Yes | Minimal | Pure Rust, no C deps |
| model2vec-rs | Yes | Yes | +7-30 MB model | Ultra-fast, lightweight |
| ollama-rs / reqwest | Yes | Yes | Negligible | External Ollama present |
| Enhanced BM25 | Yes | Yes | Zero | File-level ranking |

---

## Final Recommendation and Architecture Decisions

### Verdict: Rewrite repo-context in Rust as "Atlas"

The strongest arguments are not raw speed (Go optimizations already handle that) but rather:

| Dimension | Winner | Why it matters |
|-----------|--------|----------------|
| Deep index loading | Rust (rkyv zero-copy: <5ms vs 244ms) | Eliminates the #1 bottleneck, 2-4x faster balanced queries |
| AST chunking | Rust (tree-sitter: 100+ langs vs 8 regex) | Qualitative leap in accuracy and language coverage |
| Binary size | Rust (2-4 MB vs 10 MB) | Distribution, containers, edge |
| Gitignore | Rust (`ignore` crate: full spec) | Fixes a known functional limitation |
| WASM | Rust (sub-1 MB, production WASI) | Unlocks browser/edge deployment Go can't match |
| Compile time | Go (1-3s vs 30-90s) | Real DX cost |
| Dependencies | Go (1 vs ~150-200 crates) | Real supply chain cost |

### Ranking: LLM vs Algorithmic

The data from Sourcegraph (the largest code search engine) is definitive: **BM25F + structural signals beats naive embedding-based approaches for file-level code ranking.** Their production system saw a 20% improvement from BM25F field weighting alone -- no ML at all.

File-level ranking is fundamentally a structural problem. When you search "add health check endpoint", the strongest signal is that there's a file called `health_check.go` or a function called `HealthCheck`. BM25F with a 5x filename/symbol boost captures this perfectly.

Embeddings help with semantic queries ("find the authentication logic" matching code that uses `jwt.Verify()`), but this is the minority use case. Most users describe tasks in terms that overlap with filenames and symbols.

### Finalized Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Name | **Atlas** | Titan who held up the sky; short to type |
| Repo | **demwunz/atlas** (standalone) | Decoupled from wobot monorepo |
| CLI compat | **Drop-in replacement** | Same commands, flags, JSONL v0.3 output |
| First release | **Full vision v1** | All features + improvements ship together |
| Scoring | **BM25F + structural signals + RRF** | In-binary, no external deps |
| Embeddings | **Optional via HTTP** | `--rerank` flag, Ollama/OpenAI only |
| Index format | **rkyv zero-copy** | Fresh start, no gob backward compat |
| AST chunking | **tree-sitter** | 100+ languages, exact boundaries |
| File walking | **`ignore` crate** | Full gitignore spec from ripgrep |
| Python deps | **None** | Zero Python anywhere |

### What Goes Inside the Binary

| Signal | Query-time cost | Quality impact |
|--------|----------------|----------------|
| BM25F (filename 5x, symbols 3x, content 1x) | Microseconds | High (proven 20% improvement) |
| Import graph centrality (simplified PageRank) | Zero (pre-computed) | High |
| Git recency (change frequency) | Zero (pre-computed) | Medium |
| File role weighting (test/impl/config) | Zero (pre-computed) | Medium |
| tree-sitter symbol extraction | Zero (pre-computed) | High (feeds BM25F symbol field) |
| RRF fusion of all signals | Microseconds | High (robust combination) |

Total query-time cost: microseconds, all in-process, zero network, zero external dependency.

### Embeddings: Keep as Optional, Not Core

Ollama HTTP as an opt-in `--rerank` flag for users who want semantic reranking -- but the tool should be excellent without it. No fastembed-rs, no model bundled in the binary, no ONNX Runtime. Just `reqwest` HTTP calls to Ollama if the user explicitly asks.

This gives:
- **Default path**: Fast, deterministic, zero dependencies, works offline
- **Power user path**: `--rerank` for semantic enhancement if they have Ollama

### Crate Stack

| Category | Crate | Purpose |
|----------|-------|---------|
| CLI | clap | Argument parsing (derive mode) |
| Serialization | serde, serde_json, serde_yml | JSON/YAML/JSONL |
| Index format | rkyv, memmap2 | Zero-copy index loading |
| File walking | ignore | Full gitignore, parallel walking |
| AST chunking | tree-sitter + per-language grammars | 100+ language support |
| Scoring | Hand-rolled BM25F | Field-weighted BM25 |
| Parallelism | rayon | Data-parallel file processing |
| Hashing | sha2 | Incremental index updates |
| Errors | anyhow | Ergonomic error handling |
| HTTP (optional) | reqwest | Ollama embedding reranking |

---

## External Sources

### Go vs Rust Performance
- [Rust vs Go 2025 - JetBrains](https://blog.jetbrains.com/rust/2025/06/12/rust-vs-go/)
- [Go vs Rust Performance 2025](https://www.codezion.com/blog/go-vs-rust/)
- [Benchmarks Game: Rust vs Go](https://benchmarksgame-team.pages.debian.net/benchmarksgame/fastest/rust-go.html)

### Serialization
- [rkyv is faster than everything](https://david.kolo.ski/blog/rkyv-is-faster-than/)
- [Rust Serialization Benchmark](https://david.kolo.ski/rust_serialization_benchmark/)
- [Go Serialization Benchmarks](https://github.com/alecthomas/go_serialization_benchmarks)

### Code Search
- [Keeping it boring (and relevant) with BM25F - Sourcegraph](https://sourcegraph.com/blog/keeping-it-boring-and-relevant-with-bm25f)
- [Rethinking search results ranking - Sourcegraph](https://sourcegraph.com/blog/new-search-ranking)
- [Tantivy - Rust full-text search engine](https://github.com/quickwit-oss/tantivy)

### Rust Tooling
- [ripgrep benchmarks](https://burntsushi.net/ripgrep/)
- [cargo-dist](https://axodotdev.github.io/cargo-dist/)
- [cargo-binstall](https://github.com/cargo-bins/cargo-binstall)
- [min-sized-rust](https://github.com/johnthagen/min-sized-rust)

### Hybrid Search & Fusion
- [Reciprocal Rank Fusion - ParadeDB](https://www.paradedb.com/learn/search-concepts/reciprocal-rank-fusion)
- [Hybrid Search Scoring (RRF) - Azure AI Search](https://learn.microsoft.com/en-us/azure/search/hybrid-search-ranking)
- [Weighted RRF - Elasticsearch Labs](https://www.elastic.co/search-labs/blog/weighted-reciprocal-rank-fusion-rrf)

### ML Inference in Rust
- [ort (ONNX Runtime)](https://github.com/pykeio/ort)
- [fastembed-rs](https://github.com/Anush008/fastembed-rs)
- [Candle (HuggingFace)](https://github.com/huggingface/candle)
- [Model2Vec-rs](https://github.com/MinishLab/model2vec-rs)
- [Tract (Sonos)](https://github.com/sonos/tract)
