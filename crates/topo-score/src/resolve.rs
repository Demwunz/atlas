use crate::pagerank::ImportGraph;
use std::collections::HashMap;
use std::path::Path;
use topo_core::Language;

/// Directories whose contents should be excluded from the import graph.
/// These are vendored/generated paths — external dependencies checked into the repo.
const VENDORED_DIRS: &[&str] = &["vendor", "node_modules", "third_party"];

/// Indexes for resolving import paths to repo files.
///
/// Two lookup strategies:
/// - `stem`: file stem → files (e.g., `"handler"` → `["src/auth/handler.rs"]`)
/// - `dir`: parent directory name → files within it (e.g., `"v1"` → `["api/core/v1/types.go"]`)
///
/// Most languages use the stem index. Go uses the dir index because Go imports
/// reference packages (directories), not individual files.
pub struct RepoIndex {
    pub stem: HashMap<String, Vec<String>>,
    pub dir: HashMap<String, Vec<String>>,
}

/// Build stem and directory indexes from file paths.
pub fn build_file_index(paths: &[&str]) -> RepoIndex {
    let mut stem_index: HashMap<String, Vec<String>> = HashMap::new();
    let mut dir_index: HashMap<String, Vec<String>> = HashMap::new();

    for &path in paths {
        let p = Path::new(path);

        // Index by file stem (e.g., "handler" from "src/auth/handler.rs")
        if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
            let stem_lower = stem.to_lowercase();
            stem_index
                .entry(stem_lower.clone())
                .or_default()
                .push(path.to_string());

            // For mod.rs / index.js / __init__.py, also index the parent directory name
            if matches!(stem, "mod" | "index" | "__init__")
                && let Some(parent) = p
                    .parent()
                    .and_then(|d| d.file_name())
                    .and_then(|n| n.to_str())
            {
                stem_index
                    .entry(parent.to_lowercase())
                    .or_default()
                    .push(path.to_string());
            }
        }

        // Index by immediate parent directory name
        // e.g., "api/core/v1/types.go" → dir["v1"] contains this file
        if let Some(parent_name) = p
            .parent()
            .and_then(|d| d.file_name())
            .and_then(|n| n.to_str())
        {
            dir_index
                .entry(parent_name.to_lowercase())
                .or_default()
                .push(path.to_string());
        }
    }

    RepoIndex {
        stem: stem_index,
        dir: dir_index,
    }
}

/// Resolve a single raw import to candidate repo file paths.
///
/// Returns an empty vec for external/unresolved imports (no matching repo file).
pub fn resolve_import(
    raw_import: &str,
    importing_file: &str,
    language: Language,
    file_index: &RepoIndex,
) -> Vec<String> {
    let candidates = match language {
        Language::Rust => resolve_rust(raw_import, &file_index.stem),
        Language::JavaScript | Language::TypeScript => {
            resolve_js(raw_import, importing_file, &file_index.stem)
        }
        Language::Python => resolve_python(raw_import, importing_file, &file_index.stem),
        Language::Go => resolve_go(raw_import, file_index),
        Language::Java | Language::Kotlin => resolve_java(raw_import, &file_index.stem),
        Language::C | Language::Cpp => {
            resolve_c_include(raw_import, importing_file, &file_index.stem)
        }
        Language::Ruby => resolve_ruby(raw_import, importing_file, &file_index.stem),
        Language::Swift => resolve_swift(raw_import, &file_index.stem),
        Language::Elixir => resolve_elixir(raw_import, &file_index.stem),
        Language::Php => resolve_php(raw_import, importing_file, &file_index.stem),
        Language::Scala => resolve_scala(raw_import, &file_index.stem),
        Language::R => resolve_r(raw_import, importing_file, &file_index.stem),
        Language::Shell => resolve_shell(raw_import, importing_file, &file_index.stem),
        _ => Vec::new(),
    };

    // Filter out self-imports
    candidates
        .into_iter()
        .filter(|c| c != importing_file)
        .collect()
}

/// Returns true if a path is under a vendored/generated directory.
fn is_vendored(path: &str) -> bool {
    path.split(['/', '\\'])
        .any(|component| VENDORED_DIRS.contains(&component))
}

/// Build an ImportGraph from files with their content.
///
/// Vendored/generated paths (vendor/, node_modules/, third_party/) are excluded
/// from the graph entirely — they don't become nodes, don't appear in the file
/// index, and can't receive PageRank. This prevents checked-in dependencies
/// from dominating the structural signal.
pub fn build_import_graph(
    file_imports: &[(String, Language, Vec<String>)],
    all_paths: &[&str],
) -> ImportGraph {
    // Filter out vendored paths before building the file index and graph
    let non_vendored: Vec<&str> = all_paths
        .iter()
        .copied()
        .filter(|p| !is_vendored(p))
        .collect();

    let file_index = build_file_index(&non_vendored);
    let mut graph = ImportGraph::new();

    // Add only non-vendored files as nodes
    for path in &non_vendored {
        graph.add_node(path);
    }

    // Resolve imports and add edges (only from non-vendored files)
    for (path, language, raw_imports) in file_imports {
        if is_vendored(path) {
            continue;
        }
        for raw in raw_imports {
            let resolved = resolve_import(raw, path, *language, &file_index);
            for target in resolved {
                graph.add_edge(path, &target);
            }
        }
    }

    graph
}

/// Rust: match module name against file stems.
/// e.g., `"auth"` matches `src/auth.rs` or `src/auth/mod.rs`.
fn resolve_rust(module: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    file_index
        .get(&module.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// JS/TS: relative paths resolve relative to importing file; bare specifiers match stems.
fn resolve_js(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if import_path.starts_with('.') {
        // Relative import: resolve relative to importing file's directory
        let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
        let resolved = base.join(import_path);

        // Extract the stem from the resolved path
        let stem = resolved.file_stem().and_then(|s| s.to_str()).unwrap_or("");

        if stem.is_empty() {
            return Vec::new();
        }

        // Match against file index, but prefer files in the resolved directory
        let stem_lower = stem.to_lowercase();
        let candidates = file_index.get(&stem_lower).cloned().unwrap_or_default();

        // Try to narrow to files near the expected path
        let resolved_str = resolved.to_string_lossy();
        let near: Vec<String> = candidates
            .iter()
            .filter(|c| {
                let c_no_ext = Path::new(c.as_str())
                    .with_extension("")
                    .to_string_lossy()
                    .into_owned();
                c_no_ext == resolved_str.as_ref() || c.starts_with(resolved_str.as_ref())
            })
            .cloned()
            .collect();

        if near.is_empty() { candidates } else { near }
    } else {
        // Bare specifier: match last path segment against file stems
        let segment = import_path.rsplit('/').next().unwrap_or(import_path);
        file_index
            .get(&segment.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

/// Python: relative imports resolve relative to importing file; absolute match stems.
fn resolve_python(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if import_path.starts_with('.') {
        // Relative import
        let module = import_path.trim_start_matches('.');
        if module.is_empty() {
            // `from . import X` — try the parent package's __init__.py
            let parent = Path::new(importing_file)
                .parent()
                .and_then(|d| d.file_name())
                .and_then(|n| n.to_str())
                .unwrap_or("");
            return file_index
                .get(&parent.to_lowercase())
                .cloned()
                .unwrap_or_default();
        }
        // e.g., `.utils` — match "utils" against stems
        let parts: Vec<&str> = module.split('.').collect();
        let last = parts.last().copied().unwrap_or("");
        file_index
            .get(&last.to_lowercase())
            .cloned()
            .unwrap_or_default()
    } else {
        // Absolute import: match first/last segment against stems
        let parts: Vec<&str> = import_path.split('.').collect();
        // Try last segment first (more specific), then first
        for segment in [parts.last().copied(), parts.first().copied()]
            .iter()
            .flatten()
        {
            let candidates = file_index
                .get(&segment.to_lowercase())
                .cloned()
                .unwrap_or_default();
            if !candidates.is_empty() {
                return candidates;
            }
        }
        Vec::new()
    }
}

/// Go: resolve by matching import path segments against directory structure.
///
/// Go imports reference packages (directories), not files. `"k8s.io/api/core/v1"`
/// means "files inside a directory named `v1`". We use the directory index to find
/// files whose parent directory matches the last import segment, then narrow using
/// the penultimate segment for disambiguation.
fn resolve_go(import_path: &str, index: &RepoIndex) -> Vec<String> {
    let segments: Vec<&str> = import_path.rsplitn(3, '/').collect();
    let last = segments.first().copied().unwrap_or("");
    if last.is_empty() {
        return Vec::new();
    }

    let last_lower = last.to_lowercase();

    // Look up files whose parent directory matches the last segment
    let dir_candidates = index.dir.get(&last_lower).cloned().unwrap_or_default();

    if !dir_candidates.is_empty() {
        // If we have a penultimate segment, prefer files where the grandparent matches
        if let Some(&penultimate) = segments.get(1) {
            let pen_lower = penultimate.to_lowercase();
            let narrowed: Vec<String> = dir_candidates
                .iter()
                .filter(|path| {
                    // Check if the path contains ".../penultimate/last/file"
                    let p = Path::new(path.as_str());
                    p.parent()
                        .and_then(|d| d.parent())
                        .and_then(|gp| gp.file_name())
                        .and_then(|n| n.to_str())
                        .is_some_and(|gp_name| gp_name.to_lowercase() == pen_lower)
                })
                .cloned()
                .collect();
            if !narrowed.is_empty() {
                return narrowed;
            }
        }
        return dir_candidates;
    }

    // Fallback: stem-based matching (for single-file packages or flat layouts)
    index.stem.get(&last_lower).cloned().unwrap_or_default()
}

/// Java: match last segment of qualified name against file stems.
fn resolve_java(import_path: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    // Handle wildcard imports: com.example.* → match "example"
    let path = import_path.trim_end_matches(".*");
    let segment = path.rsplit('.').next().unwrap_or(path);
    file_index
        .get(&segment.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// C/C++: resolve `#include "header.h"` paths.
///
/// Quoted includes are project-local. Resolve relative to the importing file's
/// directory, then fall back to stem matching.
fn resolve_c_include(
    include_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    // Try resolving relative to the importing file's directory
    let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
    let resolved = base.join(include_path);
    let resolved_str = resolved.to_string_lossy();

    // Check if the resolved path matches any known file exactly
    for files in file_index.values() {
        for f in files {
            if f == resolved_str.as_ref() {
                return vec![f.clone()];
            }
        }
    }

    // Fall back to stem matching
    let stem = Path::new(include_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    file_index
        .get(&stem.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// Ruby: resolve `require` and `require_relative`.
///
/// `require_relative` resolves relative to the importing file. Plain `require`
/// matches against file stems.
fn resolve_ruby(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    // Extract the last path segment as stem for matching
    let segment = import_path.rsplit('/').next().unwrap_or(import_path);
    let stem_lower = segment.to_lowercase();

    // For paths that look relative (contain / or start with .), try relative resolution
    if import_path.contains('/') || import_path.starts_with('.') {
        let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
        let resolved = base.join(import_path);
        let resolved_str = resolved.to_string_lossy();

        // Try exact match with .rb extension
        let candidates = file_index.get(&stem_lower).cloned().unwrap_or_default();
        let near: Vec<String> = candidates
            .iter()
            .filter(|c| {
                let c_no_ext = Path::new(c.as_str())
                    .with_extension("")
                    .to_string_lossy()
                    .into_owned();
                c_no_ext == resolved_str.as_ref()
            })
            .cloned()
            .collect();
        if !near.is_empty() {
            return near;
        }
    }

    // Fall back to stem matching
    file_index.get(&stem_lower).cloned().unwrap_or_default()
}

/// Swift: match module name against file stems.
fn resolve_swift(module: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    file_index
        .get(&module.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// Elixir: match last module segment against file stems.
///
/// `MyApp.Auth.Handler` → try "Handler", then "Auth".
fn resolve_elixir(module_path: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    // Elixir modules are like MyApp.Auth.Handler — try last segment first
    for segment in module_path.rsplit('.') {
        let candidates = file_index
            .get(&segment.to_lowercase())
            .cloned()
            .unwrap_or_default();
        if !candidates.is_empty() {
            return candidates;
        }
    }
    Vec::new()
}

/// PHP: resolve `use` namespaces and `require`/`include` paths.
fn resolve_php(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    if import_path.contains('\\') {
        // Namespace import: App\Auth\Handler → match last segment "Handler"
        let segment = import_path.rsplit('\\').next().unwrap_or(import_path);
        file_index
            .get(&segment.to_lowercase())
            .cloned()
            .unwrap_or_default()
    } else {
        // File path: resolve relative to importing file, fall back to stem
        let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
        let resolved = base.join(import_path);
        let resolved_str = resolved.to_string_lossy();

        for files in file_index.values() {
            for f in files {
                if f == resolved_str.as_ref() {
                    return vec![f.clone()];
                }
            }
        }

        let stem = Path::new(import_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        file_index
            .get(&stem.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

/// Scala: match last segment of import path against file stems.
///
/// `com.example.auth.Handler` → match "Handler".
fn resolve_scala(import_path: &str, file_index: &HashMap<String, Vec<String>>) -> Vec<String> {
    let segment = import_path.rsplit('.').next().unwrap_or(import_path);
    file_index
        .get(&segment.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

/// R: resolve `source()` paths relative to importing file, `library()`/`require()` by stem.
fn resolve_r(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    // If it looks like a file path (has extension or slash), resolve as path
    if import_path.contains('/') || import_path.contains('.') {
        let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
        let resolved = base.join(import_path);
        let resolved_str = resolved.to_string_lossy();

        for files in file_index.values() {
            for f in files {
                if f == resolved_str.as_ref() {
                    return vec![f.clone()];
                }
            }
        }

        let stem = Path::new(import_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        file_index
            .get(&stem.to_lowercase())
            .cloned()
            .unwrap_or_default()
    } else {
        // Package name from library()/require() — match against stems
        file_index
            .get(&import_path.to_lowercase())
            .cloned()
            .unwrap_or_default()
    }
}

/// Shell: resolve `source`/`.` paths relative to importing file.
fn resolve_shell(
    import_path: &str,
    importing_file: &str,
    file_index: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    let base = Path::new(importing_file).parent().unwrap_or(Path::new(""));
    let resolved = base.join(import_path);
    let resolved_str = resolved.to_string_lossy();

    // Try exact path match
    for files in file_index.values() {
        for f in files {
            if f == resolved_str.as_ref() {
                return vec![f.clone()];
            }
        }
    }

    // Fall back to stem matching
    let stem = Path::new(import_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    file_index
        .get(&stem.to_lowercase())
        .cloned()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_file_index_basic() {
        let paths = vec![
            "src/auth.rs",
            "src/auth/mod.rs",
            "src/handler.rs",
            "src/lib.rs",
        ];
        let idx = build_file_index(&paths);

        assert!(idx.stem["auth"].contains(&"src/auth.rs".to_string()));
        assert!(idx.stem["auth"].contains(&"src/auth/mod.rs".to_string()));
        assert!(idx.stem["handler"].contains(&"src/handler.rs".to_string()));
        assert!(idx.stem["lib"].contains(&"src/lib.rs".to_string()));
    }

    #[test]
    fn build_file_index_mod_indexes_parent() {
        let paths = vec!["src/auth/mod.rs"];
        let idx = build_file_index(&paths);

        // "mod" stem entry
        assert!(idx.stem["mod"].contains(&"src/auth/mod.rs".to_string()));
        // parent directory "auth" entry
        assert!(idx.stem["auth"].contains(&"src/auth/mod.rs".to_string()));
    }

    #[test]
    fn build_file_index_js_index() {
        let paths = vec!["src/components/index.ts"];
        let idx = build_file_index(&paths);

        assert!(idx.stem["index"].contains(&"src/components/index.ts".to_string()));
        assert!(idx.stem["components"].contains(&"src/components/index.ts".to_string()));
    }

    #[test]
    fn resolve_rust_module() {
        let paths = vec!["src/auth.rs", "src/db.rs"];
        let idx = build_file_index(&paths);

        let result = resolve_import("auth", "src/main.rs", Language::Rust, &idx);
        assert_eq!(result, vec!["src/auth.rs".to_string()]);
    }

    #[test]
    fn resolve_js_relative() {
        let paths = vec!["src/utils.ts", "src/handler.ts"];
        let idx = build_file_index(&paths);

        let result = resolve_import("./utils", "src/handler.ts", Language::TypeScript, &idx);
        assert!(result.contains(&"src/utils.ts".to_string()));
    }

    #[test]
    fn resolve_js_bare_specifier_no_match() {
        let paths = vec!["src/handler.ts"];
        let idx = build_file_index(&paths);

        // "react" has no matching file — external dependency
        let result = resolve_import("react", "src/handler.ts", Language::JavaScript, &idx);
        assert!(result.is_empty());
    }

    #[test]
    fn resolve_python_relative() {
        let paths = vec!["src/utils.py", "src/main.py"];
        let idx = build_file_index(&paths);

        let result = resolve_import(".utils", "src/main.py", Language::Python, &idx);
        assert!(result.contains(&"src/utils.py".to_string()));
    }

    #[test]
    fn resolve_go_directory_based() {
        // Go imports reference packages (directories), not files
        let paths = vec![
            "pkg/http/handler.go",
            "pkg/http/server.go",
            "internal/auth/auth.go",
        ];
        let idx = build_file_index(&paths);

        // "myapp/pkg/http" → last segment "http" matches directory "pkg/http/"
        let result = resolve_import("myapp/pkg/http", "cmd/main.go", Language::Go, &idx);
        assert!(result.contains(&"pkg/http/handler.go".to_string()));
        assert!(result.contains(&"pkg/http/server.go".to_string()));

        // "myapp/internal/auth" → matches files in "auth/" directory
        let result2 = resolve_import("myapp/internal/auth", "cmd/main.go", Language::Go, &idx);
        assert!(result2.contains(&"internal/auth/auth.go".to_string()));
    }

    #[test]
    fn resolve_go_v1_stem_collision() {
        // The core problem: "k8s.io/api/core/v1" should match files in a
        // "v1/" directory, NOT files named "v1.yaml" or "v1.json"
        let paths = vec![
            "staging/src/k8s.io/api/core/v1/types.go",
            "staging/src/k8s.io/api/core/v1/register.go",
            "testdata/config/after/v1.yaml",
            "testdata/openapi/v3/api/v1.json",
        ];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "k8s.io/api/core/v1",
            "pkg/scheduler/scheduler.go",
            Language::Go,
            &idx,
        );
        // Should match Go files in the v1/ directory
        assert!(result.contains(&"staging/src/k8s.io/api/core/v1/types.go".to_string()));
        assert!(result.contains(&"staging/src/k8s.io/api/core/v1/register.go".to_string()));
        // Should NOT match testdata files named v1.*
        assert!(!result.contains(&"testdata/config/after/v1.yaml".to_string()));
        assert!(!result.contains(&"testdata/openapi/v3/api/v1.json".to_string()));
    }

    #[test]
    fn resolve_go_multi_segment_disambiguation() {
        // Two packages both named "v1" but in different parent dirs
        let paths = vec!["api/core/v1/types.go", "api/apps/v1/deployment.go"];
        let idx = build_file_index(&paths);

        // "k8s.io/api/core/v1" → penultimate "core" narrows to core/v1/
        let result = resolve_import("k8s.io/api/core/v1", "cmd/main.go", Language::Go, &idx);
        assert!(result.contains(&"api/core/v1/types.go".to_string()));
        assert!(!result.contains(&"api/apps/v1/deployment.go".to_string()));

        // "k8s.io/api/apps/v1" → penultimate "apps" narrows to apps/v1/
        let result2 = resolve_import("k8s.io/api/apps/v1", "cmd/main.go", Language::Go, &idx);
        assert!(result2.contains(&"api/apps/v1/deployment.go".to_string()));
        assert!(!result2.contains(&"api/core/v1/types.go".to_string()));
    }

    #[test]
    fn resolve_go_fallback_to_stem() {
        // When there's no directory match, fall back to stem matching
        let paths = vec!["pkg/handler.go"];
        let idx = build_file_index(&paths);

        let result = resolve_import("myapp/handler", "cmd/main.go", Language::Go, &idx);
        assert!(result.contains(&"pkg/handler.go".to_string()));
    }

    #[test]
    fn resolve_java_qualified() {
        let paths = vec!["src/main/java/AuthService.java"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.auth.AuthService",
            "src/main/java/App.java",
            Language::Java,
            &idx,
        );
        assert!(result.contains(&"src/main/java/AuthService.java".to_string()));
    }

    #[test]
    fn resolve_java_wildcard() {
        let paths = vec!["src/main/java/Utils.java"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.utils.*",
            "src/main/java/App.java",
            Language::Java,
            &idx,
        );
        assert!(result.contains(&"src/main/java/Utils.java".to_string()));
    }

    #[test]
    fn resolve_c_include_relative() {
        let paths = vec!["src/auth.h", "src/auth.c", "src/utils/helpers.h"];
        let idx = build_file_index(&paths);

        // #include "auth.h" from src/main.c → resolves to src/auth.h
        let result = resolve_import("auth.h", "src/main.c", Language::C, &idx);
        assert!(result.contains(&"src/auth.h".to_string()));
    }

    #[test]
    fn resolve_c_include_subdirectory() {
        let paths = vec!["src/utils/helpers.h", "src/main.c"];
        let idx = build_file_index(&paths);

        // #include "utils/helpers.h" from src/main.c
        let result = resolve_import("utils/helpers.h", "src/main.c", Language::C, &idx);
        assert!(result.contains(&"src/utils/helpers.h".to_string()));
    }

    #[test]
    fn resolve_cpp_include_stem_fallback() {
        let paths = vec!["include/myclass.hpp", "src/main.cpp"];
        let idx = build_file_index(&paths);

        // When relative path doesn't match, fall back to stem
        let result = resolve_import("myclass.hpp", "src/main.cpp", Language::Cpp, &idx);
        assert!(result.contains(&"include/myclass.hpp".to_string()));
    }

    #[test]
    fn resolve_ruby_require() {
        let paths = vec!["lib/auth.rb", "lib/handler.rb"];
        let idx = build_file_index(&paths);

        let result = resolve_import("auth", "lib/handler.rb", Language::Ruby, &idx);
        assert!(result.contains(&"lib/auth.rb".to_string()));
    }

    #[test]
    fn resolve_ruby_require_relative() {
        let paths = vec!["lib/utils.rb", "lib/main.rb"];
        let idx = build_file_index(&paths);

        let result = resolve_import("./utils", "lib/main.rb", Language::Ruby, &idx);
        assert!(result.contains(&"lib/utils.rb".to_string()));
    }

    #[test]
    fn resolve_swift_module() {
        let paths = vec!["Sources/Auth/Auth.swift", "Sources/App/App.swift"];
        let idx = build_file_index(&paths);

        // Swift imports are module names, matched against stems
        let result = resolve_import("Auth", "Sources/App/App.swift", Language::Swift, &idx);
        assert!(result.contains(&"Sources/Auth/Auth.swift".to_string()));
    }

    #[test]
    fn resolve_kotlin_import() {
        let paths = vec!["src/main/kotlin/AuthService.kt"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.auth.AuthService",
            "src/main/kotlin/App.kt",
            Language::Kotlin,
            &idx,
        );
        assert!(result.contains(&"src/main/kotlin/AuthService.kt".to_string()));
    }

    #[test]
    fn resolve_elixir_module() {
        let paths = vec!["lib/auth/handler.ex", "lib/utils.ex"];
        let idx = build_file_index(&paths);

        let result = resolve_import("MyApp.Auth.Handler", "lib/app.ex", Language::Elixir, &idx);
        assert!(result.contains(&"lib/auth/handler.ex".to_string()));
    }

    #[test]
    fn resolve_php_namespace() {
        let paths = vec!["src/Auth/Handler.php", "src/App.php"];
        let idx = build_file_index(&paths);

        let result = resolve_import(r"App\Auth\Handler", "src/App.php", Language::Php, &idx);
        assert!(result.contains(&"src/Auth/Handler.php".to_string()));
    }

    #[test]
    fn resolve_php_require() {
        let paths = vec!["src/config.php", "src/main.php"];
        let idx = build_file_index(&paths);

        let result = resolve_import("config.php", "src/main.php", Language::Php, &idx);
        assert!(result.contains(&"src/config.php".to_string()));
    }

    #[test]
    fn resolve_scala_import() {
        let paths = vec!["src/main/scala/Handler.scala"];
        let idx = build_file_index(&paths);

        let result = resolve_import(
            "com.example.auth.Handler",
            "src/main/scala/App.scala",
            Language::Scala,
            &idx,
        );
        assert!(result.contains(&"src/main/scala/Handler.scala".to_string()));
    }

    #[test]
    fn resolve_r_source() {
        let paths = vec!["R/utils.R", "R/main.R"];
        let idx = build_file_index(&paths);

        let result = resolve_import("utils.R", "R/main.R", Language::R, &idx);
        assert!(result.contains(&"R/utils.R".to_string()));
    }

    #[test]
    fn resolve_shell_source() {
        let paths = vec!["lib/utils.sh", "bin/run.sh"];
        let idx = build_file_index(&paths);

        let result = resolve_import("../lib/utils.sh", "bin/run.sh", Language::Shell, &idx);
        assert!(result.contains(&"lib/utils.sh".to_string()));
    }

    #[test]
    fn resolve_filters_self_import() {
        let paths = vec!["src/auth.rs"];
        let idx = build_file_index(&paths);

        let result = resolve_import("auth", "src/auth.rs", Language::Rust, &idx);
        assert!(result.is_empty());
    }

    #[test]
    fn build_import_graph_basic() {
        let all_paths = vec!["src/main.rs", "src/auth.rs", "src/utils.rs"];
        let file_imports = vec![
            (
                "src/main.rs".to_string(),
                Language::Rust,
                vec!["auth".to_string()],
            ),
            (
                "src/auth.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
        ];

        let graph = build_import_graph(&file_imports, &all_paths);

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);

        // utils should have highest PageRank (most downstream)
        let scores = graph.normalized_pagerank();
        assert!(scores["src/utils.rs"] > scores["src/main.rs"]);
    }

    #[test]
    fn build_import_graph_external_imports_ignored() {
        let all_paths = vec!["src/main.rs"];
        let file_imports = vec![(
            "src/main.rs".to_string(),
            Language::Rust,
            vec!["serde".to_string(), "tokio".to_string()],
        )];

        let graph = build_import_graph(&file_imports, &all_paths);

        // External imports should not create edges
        assert_eq!(graph.node_count(), 1);
        assert_eq!(graph.edge_count(), 0);
    }

    #[test]
    fn vendor_paths_excluded_from_graph() {
        let all_paths = vec![
            "cmd/main.go",
            "pkg/handler.go",
            "vendor/github.com/lib/strings.go",
            "vendor/github.com/lib/reflect.go",
            "node_modules/react/index.js",
            "third_party/proto/types.go",
        ];
        let file_imports = vec![
            (
                "cmd/main.go".to_string(),
                Language::Go,
                vec!["handler".to_string(), "strings".to_string()],
            ),
            (
                "vendor/github.com/lib/strings.go".to_string(),
                Language::Go,
                vec!["reflect".to_string()],
            ),
        ];

        let graph = build_import_graph(&file_imports, &all_paths);

        // Only non-vendored files should be nodes
        assert_eq!(graph.node_count(), 2); // cmd/main.go, pkg/handler.go
        // "strings" import should NOT resolve to vendor path
        assert_eq!(graph.edge_count(), 1); // main → handler only

        let scores = graph.normalized_pagerank();
        assert!(scores.contains_key("pkg/handler.go"));
        assert!(!scores.contains_key("vendor/github.com/lib/strings.go"));
    }

    #[test]
    fn is_vendored_detects_vendor_dirs() {
        assert!(is_vendored("vendor/github.com/lib/foo.go"));
        assert!(is_vendored("node_modules/react/index.js"));
        assert!(is_vendored("third_party/proto/types.go"));
        assert!(!is_vendored("src/vendor_utils.go"));
        assert!(!is_vendored("pkg/handler.go"));
        assert!(!is_vendored("cmd/main.go"));
    }

    #[test]
    fn build_import_graph_diamond() {
        // main → auth, main → db, auth → utils, db → utils
        let all_paths = vec!["src/main.rs", "src/auth.rs", "src/db.rs", "src/utils.rs"];
        let file_imports = vec![
            (
                "src/main.rs".to_string(),
                Language::Rust,
                vec!["auth".to_string(), "db".to_string()],
            ),
            (
                "src/auth.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
            (
                "src/db.rs".to_string(),
                Language::Rust,
                vec!["utils".to_string()],
            ),
        ];

        let graph = build_import_graph(&file_imports, &all_paths);
        let scores = graph.normalized_pagerank();

        // utils should have the highest PageRank (imported by auth + db)
        assert_eq!(scores["src/utils.rs"], 1.0);
        assert!(scores["src/utils.rs"] > scores["src/main.rs"]);
    }
}
