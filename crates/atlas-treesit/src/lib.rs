//! Code chunking: extract functions, types, and imports from source files.
//!
//! Provides a regex-based chunker for all target languages. A tree-sitter
//! backend can be added behind a feature flag for more precise AST chunking.

mod regex_chunker;

pub use regex_chunker::RegexChunker;

use atlas_core::{Chunk, Language};

/// Trait for code chunk extraction.
pub trait Chunker {
    /// Extract code chunks from file content.
    fn chunk(&self, content: &str, language: Language) -> Vec<Chunk>;
}

/// Create the default chunker (regex-based).
pub fn default_chunker() -> RegexChunker {
    RegexChunker
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::ChunkKind;

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
}
