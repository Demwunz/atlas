use std::collections::HashMap;

/// Default damping factor for PageRank.
const DAMPING: f64 = 0.85;
/// Default convergence threshold.
const EPSILON: f64 = 1e-6;
/// Maximum iterations to prevent infinite loops.
const MAX_ITERATIONS: usize = 100;

/// Directed graph of file imports for PageRank computation.
pub struct ImportGraph {
    /// Map from file path to list of files it imports.
    edges: HashMap<String, Vec<String>>,
    /// All known file paths.
    nodes: Vec<String>,
}

impl ImportGraph {
    pub fn new() -> Self {
        Self {
            edges: HashMap::new(),
            nodes: Vec::new(),
        }
    }

    /// Add a node (file path) to the graph.
    pub fn add_node(&mut self, path: &str) {
        if !self.edges.contains_key(path) {
            self.edges.insert(path.to_string(), Vec::new());
            self.nodes.push(path.to_string());
        }
    }

    /// Add a directed edge: `from` imports `to`.
    pub fn add_edge(&mut self, from: &str, to: &str) {
        self.add_node(from);
        self.add_node(to);
        self.edges.get_mut(from).unwrap().push(to.to_string());
    }

    /// Build the graph from import relationships extracted from source files.
    ///
    /// Each entry is (file_path, vec_of_imported_paths).
    pub fn from_imports(imports: &[(String, Vec<String>)]) -> Self {
        let mut graph = Self::new();

        for (file, imported) in imports {
            graph.add_node(file);
            for dep in imported {
                graph.add_edge(file, dep);
            }
        }

        graph
    }

    /// Compute PageRank scores for all nodes in the graph.
    ///
    /// Returns a map from file path to PageRank score (0.0 - 1.0 range, sums to ~1.0).
    pub fn pagerank(&self) -> HashMap<String, f64> {
        let n = self.nodes.len();
        if n == 0 {
            return HashMap::new();
        }

        let initial = 1.0 / n as f64;
        let mut scores: HashMap<String, f64> = self
            .nodes
            .iter()
            .map(|node| (node.clone(), initial))
            .collect();

        // Build reverse edges (who imports each file)
        let mut incoming: HashMap<&str, Vec<&str>> = HashMap::new();
        for node in &self.nodes {
            incoming.insert(node.as_str(), Vec::new());
        }
        for (from, tos) in &self.edges {
            for to in tos {
                if let Some(inc) = incoming.get_mut(to.as_str()) {
                    inc.push(from.as_str());
                }
            }
        }

        // Outgoing edge counts
        let out_degree: HashMap<&str, usize> = self
            .edges
            .iter()
            .map(|(k, v)| (k.as_str(), v.len()))
            .collect();

        for _ in 0..MAX_ITERATIONS {
            let mut new_scores: HashMap<String, f64> = HashMap::new();
            let mut max_diff: f64 = 0.0;

            for node in &self.nodes {
                let mut rank = (1.0 - DAMPING) / n as f64;

                if let Some(inbound) = incoming.get(node.as_str()) {
                    for &src in inbound {
                        let src_out = *out_degree.get(src).unwrap_or(&1);
                        let src_score = scores.get(src).copied().unwrap_or(initial);
                        rank += DAMPING * src_score / src_out as f64;
                    }
                }

                let old = scores.get(node).copied().unwrap_or(initial);
                max_diff = max_diff.max((rank - old).abs());
                new_scores.insert(node.clone(), rank);
            }

            scores = new_scores;

            if max_diff < EPSILON {
                break;
            }
        }

        scores
    }

    /// Compute PageRank and normalize to [0.0, 1.0] range.
    pub fn normalized_pagerank(&self) -> HashMap<String, f64> {
        let scores = self.pagerank();
        if scores.is_empty() {
            return scores;
        }

        let max = scores.values().cloned().fold(0.0f64, f64::max);
        if max == 0.0 {
            return scores;
        }

        scores.into_iter().map(|(k, v)| (k, v / max)).collect()
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    pub fn edge_count(&self) -> usize {
        self.edges.values().map(|v| v.len()).sum()
    }
}

impl Default for ImportGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract import paths from common language patterns.
///
/// Returns a list of imported module/file paths (not yet resolved to actual file paths).
pub fn extract_imports(content: &str, language: atlas_core::Language) -> Vec<String> {
    match language {
        atlas_core::Language::Rust => extract_rust_imports(content),
        atlas_core::Language::Python => extract_python_imports(content),
        atlas_core::Language::JavaScript | atlas_core::Language::TypeScript => {
            extract_js_imports(content)
        }
        atlas_core::Language::Go => extract_go_imports(content),
        atlas_core::Language::Java | atlas_core::Language::Kotlin => extract_java_imports(content),
        atlas_core::Language::C | atlas_core::Language::Cpp => extract_c_includes(content),
        atlas_core::Language::Ruby => extract_ruby_imports(content),
        atlas_core::Language::Swift => extract_swift_imports(content),
        _ => Vec::new(),
    }
}

fn extract_rust_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("use ") {
            // "use crate::foo::bar;" -> "foo::bar"
            if let Some(path) = rest.strip_prefix("crate::") {
                let path = path.trim_end_matches(';').trim();
                // Take the first component as the module
                if let Some(module) = path.split("::").next()
                    && !module.is_empty()
                    && module != "{"
                {
                    imports.push(module.to_string());
                }
            }
        } else if let Some(rest) = trimmed.strip_prefix("mod ") {
            let module = rest.trim_end_matches(';').trim();
            if !module.is_empty() && !module.starts_with('{') {
                imports.push(module.to_string());
            }
        }
    }
    imports
}

fn extract_python_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            let module = rest.split_whitespace().next().unwrap_or("");
            if !module.is_empty() {
                imports.push(module.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("from ") {
            let module = rest.split_whitespace().next().unwrap_or("");
            if !module.is_empty() {
                imports.push(module.to_string());
            }
        }
    }
    imports
}

fn extract_js_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // import ... from "path"
        if trimmed.starts_with("import ")
            && let Some(from_idx) = trimmed.find("from ")
        {
            let path_part = &trimmed[from_idx + 5..];
            let path = path_part
                .trim()
                .trim_matches(|c| c == '\'' || c == '"' || c == ';');
            if !path.is_empty() {
                imports.push(path.to_string());
            }
        }
        // const x = require("path")
        if let Some(req_idx) = trimmed.find("require(") {
            let after = &trimmed[req_idx + 8..];
            let path = after
                .trim_start_matches(['\'', '"'])
                .split(['\'', '"'])
                .next()
                .unwrap_or("");
            if !path.is_empty() {
                imports.push(path.to_string());
            }
        }
    }
    imports
}

fn extract_java_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix("import ") {
            // Skip static imports' "static " prefix
            let rest = rest.strip_prefix("static ").unwrap_or(rest);
            let path = rest.trim_end_matches(';').trim();
            if !path.is_empty() {
                imports.push(path.to_string());
            }
        }
    }
    imports
}

fn extract_c_includes(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // #include "local_header.h" — project-local includes (quoted)
        // Skip <system_header.h> — those are system/external
        if let Some(rest) = trimmed.strip_prefix("#include") {
            let rest = rest.trim();
            if rest.starts_with('"') {
                let path = rest.trim_start_matches('"').split('"').next().unwrap_or("");
                if !path.is_empty() {
                    imports.push(path.to_string());
                }
            }
        }
    }
    imports
}

fn extract_ruby_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // require "foo" or require 'foo'
        if let Some(rest) = trimmed
            .strip_prefix("require ")
            .or_else(|| trimmed.strip_prefix("require_relative "))
        {
            let path = rest.trim().trim_matches(|c| c == '\'' || c == '"');
            if !path.is_empty() {
                imports.push(path.to_string());
            }
        }
    }
    imports
}

fn extract_swift_imports(content: &str) -> Vec<String> {
    // Swift allows `import kind Module.Symbol` where kind is class/struct/enum/protocol/func/var/typealias
    const SWIFT_IMPORT_KINDS: &[&str] = &[
        "class",
        "struct",
        "enum",
        "protocol",
        "func",
        "var",
        "typealias",
    ];

    let mut imports = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        // import Foundation / import UIKit / @testable import MyModule
        let line_to_check = trimmed.strip_prefix("@testable ").unwrap_or(trimmed);
        if let Some(rest) = line_to_check.strip_prefix("import ") {
            let mut tokens = rest.split_whitespace();
            let first = tokens.next().unwrap_or("");
            // If the first token is a kind keyword, the module is the next token
            let module = if SWIFT_IMPORT_KINDS.contains(&first) {
                tokens.next().unwrap_or("")
            } else {
                first
            };
            if !module.is_empty() {
                // Extract module name from qualified path (e.g., "CoreData.NSManagedObject" → "CoreData")
                let module_name = module.split('.').next().unwrap_or(module);
                imports.push(module_name.to_string());
            }
        }
    }
    imports
}

fn extract_go_imports(content: &str) -> Vec<String> {
    let mut imports = Vec::new();
    let mut in_import_block = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "import (" {
            in_import_block = true;
            continue;
        }
        if in_import_block && trimmed == ")" {
            in_import_block = false;
            continue;
        }

        if in_import_block {
            let path = trimmed.trim_matches('"');
            if !path.is_empty() {
                imports.push(path.to_string());
            }
        } else if let Some(rest) = trimmed.strip_prefix("import ") {
            let path = rest.trim().trim_matches('"');
            if !path.is_empty() && path != "(" {
                imports.push(path.to_string());
            }
        }
    }
    imports
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagerank_empty_graph() {
        let graph = ImportGraph::new();
        let scores = graph.pagerank();
        assert!(scores.is_empty());
    }

    #[test]
    fn pagerank_single_node() {
        let mut graph = ImportGraph::new();
        graph.add_node("main.rs");
        let scores = graph.pagerank();
        assert_eq!(scores.len(), 1);
        assert!(*scores.get("main.rs").unwrap() > 0.0);
    }

    #[test]
    fn pagerank_chain() {
        // a -> b -> c: c should have highest PageRank (most downstream)
        let mut graph = ImportGraph::new();
        graph.add_edge("a.rs", "b.rs");
        graph.add_edge("b.rs", "c.rs");

        let scores = graph.normalized_pagerank();
        let a = scores["a.rs"];
        let b = scores["b.rs"];
        let c = scores["c.rs"];

        assert!(c > b);
        assert!(b > a);
    }

    #[test]
    fn pagerank_star() {
        // a, b, c all import d: d should have highest PageRank
        let mut graph = ImportGraph::new();
        graph.add_edge("a.rs", "d.rs");
        graph.add_edge("b.rs", "d.rs");
        graph.add_edge("c.rs", "d.rs");

        let scores = graph.normalized_pagerank();
        let d = scores["d.rs"];

        assert_eq!(d, 1.0); // d should be the max (normalized to 1.0)
        for node in ["a.rs", "b.rs", "c.rs"] {
            assert!(scores[node] < d);
        }
    }

    #[test]
    fn pagerank_cycle() {
        // a -> b -> c -> a: all should have roughly equal PageRank
        let mut graph = ImportGraph::new();
        graph.add_edge("a.rs", "b.rs");
        graph.add_edge("b.rs", "c.rs");
        graph.add_edge("c.rs", "a.rs");

        let scores = graph.pagerank();
        let values: Vec<f64> = scores.values().copied().collect();
        let max = values.iter().cloned().fold(0.0f64, f64::max);
        let min = values.iter().cloned().fold(f64::MAX, f64::min);

        // All should be approximately equal in a symmetric cycle
        assert!((max - min) / max < 0.01);
    }

    #[test]
    fn pagerank_from_imports() {
        let imports = vec![
            ("src/main.rs".to_string(), vec!["src/lib.rs".to_string()]),
            (
                "src/lib.rs".to_string(),
                vec!["src/auth.rs".to_string(), "src/db.rs".to_string()],
            ),
            (
                "src/handler.rs".to_string(),
                vec!["src/auth.rs".to_string()],
            ),
        ];

        let graph = ImportGraph::from_imports(&imports);
        let scores = graph.normalized_pagerank();

        // auth.rs is imported by both lib.rs and handler.rs, should have high score
        assert!(scores["src/auth.rs"] > scores["src/main.rs"]);
    }

    #[test]
    fn extract_rust_imports_basic() {
        let code = r#"
use crate::auth::handler;
use crate::db;
mod config;
use std::collections::HashMap;
"#;
        let imports = extract_imports(code, atlas_core::Language::Rust);
        assert!(imports.contains(&"auth".to_string()));
        assert!(imports.contains(&"db".to_string()));
        assert!(imports.contains(&"config".to_string()));
        // std imports should be skipped (no crate:: prefix)
        assert!(!imports.contains(&"std".to_string()));
    }

    #[test]
    fn extract_python_imports_basic() {
        let code = r#"
import os
from pathlib import Path
from . import utils
import json
"#;
        let imports = extract_imports(code, atlas_core::Language::Python);
        assert!(imports.contains(&"os".to_string()));
        assert!(imports.contains(&"pathlib".to_string()));
        assert!(imports.contains(&".".to_string()));
        assert!(imports.contains(&"json".to_string()));
    }

    #[test]
    fn extract_js_imports_basic() {
        let code = r#"
import React from 'react';
import { useState } from "react";
const fs = require('fs');
"#;
        let imports = extract_imports(code, atlas_core::Language::JavaScript);
        assert!(imports.contains(&"react".to_string()));
        assert!(imports.contains(&"fs".to_string()));
    }

    #[test]
    fn extract_go_imports_basic() {
        let code = r#"
import (
	"fmt"
	"net/http"
)
"#;
        let imports = extract_imports(code, atlas_core::Language::Go);
        assert!(imports.contains(&"fmt".to_string()));
        assert!(imports.contains(&"net/http".to_string()));
    }

    #[test]
    fn extract_java_imports_basic() {
        let code = r#"
import com.example.auth.AuthService;
import java.util.List;
import static org.junit.Assert.assertEquals;
"#;
        let imports = extract_imports(code, atlas_core::Language::Java);
        assert!(imports.contains(&"com.example.auth.AuthService".to_string()));
        assert!(imports.contains(&"java.util.List".to_string()));
        assert!(imports.contains(&"org.junit.Assert.assertEquals".to_string()));
    }

    #[test]
    fn extract_c_includes_basic() {
        let code = r#"
#include <stdio.h>
#include "auth.h"
#include "utils/helpers.h"
"#;
        let imports = extract_imports(code, atlas_core::Language::C);
        // Only quoted includes (project-local)
        assert!(imports.contains(&"auth.h".to_string()));
        assert!(imports.contains(&"utils/helpers.h".to_string()));
        // Angle-bracket includes (system) should be skipped
        assert!(!imports.contains(&"stdio.h".to_string()));
    }

    #[test]
    fn extract_cpp_includes_basic() {
        let code = r#"
#include <iostream>
#include <vector>
#include "myclass.hpp"
#include "detail/impl.h"
"#;
        let imports = extract_imports(code, atlas_core::Language::Cpp);
        assert!(imports.contains(&"myclass.hpp".to_string()));
        assert!(imports.contains(&"detail/impl.h".to_string()));
        assert!(!imports.contains(&"iostream".to_string()));
        assert!(!imports.contains(&"vector".to_string()));
    }

    #[test]
    fn extract_ruby_imports_basic() {
        let code = r#"
require 'json'
require "auth/handler"
require_relative "utils"
require_relative "../helpers/crypto"
"#;
        let imports = extract_imports(code, atlas_core::Language::Ruby);
        assert!(imports.contains(&"json".to_string()));
        assert!(imports.contains(&"auth/handler".to_string()));
        assert!(imports.contains(&"utils".to_string()));
        assert!(imports.contains(&"../helpers/crypto".to_string()));
    }

    #[test]
    fn extract_swift_imports_basic() {
        let code = r#"
import Foundation
import UIKit
@testable import MyModule
import class CoreData.NSManagedObject
"#;
        let imports = extract_imports(code, atlas_core::Language::Swift);
        assert!(imports.contains(&"Foundation".to_string()));
        assert!(imports.contains(&"UIKit".to_string()));
        assert!(imports.contains(&"MyModule".to_string()));
        // "import class CoreData.NSManagedObject" → extracts "CoreData" (skips kind keyword)
        assert!(imports.contains(&"CoreData".to_string()));
    }

    #[test]
    fn extract_kotlin_imports_basic() {
        let code = r#"
import com.example.auth.AuthService
import kotlinx.coroutines.launch
import java.util.List
"#;
        let imports = extract_imports(code, atlas_core::Language::Kotlin);
        assert!(imports.contains(&"com.example.auth.AuthService".to_string()));
        assert!(imports.contains(&"kotlinx.coroutines.launch".to_string()));
        assert!(imports.contains(&"java.util.List".to_string()));
    }

    #[test]
    fn graph_counts() {
        let mut graph = ImportGraph::new();
        graph.add_edge("a.rs", "b.rs");
        graph.add_edge("a.rs", "c.rs");

        assert_eq!(graph.node_count(), 3);
        assert_eq!(graph.edge_count(), 2);
    }
}
