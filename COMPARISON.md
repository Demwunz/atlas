<div align="center">

# Tool Comparison

**How Topo compares to code search, AI editors, and coding assistants.**

[![Rust](https://img.shields.io/badge/Rust-2024_edition-000000?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Free](https://img.shields.io/badge/cost-free-brightgreen?style=for-the-badge)](#cost-comparison)
[![Offline](https://img.shields.io/badge/runs-fully_offline-blue?style=for-the-badge)](#feature-matrix)

[README](README.md) · [Benchmarks](BENCHMARKS.md)

</div>

---

## Feature Matrix

| Capability | ripgrep | Sourcegraph | Cursor | aider | Copilot | **Topo** |
|---|:---:|:---:|:---:|:---:|:---:|:---:|
| Semantic parsing (AST-level symbols) | — | ✓ (SCIP) | partial | ✓ (tree-sitter) | undisclosed | **✓** (18 languages) |
| Import graph | — | ✓ | — | ✓ | undisclosed | **✓** |
| Structural scoring (PageRank) | — | — | — | ✓ | — | **✓** |
| Multi-signal fusion (text + heuristic + structural) | — | — | — | — | — | **✓** |
| Token-budget control | — | — | — | ✓ | — | **✓** |
| Persistent index (cached, incremental) | — | ✓ | ✓ | — | undisclosed | **✓** |
| Fully offline (no cloud, no API keys) | ✓ | — | — | partial | — | **✓** |
| Standalone CLI | ✓ | partial (src CLI) | — | ✓ | partial (gh ext) | **✓** |
| Open source | ✓ | ✓ | — | ✓ | — | **✓** |
| Languages with structural support | — | 40+ (SCIP) | undisclosed | ~10 | undisclosed | **18** |

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

## Where Each Tool Fits

**ripgrep** is the fastest text search tool available — it searches the Linux kernel in ~0.06 seconds. But it has zero semantic understanding. It matches character sequences, not code concepts. If the function you need doesn't contain your search terms, ripgrep won't find it.

**Sourcegraph** provides the deepest code intelligence in this list. Its SCIP indexers deliver IDE-grade navigation (go-to-definition, find-references) across 40+ languages, backed by a server-side search platform. It doesn't rank files by importance or select context for LLMs — it's a search and navigation tool. Cody, its AI assistant, uses Sourcegraph's index for retrieval. Enterprise pricing starts at $59/user/month.

**Cursor** is an AI-powered editor that uses embedding-based RAG to find relevant code for its built-in LLM. Its indexing and retrieval are proprietary and cloud-dependent. It doesn't expose structural scoring, import graphs, or file-importance ranking — context selection is a black box optimized for Cursor's own AI workflow.

**aider** is the closest analog to Topo's approach. It uses tree-sitter to extract a "repo map" of symbols, builds a dependency graph with PageRank, and fits results within a token budget. The differences: aider is Python (not a compiled binary), has no persistent index (rebuilds on every invocation), uses a single-signal ranking (no multi-signal fusion), and is coupled to an AI chat loop — you can't use its file-selection outside of aider itself.

**GitHub Copilot** has the widest adoption of any AI coding tool. Its context retrieval is entirely proprietary — there's no public documentation of how it selects files for its context window. It requires cloud connectivity and a subscription. There's no standalone CLI for file selection independent of the Copilot experience.

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

## Speed Comparison

Different tools solve different problems, so direct speed comparisons require context.

| Operation | Tool | Time | Scale |
|-----------|------|------|-------|
| Text search (Linux kernel) | ripgrep | ~0.06 s | 70k+ files |
| Deep index (cold build) | **Topo** | 11.0 s | 28k files (Kubernetes) |
| Deep index (cold build) | **Topo** | 2.1 s | 16k files (Discourse) |
| Query (balanced preset) | **Topo** | 2.1 s | 28k files (Kubernetes) |
| Query (balanced preset) | **Topo** | 0.9 s | 9.6k files (Mastodon) |
| Indexing claim | Cursor | "4 hours → 21 seconds" | undisclosed repo size |

ripgrep is faster because it does less — it searches text, not ranks files. Topo's query time includes loading a semantic index from disk, scoring every file across multiple signals, and rendering output. Cursor's indexing claim comes from its marketing; the repo size and methodology are not published.

All Topo numbers are from [BENCHMARKS.md](BENCHMARKS.md), reproducible on any machine.

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

## Scoring Quality

Topo is the only tool in this comparison that publishes file-selection quality benchmarks.

Key findings from [BENCHMARKS.md](BENCHMARKS.md):

- **PageRank surfaces files text search misses.** On Discourse, `lib/service.rb` (PageRank 0.92) — the core module required by nearly every Ruby file — jumps from absent to #1 when structural scoring is enabled. It doesn't contain the query terms, but any developer working on the task needs it.
- **Multi-signal fusion eliminates false positives.** On Mastodon, the balanced preset surfaced 3 PNG icon files in the top 7 for "notification push web" because they had matching filenames. The deep preset (with RRF fusion) eliminated all of them and replaced them with actual source files.
- **Cross-language structure works.** On Discourse, a query about "middleware plugin" surfaces both Ruby service modules and JavaScript plugin APIs — two different languages, connected through the import graph.

No other tool in this comparison publishes equivalent file-selection quality data.

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

## Cost Comparison

| Tool | Cost | What you get |
|------|------|-------------|
| ripgrep | Free | Text search |
| Sourcegraph | $59/user/mo (enterprise) | Code search + navigation + Cody AI |
| Cursor | $20–200/mo | AI editor with built-in context retrieval |
| aider | Free + LLM API costs | AI coding assistant with repo-map |
| Copilot | $10–39/mo | AI completions + chat |
| **Topo** | **Free** | **Structural file selection — no LLM required** |

Topo requires no API keys, no subscriptions, and no cloud connectivity. It complements every tool in this table — use Topo to select files, then feed them to whichever AI tool you prefer.

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

## Reproducing

All Topo benchmarks in this document reference data from [BENCHMARKS.md](BENCHMARKS.md). Every number is reproducible:

```bash
cargo build --release
git clone --depth 1 https://github.com/kubernetes/kubernetes.git /tmp/k8s-bench
time topo --root /tmp/k8s-bench index --deep
topo --root /tmp/k8s-bench explain "auth middleware" --top 10 --preset deep --format human
```

See [BENCHMARKS.md — Reproducing These Benchmarks](BENCHMARKS.md#-reproducing-these-benchmarks) for full instructions.

<p align="right">(<a href="#tool-comparison">back to top</a>)</p>

---

<div align="center">

**[Back to README](README.md) · [Benchmarks](BENCHMARKS.md) · [Report Bug](https://github.com/demwunz/topo/issues)**

</div>
