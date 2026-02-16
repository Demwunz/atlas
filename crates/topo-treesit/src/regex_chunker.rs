//! Regex-based code chunker for all target languages.
//!
//! Extracts function, type, impl, and import declarations using
//! line-by-line pattern matching. This is the default backend;
//! tree-sitter can be added behind a feature flag for AST precision.

use topo_core::{Chunk, ChunkKind, Language};

use crate::Chunker;

/// Regex-free, pattern-matching chunker that works for all target languages.
pub struct RegexChunker;

impl Chunker for RegexChunker {
    fn chunk(&self, content: &str, language: Language) -> Vec<Chunk> {
        let mut chunks = Vec::new();

        for (i, line) in content.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with("//") {
                continue;
            }
            // '#' is a comment in Python/Ruby/Shell, but not C/C++ (#include, #define)
            if trimmed.starts_with('#') && !matches!(language, Language::C | Language::Cpp) {
                continue;
            }

            let line_num = (i + 1) as u32;

            let result = match language {
                Language::Rust => extract_rust(trimmed),
                Language::Go => extract_go(trimmed),
                Language::Python => extract_python(trimmed),
                Language::JavaScript | Language::TypeScript => extract_js_ts(trimmed),
                Language::Java => extract_java(trimmed),
                Language::Ruby => extract_ruby(trimmed),
                Language::C | Language::Cpp => extract_c_cpp(trimmed),
                _ => None,
            };

            if let Some((kind, name)) = result {
                chunks.push(Chunk {
                    kind,
                    name,
                    start_line: line_num,
                    end_line: line_num,
                    content: String::new(),
                });
            }
        }

        chunks
    }
}

// ── Rust ───────────────────────────────────────────────────────────

fn extract_rust(line: &str) -> Option<(ChunkKind, String)> {
    let stripped = line
        .trim_start_matches("pub ")
        .trim_start_matches("pub(crate) ")
        .trim_start_matches("pub(super) ")
        .trim_start_matches("async ")
        .trim_start_matches("unsafe ")
        .trim_start_matches("const ");

    if let Some(rest) = stripped.strip_prefix("fn ") {
        return ident(rest, &['(', '<', ' ']).map(|n| (ChunkKind::Function, n));
    }
    if let Some(rest) = stripped.strip_prefix("struct ") {
        return ident(rest, &[' ', '{', '<', '(']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("enum ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("trait ") {
        return ident(rest, &[' ', '{', '<', ':']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("type ") {
        return ident(rest, &[' ', '=', '<', ';']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("impl ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Impl, n));
    }
    if stripped.starts_with("use ") {
        return Some((ChunkKind::Import, stripped.to_string()));
    }
    None
}

// ── Go ─────────────────────────────────────────────────────────────

fn extract_go(line: &str) -> Option<(ChunkKind, String)> {
    if let Some(rest) = line.strip_prefix("func ") {
        // Method: func (r *Receiver) Name(...)
        let rest = if rest.starts_with('(') {
            rest.split(')').nth(1)?.trim_start()
        } else {
            rest
        };
        return ident(rest, &['(', ' ']).map(|n| (ChunkKind::Function, n));
    }
    if let Some(rest) = line.strip_prefix("type ") {
        return ident(rest, &[' ']).map(|n| (ChunkKind::Type, n));
    }
    if line.starts_with("import ") || line == "import (" {
        return Some((ChunkKind::Import, line.to_string()));
    }
    None
}

// ── Python ─────────────────────────────────────────────────────────

fn extract_python(line: &str) -> Option<(ChunkKind, String)> {
    let stripped = line.trim_start_matches("async ");
    if let Some(rest) = stripped.strip_prefix("def ") {
        return ident(rest, &['(']).map(|n| (ChunkKind::Function, n));
    }
    if let Some(rest) = stripped.strip_prefix("class ") {
        return ident(rest, &['(', ':']).map(|n| (ChunkKind::Type, n));
    }
    if line.starts_with("import ") || line.starts_with("from ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    None
}

// ── JavaScript / TypeScript ────────────────────────────────────────

fn extract_js_ts(line: &str) -> Option<(ChunkKind, String)> {
    let stripped = line
        .trim_start_matches("export ")
        .trim_start_matches("default ")
        .trim_start_matches("async ")
        .trim_start_matches("abstract ")
        .trim_start_matches("declare ");

    if let Some(rest) = stripped.strip_prefix("function ") {
        let name = ident(rest, &['(', '<', ' '])?;
        if name != "*" {
            return Some((ChunkKind::Function, name));
        }
    }
    if let Some(rest) = stripped.strip_prefix("class ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("interface ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("type ") {
        return ident(rest, &[' ', '=', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("enum ") {
        return ident(rest, &[' ', '{']).map(|n| (ChunkKind::Type, n));
    }
    // Arrow functions: const foo = (...) =>
    if let Some(rest) = stripped
        .strip_prefix("const ")
        .or_else(|| stripped.strip_prefix("let "))
        && (rest.contains("=>") || rest.contains("function"))
    {
        return ident(rest, &[' ', '=', ':']).map(|n| (ChunkKind::Function, n));
    }
    if line.starts_with("import ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    None
}

// ── Java ───────────────────────────────────────────────────────────

fn extract_java(line: &str) -> Option<(ChunkKind, String)> {
    let stripped = strip_java_modifiers(line);

    if let Some(rest) = stripped.strip_prefix("class ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("interface ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("enum ") {
        return ident(rest, &[' ', '{', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("record ") {
        return ident(rest, &[' ', '(', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("@interface ") {
        return ident(rest, &[' ', '{']).map(|n| (ChunkKind::Type, n));
    }

    // Method: returnType name(...)  — look for '(' on lines with a return type
    if stripped.contains('(')
        && !stripped.starts_with("if ")
        && !stripped.starts_with("for ")
        && !stripped.starts_with("while ")
        && !stripped.starts_with("switch ")
        && !stripped.starts_with("return ")
        && !stripped.starts_with("new ")
        && !stripped.starts_with("super(")
        && !stripped.starts_with("this(")
    {
        // Try to extract: ReturnType name( or just name(
        if let Some(method_name) = extract_java_method_name(stripped) {
            return Some((ChunkKind::Function, method_name));
        }
    }

    if line.starts_with("import ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    if line.starts_with("package ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    None
}

fn strip_java_modifiers(line: &str) -> &str {
    let mut s = line;
    let modifiers = [
        "public ",
        "private ",
        "protected ",
        "static ",
        "final ",
        "abstract ",
        "synchronized ",
        "native ",
        "default ",
    ];
    loop {
        let before = s;
        for m in &modifiers {
            if let Some(rest) = s.strip_prefix(m) {
                s = rest;
            }
        }
        // Also strip annotations on the same line: @Override etc.
        if s.starts_with('@')
            && let Some(after_space) = s.find(' ')
        {
            s = s[after_space + 1..].trim_start();
            continue;
        }
        if s == before {
            break;
        }
    }
    s
}

fn extract_java_method_name(stripped: &str) -> Option<String> {
    let paren_pos = stripped.find('(')?;
    let before_paren = stripped[..paren_pos].trim();
    // Last word before '(' is the method name
    let name = before_paren.rsplit_once(' ')?.1;
    // Validate: must be a valid identifier
    if !name.is_empty()
        && name.chars().next()?.is_alphabetic()
        && name.chars().all(|c| c.is_alphanumeric() || c == '_')
    {
        Some(name.to_string())
    } else {
        None
    }
}

// ── Ruby ───────────────────────────────────────────────────────────

fn extract_ruby(line: &str) -> Option<(ChunkKind, String)> {
    if let Some(rest) = line.strip_prefix("def ") {
        // self.method_name or method_name
        let rest = rest.strip_prefix("self.").unwrap_or(rest);
        return ident(rest, &['(', ' ', ';']).map(|n| (ChunkKind::Function, n));
    }
    if let Some(rest) = line.strip_prefix("class ") {
        return ident(rest, &[' ', '<']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = line.strip_prefix("module ") {
        return ident(rest, &[' ', ';']).map(|n| (ChunkKind::Type, n));
    }
    if line.starts_with("require ") || line.starts_with("require_relative ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    if line.starts_with("include ") || line.starts_with("extend ") {
        return Some((ChunkKind::Import, line.to_string()));
    }
    None
}

// ── C / C++ ────────────────────────────────────────────────────────

fn extract_c_cpp(line: &str) -> Option<(ChunkKind, String)> {
    let stripped = line
        .trim_start_matches("static ")
        .trim_start_matches("inline ")
        .trim_start_matches("extern ")
        .trim_start_matches("virtual ")
        .trim_start_matches("explicit ");

    // Preprocessor includes
    if line.starts_with("#include") {
        return Some((ChunkKind::Import, line.to_string()));
    }

    // struct/class/enum/union/namespace
    if let Some(rest) = stripped.strip_prefix("struct ") {
        return ident(rest, &[' ', '{', ':', ';']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("class ") {
        return ident(rest, &[' ', '{', ':', ';']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("enum ") {
        let rest = rest.strip_prefix("class ").unwrap_or(rest);
        return ident(rest, &[' ', '{', ':', ';']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("union ") {
        return ident(rest, &[' ', '{', ';']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("namespace ") {
        return ident(rest, &[' ', '{']).map(|n| (ChunkKind::Type, n));
    }
    if let Some(rest) = stripped.strip_prefix("typedef ") {
        // typedef ... Name; → last word before ';'
        if let Some(name) = rest.trim_end_matches(';').rsplit_once(' ') {
            let n = name.1.trim();
            if !n.is_empty() && n.chars().next()?.is_alphabetic() {
                return Some((ChunkKind::Type, n.to_string()));
            }
        }
    }
    if let Some(rest) = stripped.strip_prefix("template") {
        // template<...> class/struct
        if let Some(after_angle) = rest.find('>') {
            let after = rest[after_angle + 1..].trim();
            return extract_c_cpp(after);
        }
    }

    // Function: type name(...) { or type name(...);
    if stripped.contains('(')
        && !stripped.starts_with("if ")
        && !stripped.starts_with("for ")
        && !stripped.starts_with("while ")
        && !stripped.starts_with("switch ")
        && !stripped.starts_with("return ")
        && !stripped.starts_with("case ")
        && !stripped.starts_with("else")
        && let Some(name) = extract_c_function_name(stripped)
    {
        return Some((ChunkKind::Function, name));
    }

    None
}

fn extract_c_function_name(line: &str) -> Option<String> {
    let paren_pos = line.find('(')?;
    let before_paren = line[..paren_pos].trim();
    if before_paren.is_empty() {
        return None;
    }
    // Handle pointer returns: int *func_name -> split on '*' too
    let name = before_paren.rsplit([' ', '*']).find(|s| !s.is_empty())?;
    // Skip if the name starts with a digit or is a keyword
    if name.chars().next()?.is_alphabetic()
        && name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == ':')
    {
        // Skip common macros / keywords
        if matches!(name, "sizeof" | "typeof" | "alignof" | "defined") {
            return None;
        }
        Some(name.to_string())
    } else {
        None
    }
}

// ── Helpers ────────────────────────────────────────────────────────

/// Extract the first identifier token from `rest`, splitting on any char in `delims`.
fn ident(rest: &str, delims: &[char]) -> Option<String> {
    let name = rest.split(delims).next()?.trim();
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Rust ───────────────────────────────────────────────────────

    #[test]
    fn rust_functions() {
        let src = "pub async fn authenticate(token: &str) -> bool {\n    true\n}\n";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, ChunkKind::Function);
        assert_eq!(chunks[0].name, "authenticate");
    }

    #[test]
    fn rust_types_and_impls() {
        let src = "\
pub struct Config<T> {
    name: String,
}

pub enum Status {
    Active,
}

pub trait Handler {
    fn handle(&self);
}

impl Config<String> {
    pub fn new() -> Self { todo!() }
}
";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        let kinds: Vec<_> = chunks.iter().map(|c| c.kind).collect();
        assert!(kinds.contains(&ChunkKind::Type)); // struct, enum, trait
        assert!(kinds.contains(&ChunkKind::Impl));
        assert!(kinds.contains(&ChunkKind::Function)); // new, handle
        assert!(chunks.iter().any(|c| c.name == "Config"));
        assert!(chunks.iter().any(|c| c.name == "Status"));
        assert!(chunks.iter().any(|c| c.name == "Handler"));
    }

    #[test]
    fn rust_imports() {
        let src = "use std::collections::HashMap;\nuse crate::Foo;\n";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        assert_eq!(chunks.len(), 2);
        assert!(chunks.iter().all(|c| c.kind == ChunkKind::Import));
    }

    #[test]
    fn rust_type_alias() {
        let src = "pub type Result<T> = std::result::Result<T, Error>;\n";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, ChunkKind::Type);
        assert_eq!(chunks[0].name, "Result");
    }

    // ── Go ─────────────────────────────────────────────────────────

    #[test]
    fn go_functions() {
        let src = "func main() {\n}\n\nfunc (s *Server) Handle(w http.ResponseWriter) {\n}\n";
        let chunks = RegexChunker.chunk(src, Language::Go);
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "main" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Handle" && c.kind == ChunkKind::Function)
        );
    }

    #[test]
    fn go_types_and_imports() {
        let src = "type Config struct {\n\tName string\n}\n\nimport (\n\t\"fmt\"\n)\n";
        let chunks = RegexChunker.chunk(src, Language::Go);
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Config" && c.kind == ChunkKind::Type)
        );
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Import));
    }

    // ── Python ─────────────────────────────────────────────────────

    #[test]
    fn python_functions_and_classes() {
        let src = "\
class UserService:
    def authenticate(self, token):
        return True

async def fetch_data(url):
    pass

import os
from pathlib import Path
";
        let chunks = RegexChunker.chunk(src, Language::Python);
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
                == 2
        );
    }

    // ── JavaScript / TypeScript ────────────────────────────────────

    #[test]
    fn js_ts_functions_and_classes() {
        let src = "\
export function authenticate(token) {
    return true;
}

export class UserService {
    constructor() {}
}

const fetchData = async (url) => {
    return fetch(url);
};

import { useState } from 'react';
";
        let chunks = RegexChunker.chunk(src, Language::TypeScript);
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
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "fetchData" && c.kind == ChunkKind::Function)
        );
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Import));
    }

    #[test]
    fn ts_interfaces_and_types() {
        let src = "\
export interface Config {
    name: string;
}

export type Result<T> = Success<T> | Error;

export enum Status {
    Active,
    Inactive,
}
";
        let chunks = RegexChunker.chunk(src, Language::TypeScript);
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Config" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Result" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Status" && c.kind == ChunkKind::Type)
        );
    }

    // ── Java ───────────────────────────────────────────────────────

    #[test]
    fn java_classes_and_methods() {
        let src = "\
public class UserService {
    public void authenticate(String token) {
        return;
    }

    private static boolean isValid(String s) {
        return true;
    }
}
";
        let chunks = RegexChunker.chunk(src, Language::Java);
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
                .any(|c| c.name == "isValid" && c.kind == ChunkKind::Function)
        );
    }

    #[test]
    fn java_interfaces_and_enums() {
        let src = "\
public interface Handler {
    void handle();
}

public enum Status {
    ACTIVE, INACTIVE
}

import java.util.List;
";
        let chunks = RegexChunker.chunk(src, Language::Java);
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Handler" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Status" && c.kind == ChunkKind::Type)
        );
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Import));
    }

    // ── Ruby ───────────────────────────────────────────────────────

    #[test]
    fn ruby_classes_and_methods() {
        let src = "\
class UserService
  def authenticate(token)
    true
  end

  def self.create(attrs)
    new(attrs)
  end
end

module Auth
end

require 'json'
require_relative 'helper'
";
        let chunks = RegexChunker.chunk(src, Language::Ruby);
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
                .any(|c| c.name == "create" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Auth" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .filter(|c| c.kind == ChunkKind::Import)
                .count()
                == 2
        );
    }

    // ── C / C++ ────────────────────────────────────────────────────

    #[test]
    fn c_functions_and_types() {
        let src = "\
#include <stdio.h>
#include \"myheader.h\"

struct Config {
    char *name;
};

void authenticate(const char *token) {
    return;
}

static int *parse_data(char *buf) {
    return NULL;
}
";
        let chunks = RegexChunker.chunk(src, Language::C);
        assert!(
            chunks
                .iter()
                .filter(|c| c.kind == ChunkKind::Import)
                .count()
                == 2
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Config" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "authenticate" && c.kind == ChunkKind::Function)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "parse_data" && c.kind == ChunkKind::Function)
        );
    }

    #[test]
    fn cpp_classes_and_namespaces() {
        let src = "\
namespace mylib {

class Handler {
public:
    virtual void handle() = 0;
};

enum class Status {
    Active,
    Inactive,
};

}
";
        let chunks = RegexChunker.chunk(src, Language::Cpp);
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "mylib" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Handler" && c.kind == ChunkKind::Type)
        );
        assert!(
            chunks
                .iter()
                .any(|c| c.name == "Status" && c.kind == ChunkKind::Type)
        );
    }

    #[test]
    fn cpp_typedef() {
        let src = "typedef unsigned long size_t;\n";
        let chunks = RegexChunker.chunk(src, Language::Cpp);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].kind, ChunkKind::Type);
        assert_eq!(chunks[0].name, "size_t");
    }

    // ── Edge cases ─────────────────────────────────────────────────

    #[test]
    fn empty_content() {
        let chunks = RegexChunker.chunk("", Language::Rust);
        assert!(chunks.is_empty());
    }

    #[test]
    fn unsupported_language_returns_empty() {
        let chunks = RegexChunker.chunk("fn main() {}", Language::Markdown);
        assert!(chunks.is_empty());
    }

    #[test]
    fn line_numbers_are_correct() {
        let src = "\n\nfn hello() {}\n";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start_line, 3);
    }

    #[test]
    fn comments_are_skipped() {
        let src = "// fn not_a_function() {}\nfn real_function() {}\n";
        let chunks = RegexChunker.chunk(src, Language::Rust);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].name, "real_function");
    }
}
