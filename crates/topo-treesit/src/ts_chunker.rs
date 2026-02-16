//! Tree-sitter based code chunker.
//!
//! Uses `LazyLock` to initialize grammars once on first use.
//! Each grammar entry contains a pre-compiled `Query` for efficient reuse.

use std::collections::HashMap;
use std::sync::LazyLock;

use topo_core::{Chunk, ChunkKind, Language};
use tree_sitter::{Parser, Query, QueryCursor, StreamingIterator};

use crate::Chunker;
use crate::queries;

struct GrammarEntry {
    language: tree_sitter::Language,
    query: Query,
    function_idx: Option<u32>,
    type_idx: Option<u32>,
    impl_idx: Option<u32>,
    import_idx: Option<u32>,
    name_idx: Option<u32>,
}

static GRAMMARS: LazyLock<HashMap<Language, GrammarEntry>> = LazyLock::new(init_grammars);

/// Tree-sitter based chunker. Zero-size struct — all state lives in `GRAMMARS`.
pub struct TreeSitterChunker;

impl Chunker for TreeSitterChunker {
    fn chunk(&self, content: &str, language: Language) -> Vec<Chunk> {
        let grammars = &*GRAMMARS;
        let entry = match grammars.get(&language) {
            Some(e) => e,
            None => return vec![],
        };

        let mut parser = Parser::new();
        if parser.set_language(&entry.language).is_err() {
            return vec![];
        }
        let tree = match parser.parse(content, None) {
            Some(t) => t,
            None => return vec![],
        };

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&entry.query, tree.root_node(), content.as_bytes());
        let mut chunks = Vec::new();

        while let Some(m) = matches.next() {
            let mut outer_node = None;
            let mut name_node = None;
            let mut kind = ChunkKind::Other;

            for capture in m.captures {
                if entry.name_idx == Some(capture.index) {
                    name_node = Some(capture.node);
                } else if entry.function_idx == Some(capture.index) {
                    outer_node = Some(capture.node);
                    kind = ChunkKind::Function;
                } else if entry.type_idx == Some(capture.index) {
                    outer_node = Some(capture.node);
                    kind = ChunkKind::Type;
                } else if entry.impl_idx == Some(capture.index) {
                    outer_node = Some(capture.node);
                    kind = ChunkKind::Impl;
                } else if entry.import_idx == Some(capture.index) {
                    outer_node = Some(capture.node);
                    kind = ChunkKind::Import;
                }
            }

            let node = match outer_node {
                Some(n) => n,
                None => continue,
            };

            let name = name_node
                .and_then(|n| n.utf8_text(content.as_bytes()).ok())
                .unwrap_or("")
                .to_string();

            let start_line = node.start_position().row as u32 + 1;
            let end_line = node.end_position().row as u32 + 1;
            // Content not populated — BM25F only uses chunk.name for scoring.
            // Skipping utf8_text() avoids ~27K string allocations on large repos.
            let node_content = String::new();

            chunks.push(Chunk {
                kind,
                name,
                start_line,
                end_line,
                content: node_content,
            });
        }

        chunks
    }
}

/// Initialize all grammar entries.
fn init_grammars() -> HashMap<Language, GrammarEntry> {
    let mut map = HashMap::new();

    // Most grammars export `LANGUAGE: LanguageFn`
    type LangInit = (Language, fn() -> tree_sitter::Language);
    let lang_fn_entries: &[LangInit] = &[
        (Language::Rust, || tree_sitter_rust::LANGUAGE.into()),
        (Language::Go, || tree_sitter_go::LANGUAGE.into()),
        (Language::Python, || tree_sitter_python::LANGUAGE.into()),
        (Language::JavaScript, || {
            tree_sitter_javascript::LANGUAGE.into()
        }),
        (Language::TypeScript, || {
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into()
        }),
        (Language::Java, || tree_sitter_java::LANGUAGE.into()),
        (Language::Ruby, || tree_sitter_ruby::LANGUAGE.into()),
        (Language::C, || tree_sitter_c::LANGUAGE.into()),
        (Language::Cpp, || tree_sitter_cpp::LANGUAGE.into()),
        (Language::Shell, || tree_sitter_bash::LANGUAGE.into()),
        (Language::Swift, || tree_sitter_swift::LANGUAGE.into()),
        (Language::Kotlin, || tree_sitter_kotlin_ng::LANGUAGE.into()),
        (Language::Scala, || tree_sitter_scala::LANGUAGE.into()),
        (Language::Haskell, || tree_sitter_haskell::LANGUAGE.into()),
        (Language::Elixir, || tree_sitter_elixir::LANGUAGE.into()),
        (Language::Lua, || tree_sitter_lua::LANGUAGE.into()),
        (Language::Php, || tree_sitter_php::LANGUAGE_PHP.into()),
        (Language::R, || tree_sitter_r::LANGUAGE.into()),
    ];

    for &(lang, make_ts_lang) in lang_fn_entries {
        let query_src = match queries::query_for(lang) {
            Some(q) => q,
            None => continue,
        };

        let ts_lang = make_ts_lang();
        let query = match Query::new(&ts_lang, query_src) {
            Ok(q) => q,
            Err(_) => continue,
        };

        let function_idx = capture_index(&query, "function");
        let type_idx = capture_index(&query, "type");
        let impl_idx = capture_index(&query, "impl");
        let import_idx = capture_index(&query, "import");
        let name_idx = capture_index(&query, "name");

        map.insert(
            lang,
            GrammarEntry {
                language: ts_lang,
                query,
                function_idx,
                type_idx,
                impl_idx,
                import_idx,
                name_idx,
            },
        );
    }

    map
}

fn capture_index(query: &Query, name: &str) -> Option<u32> {
    let names = query.capture_names();
    names.iter().position(|n| *n == name).map(|i| i as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rust_functions_and_types() {
        let src = r#"
pub fn authenticate(token: &str) -> bool {
    !token.is_empty()
}

pub struct Config {
    pub name: String,
}

pub enum Status {
    Active,
    Inactive,
}

impl Config {
    pub fn new() -> Self {
        Self { name: String::new() }
    }
}

use std::collections::HashMap;
"#;
        let chunks = TreeSitterChunker.chunk(src, Language::Rust);
        assert!(!chunks.is_empty(), "should produce chunks for Rust");

        let fn_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.kind == ChunkKind::Function)
            .collect();
        assert!(!fn_chunks.is_empty(), "should find functions");
        assert!(fn_chunks.iter().any(|c| c.name == "authenticate"));
        assert!(fn_chunks.iter().any(|c| c.name == "new"));

        // Functions should have multi-line spans
        let auth = fn_chunks.iter().find(|c| c.name == "authenticate").unwrap();
        assert!(
            auth.end_line > auth.start_line,
            "function should span multiple lines"
        );
        // Content is intentionally empty (not used by scoring pipeline)
        assert!(auth.content.is_empty(), "content should be empty");

        let type_chunks: Vec<_> = chunks
            .iter()
            .filter(|c| c.kind == ChunkKind::Type)
            .collect();
        assert!(type_chunks.iter().any(|c| c.name == "Config"));
        assert!(type_chunks.iter().any(|c| c.name == "Status"));

        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Impl));
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Import));
    }

    #[test]
    fn python_functions_and_classes() {
        let src = r#"
class UserService:
    def authenticate(self, token):
        return True

async def fetch_data(url):
    pass

import os
from pathlib import Path
"#;
        let chunks = TreeSitterChunker.chunk(src, Language::Python);
        assert!(!chunks.is_empty(), "should produce chunks for Python");
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "UserService" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "authenticate" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "fetch_data" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .filter(|c| c.kind == ChunkKind::Import)
                .count()
                >= 2
        );
    }

    #[test]
    fn go_functions_and_types() {
        let src = r#"
package main

import "fmt"

func main() {
    fmt.Println("hello")
}

type Config struct {
    Name string
}
"#;
        let chunks = TreeSitterChunker.chunk(src, Language::Go);
        assert!(!chunks.is_empty(), "should produce chunks for Go");
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "main" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Config" && c.kind == ChunkKind::Type)
        );
    }

    #[test]
    fn javascript_functions_and_classes() {
        let src = r#"
function authenticate(token) {
    return true;
}

class UserService {
    constructor() {}
}

import { useState } from 'react';
"#;
        let chunks = TreeSitterChunker.chunk(src, Language::JavaScript);
        assert!(!chunks.is_empty(), "should produce chunks for JavaScript");
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "authenticate" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "UserService" && c.kind == ChunkKind::Type)
        );
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Import));
    }

    #[test]
    fn unsupported_language_returns_empty() {
        let chunks = TreeSitterChunker.chunk("# heading\nsome text", Language::Markdown);
        assert!(chunks.is_empty());
    }

    #[test]
    fn empty_content() {
        let chunks = TreeSitterChunker.chunk("", Language::Rust);
        assert!(chunks.is_empty());
    }

    #[test]
    fn end_line_greater_than_start_for_multiline() {
        let src = "fn hello() {\n    println!(\"hi\");\n}\n";
        let chunks = TreeSitterChunker.chunk(src, Language::Rust);
        let fn_chunk = chunks.iter().find(|c| c.kind == ChunkKind::Function);
        assert!(fn_chunk.is_some());
        let f = fn_chunk.unwrap();
        assert!(
            f.end_line > f.start_line,
            "multi-line function should have end_line > start_line"
        );
    }
}
