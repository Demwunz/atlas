//! Code chunking: extract functions, types, and imports from source files.
//!
//! Uses tree-sitter for precise AST chunking when a grammar is available,
//! with regex-based fallback for unsupported languages.

mod queries;
mod regex_chunker;
mod ts_chunker;

pub use regex_chunker::RegexChunker;
pub use ts_chunker::TreeSitterChunker;

use topo_core::{Chunk, Language};

/// Trait for code chunk extraction.
pub trait Chunker {
    /// Extract code chunks from file content.
    fn chunk(&self, content: &str, language: Language) -> Vec<Chunk>;
}

/// Composite chunker: tries tree-sitter first, falls back to regex.
pub struct CompositeChunker;

impl Chunker for CompositeChunker {
    fn chunk(&self, content: &str, language: Language) -> Vec<Chunk> {
        let ts_chunks = TreeSitterChunker.chunk(content, language);
        if !ts_chunks.is_empty() {
            return ts_chunks;
        }
        RegexChunker.chunk(content, language)
    }
}

/// Create the default chunker (regex-based, fast indexing).
///
/// Tree-sitter chunkers (`TreeSitterChunker`, `CompositeChunker`) remain
/// available for on-demand enrichment of selected files.
pub fn default_chunker() -> RegexChunker {
    RegexChunker
}

#[cfg(test)]
mod tests {
    use super::*;
    use topo_core::ChunkKind;

    #[test]
    fn default_chunker_works() {
        let chunker = default_chunker();
        let chunks = chunker.chunk("fn main() {}\n", Language::Rust);
        assert!(!chunks.is_empty());
        assert!(chunks.iter().any(|c| c.kind == ChunkKind::Function));
    }

    #[test]
    fn chunker_trait_object() {
        let chunker: Box<dyn Chunker> = Box::new(default_chunker());
        let chunks = chunker.chunk("def hello():\n    pass\n", Language::Python);
        assert!(!chunks.is_empty());
    }

    #[test]
    fn composite_prefers_tree_sitter() {
        let chunker = CompositeChunker;
        let src = "pub fn authenticate(token: &str) -> bool {\n    !token.is_empty()\n}\n";
        let chunks = chunker.chunk(src, Language::Rust);
        assert!(!chunks.is_empty());
        // Tree-sitter produces multi-line spans
        let f = chunks
            .iter()
            .find(|c| c.kind == ChunkKind::Function)
            .unwrap();
        assert!(
            f.end_line > f.start_line,
            "tree-sitter should give multi-line spans"
        );
    }

    #[test]
    fn composite_falls_back_to_regex() {
        let chunker = CompositeChunker;
        // Markdown has no tree-sitter query — should get empty from both
        let chunks = chunker.chunk("# heading", Language::Markdown);
        assert!(chunks.is_empty());
    }
}
