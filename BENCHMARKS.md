<div align="center">

# Benchmarks

**Real-world performance and quality measurements across production codebases.**

[![Rust](https://img.shields.io/badge/Rust-2024_edition-000000?style=for-the-badge&logo=rust)](https://www.rust-lang.org)
[![Tests](https://img.shields.io/badge/tests-266_passing-brightgreen?style=for-the-badge)](#)

[Index Performance](#-index-performance) · [Query Quality](#-query-quality-balanced-vs-deep) · [PageRank Impact](#-pagerank-impact-on-polyglot-repos) · [Language Coverage](#-import-extraction-coverage) · [Tool Comparison](COMPARISON.md)

</div>

---

<details>
<summary>Table of Contents</summary>

- [Test Repos](#-test-repos)
- [Index Performance](#-index-performance)
- [Query Timing](#-query-timing)
- [Query Quality: Balanced vs Deep](#-query-quality-balanced-vs-deep)
- [PageRank Impact on Polyglot Repos](#-pagerank-impact-on-polyglot-repos)
- [Import Extraction Coverage](#-import-extraction-coverage)
- [Reproducing These Benchmarks](#-reproducing-these-benchmarks)

</details>

---

## Test Repos

All benchmarks run on Apple Silicon (M-series, release build) against real open-source repositories cloned at `--depth 1`.

| Repo | Files | Primary Languages | Character |
|------|-------|-------------------|-----------|
| **[Kubernetes](https://github.com/kubernetes/kubernetes)** | 28,359 | Go (16,677), YAML (5,621), Shell (307) | Monolingual (Go), massive scale |
| **[Discourse](https://github.com/discourse/discourse)** | 16,804 | Ruby (8,979), JS (2,642), GJS (2,431), SCSS (639) | Polyglot (Ruby + JS frontend) |
| **[Mastodon](https://github.com/mastodon/mastodon)** | 9,616 | Ruby (3,057), TSX (330), TS (239), JSX (110), JS (104) | Polyglot (Ruby + React frontend) |

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## Index Performance

### Deep index (cold build, from scratch)

| Repo | Files Indexed | Wall Clock | Index Size |
|------|--------------|-----------|------------|
| Kubernetes | 27,828 | **11.0 s** | 139 MB |
| Discourse | 16,402 | **2.1 s** | 50 MB |
| Mastodon | 9,456 | **1.4 s** | 58 MB |

Index builds are parallelized with `rayon`. The Kubernetes index is larger because Go files tend to have more exported symbols and longer term frequency maps.

### Incremental updates

After modifying a single file and re-running `topo index --deep`:

| Repo | Changed Files | Wall Clock |
|------|--------------|-----------|
| Kubernetes | 7 | **5.1 s** |
| Discourse | 0 | **< 1 s** |

Unchanged files are detected via SHA-256 comparison and carried forward from the existing index.

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## Query Timing

Query timing includes loading the index from disk, scoring all files, and rendering output.

| Repo | Balanced | Deep | Overhead |
|------|----------|------|----------|
| Kubernetes | 2.1 s | 2.6 s | +24% |
| Discourse | 1.4 s | 1.6 s | +14% |
| Mastodon | 0.9 s | 1.0 s | +11% |

The `deep` preset adds PageRank via RRF fusion. The overhead is the cost of loading PageRank scores from the index — the PageRank computation itself is done at index time.

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## Query Quality: Balanced vs Deep

The `balanced` preset uses BM25F + heuristic scoring. The `deep` preset adds PageRank via RRF fusion, surfacing structurally important files that text matching alone would miss.

### Discourse: "authentication login session"

**Balanced** (BM25F + heuristic only):

```
PATH                                                  TOTAL    BM25F     HEUR       PR     ROLE
-----------------------------------------------------------------------------------------------
admin_config_login_and_authentication_spec.rb         6.5947  10.5666   0.6367        -     test
pages/admin_login_and_authentication.rb               6.4617  10.3851   0.5767        -     test
session/sso_login.html.erb                            6.3777  10.2584   0.5567        -    other
lib/email/authentication_results.rb                   4.1750   6.4994   0.6883        -     impl
lib/discourse_webauthn/authentication_service.rb      4.0963   6.3817   0.6683        -     impl
```

**Deep** (+ PageRank):

```
PATH                                                  TOTAL    BM25F     HEUR       PR     ROLE
-----------------------------------------------------------------------------------------------
lib/service.rb                                       0.0203   0.0000   0.5850   0.9224     impl
frontend/discourse/app/models/session.js             0.0180   5.8765   0.5583   0.0373     impl
lib/email/authentication_results.rb                  0.0175   6.4994   0.6883   0.0149     impl
admin_config_login_and_authentication_spec.rb        0.0168  10.5666   0.6367   0.0081     test
frontend/discourse/app/lib/object.js                 0.0166   0.0000   0.4350   1.0000     impl
```

**What changed:** `lib/service.rb` (PR=0.92) is the core service module that nearly every Ruby file in Discourse requires. It jumps from nowhere to #1. `frontend/discourse/app/lib/object.js` (PR=1.0) is the most-imported JS file in the frontend. Both are foundational files that pure text matching can't discover because they don't contain the words "authentication" or "session" — but any developer working on auth would need to understand them.

### Mastodon: "notification push web"

**Balanced:**

```
PATH                                                  TOTAL    BM25F     HEUR       PR     ROLE
-----------------------------------------------------------------------------------------------
workers/web/push_notification_worker.rb              7.7338  12.2730   0.9250        -     impl
spec/workers/web/push_notification_worker_spec.rb    7.4967  12.0078   0.7300        -     test
lib/web_push_request.rb                              5.6120   8.8056   0.8217        -     impl
models/web/push_subscription.rb                      5.6000   8.8056   0.7917        -     impl
public/web-push-icon_expand.png                      5.5100   8.8056   0.5667        -    other
public/web-push-icon_favourite.png                   5.5060   8.8056   0.5567        -    other
public/web-push-icon_reblog.png                      5.5060   8.8056   0.5567        -    other
```

**Deep:**

```
PATH                                                  TOTAL    BM25F     HEUR       PR     ROLE
-----------------------------------------------------------------------------------------------
app/models/web.rb                                    0.0201   4.6111   0.6783   0.0075     impl
workers/web/push_notification_worker.rb              0.0176  12.2730   0.9250   0.0012     impl
spec/workers/web/push_notification_worker_spec.rb    0.0173  12.0078   0.7300   0.0012     test
lib/web_push_request.rb                              0.0171   8.8056   0.8217   0.0012     impl
models/web/push_subscription.rb                      0.0168   8.8056   0.7917   0.0012     impl
spec/rails_helper.rb                                 0.0168   0.0000   0.3900   1.0000     test
```

**What changed:** The balanced preset surfaced **3 PNG icon files** (`.png`) in the top 7 because they had "web-push" in their filename — classic BM25 false positives. Deep preset eliminated all of them via RRF fusion. `spec/rails_helper.rb` (PR=1.0, the most-required file in any Rails test suite) and `app/models/web.rb` (the Web namespace root) appeared instead.

### Discourse: "middleware plugin request processing"

**Balanced** — top 5 are all `lib/middleware/*.rb` files, sorted purely by keyword density.

**Deep** — `lib/service.rb` (PR=0.92) surfaces at #1 as the core require. `select-kit/lib/plugin-api.js` (PR=0.30) surfaces at #5 — the JavaScript plugin API that's imported across the entire Discourse frontend. Cross-language structural signals at work.

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## PageRank Impact on Polyglot Repos

### Why polyglot repos benefit more

On a monolingual repo like Kubernetes (99% Go), the import graph reflects a single language's dependency structure. The same "hub" files (`meta/v1/time.go`, `runtime/schema/interfaces.go`) appear in every query's top results.

On polyglot repos, the import graph captures **cross-language structure**:

- **Discourse:** Ruby `require` edges + JavaScript `import` edges form separate subgraphs, each with their own hub files. A query about "middleware" surfaces both the Ruby service core and the JS plugin API.
- **Mastodon:** Ruby backend + React (TS/JSX) frontend have distinct import hierarchies. `rails_helper.rb` is the hub for Ruby tests; `initial_state.ts` is the hub for frontend state.

### Top PageRank files by repo

**Kubernetes (Go):**

| File | PageRank | Role |
|------|----------|------|
| `apimachinery/pkg/apis/meta/v1/time.go` | 1.000 | Most-imported file in all of k8s |
| `kubelet/container/testing/os.go` | 0.542 | OS abstraction for kubelet tests |
| `controller/nodeipam/ipam/sync/sync.go` | 0.430 | IPAM sync controller |
| `apimachinery/pkg/runtime/schema/interfaces.go` | 0.353 | Core schema types |

**Discourse (Ruby + JS):**

| File | PageRank | Role |
|------|----------|------|
| `frontend/discourse/app/lib/object.js` | 1.000 | Base JS object — imported everywhere |
| `frontend/discourse/app/form-kit/components/fk/object.gjs` | 1.000 | FormKit base component |
| `lib/service.rb` | 0.922 | Core Ruby service module |
| `frontend/discourse/app/lib/deprecated.js` | 0.938 | Deprecation helpers (widely imported) |
| `frontend/discourse/select-kit/lib/plugin-api.js` | 0.301 | Plugin API for the frontend |

**Mastodon (Ruby + TS/JSX):**

| File | PageRank | Role |
|------|----------|------|
| `spec/rails_helper.rb` | 1.000 | Required by every test file |
| `.rubocop/rspec.yml` | 0.285 | RSpec config (required transitively) |
| `mastodon/initial_state.ts` | 0.098 | Frontend state initialization |
| `config/environment.rb` | 0.079 | Rails environment bootstrap |

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## Import Extraction Coverage

Topo extracts imports from **16 programming languages** and resolves them to repo-local files. External/stdlib imports (no matching file in the repo) are automatically filtered out.

| Language | Extraction Pattern | Resolution Strategy |
|----------|--------------------|---------------------|
| Rust | `use crate::`, `mod` | File stem matching |
| Go | `import "path"`, block imports | Directory-based + multi-segment disambiguation |
| Python | `import`, `from ... import` | Stem + relative path resolution |
| JavaScript | `import from`, `require()` | Stem + relative path resolution |
| TypeScript | Same as JavaScript | Same as JavaScript |
| Java | `import`, `import static` | Last qualified segment |
| Kotlin | Same as Java | Same as Java |
| C | `#include "header.h"` (quoted only) | Relative path + stem fallback |
| C++ | Same as C | Same as C |
| Ruby | `require`, `require_relative` | Stem + relative path resolution |
| Swift | `import Module`, `@testable import` | Stem matching |
| Elixir | `alias`, `import`, `use`, `require` | Last module segment |
| PHP | `use` namespace, `require`/`include` | Namespace last segment + relative path |
| Scala | `import` qualified paths | Last qualified segment |
| R | `library()`, `require()`, `source()` | Stem + relative path resolution |
| Shell | `source`, `.` | Relative path + stem fallback |

### What's excluded

- **Angle-bracket includes** (`#include <stdio.h>`) — system/external headers
- **Vendored paths** (`vendor/`, `node_modules/`, `third_party/`) — excluded from the import graph entirely
- **Markup/config formats** (Markdown, YAML, TOML, JSON, HTML, CSS) — no import semantics

### Go-specific handling

Go imports reference **packages (directories)**, not individual files. Topo uses a dual-index strategy:

- **Directory index:** maps parent directory names to contained files (e.g., `v1/` -> `[api/core/v1/types.go, ...]`)
- **Multi-segment disambiguation:** when the last import segment is ambiguous (e.g., multiple `v1/` dirs), the penultimate segment narrows results (`core/v1` vs `apps/v1`)
- **Stem fallback:** for flat layouts without matching directories

This prevents false positives like `v1.yaml` or `v1.json` matching a `k8s.io/api/core/v1` import.

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

## Reproducing These Benchmarks

```bash
# Build release binary
cargo build --release

# Clone test repos
git clone --depth 1 https://github.com/kubernetes/kubernetes.git /tmp/k8s-bench
git clone --depth 1 https://github.com/discourse/discourse.git /tmp/discourse-bench
git clone --depth 1 https://github.com/mastodon/mastodon.git /tmp/mastodon-bench

# Index
time topo --root /tmp/k8s-bench index --deep
time topo --root /tmp/discourse-bench index --deep
time topo --root /tmp/mastodon-bench index --deep

# Inspect
topo --root /tmp/discourse-bench inspect

# Compare balanced vs deep
topo --root /tmp/discourse-bench explain "authentication login session" --top 15 --preset balanced --format human
topo --root /tmp/discourse-bench explain "authentication login session" --top 15 --preset deep --format human

# Run your own queries
topo --root /tmp/mastodon-bench quick "notification push" --preset deep --format human
```

<p align="right">(<a href="#benchmarks">back to top</a>)</p>

---

<div align="center">

**[Back to README](README.md) · [Tool Comparison](COMPARISON.md) · [Report Bug](https://github.com/demwunz/topo/issues) · [Request Feature](https://github.com/demwunz/topo/issues)**

</div>
