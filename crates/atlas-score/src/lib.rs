//! BM25F, heuristic, structural, and RRF fusion scoring.

mod bm25f;
mod fusion;
mod git_recency;
mod heuristic;
mod pagerank;
mod tokenizer;

pub mod hybrid;

pub use bm25f::{Bm25fScorer, CorpusStats};
pub use fusion::{RrfFusion, RrfResult};
pub use git_recency::{file_recency, git_recency_scores};
pub use heuristic::HeuristicScorer;
pub use hybrid::HybridScorer;
pub use pagerank::{ImportGraph, extract_imports};
pub use tokenizer::Tokenizer;

#[cfg(test)]
mod tests {
    use super::*;

    // --- Tokenizer tests ---

    #[test]
    fn tokenize_simple_words() {
        let tokens = Tokenizer::tokenize("hello world");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_camel_case() {
        let tokens = Tokenizer::tokenize("insertBreak");
        assert_eq!(tokens, vec!["insert", "break"]);
    }

    #[test]
    fn tokenize_pascal_case() {
        let tokens = Tokenizer::tokenize("FileInfo");
        assert_eq!(tokens, vec!["file", "info"]);
    }

    #[test]
    fn tokenize_snake_case() {
        let tokens = Tokenizer::tokenize("file_info");
        assert_eq!(tokens, vec!["file", "info"]);
    }

    #[test]
    fn tokenize_removes_stop_words() {
        let tokens = Tokenizer::tokenize("the quick brown fox");
        assert!(!tokens.contains(&"the".to_string()));
        assert!(tokens.contains(&"quick".to_string()));
        assert!(tokens.contains(&"brown".to_string()));
        assert!(tokens.contains(&"fox".to_string()));
    }

    #[test]
    fn tokenize_lowercase_normalization() {
        let tokens = Tokenizer::tokenize("HELLO World");
        assert_eq!(tokens, vec!["hello", "world"]);
    }

    #[test]
    fn tokenize_mixed_separators() {
        let tokens = Tokenizer::tokenize("get_userName from HTTP");
        assert!(tokens.contains(&"get".to_string()));
        assert!(tokens.contains(&"user".to_string()));
        assert!(tokens.contains(&"name".to_string()));
        assert!(tokens.contains(&"http".to_string()));
    }

    #[test]
    fn tokenize_empty_string() {
        let tokens = Tokenizer::tokenize("");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_only_stop_words() {
        let tokens = Tokenizer::tokenize("the a an is are");
        assert!(tokens.is_empty());
    }

    #[test]
    fn tokenize_acronyms() {
        let tokens = Tokenizer::tokenize("parseHTTPResponse");
        assert!(tokens.contains(&"parse".to_string()));
        assert!(tokens.contains(&"http".to_string()));
        assert!(tokens.contains(&"response".to_string()));
    }

    #[test]
    fn tokenize_numbers_mixed() {
        let tokens = Tokenizer::tokenize("file2path");
        // Should handle numbers in identifiers
        assert!(!tokens.is_empty());
    }

    #[test]
    fn tokenize_path_separators() {
        let tokens = Tokenizer::tokenize("src/auth/middleware.rs");
        assert!(tokens.contains(&"src".to_string()));
        assert!(tokens.contains(&"auth".to_string()));
        assert!(tokens.contains(&"middleware".to_string()));
    }

    // --- Heuristic scorer tests ---

    #[test]
    fn heuristic_score_is_bounded() {
        let scorer = HeuristicScorer::new("auth middleware");
        let score = scorer.score(
            "src/auth/middleware.rs",
            atlas_core::FileRole::Implementation,
            500,
        );
        assert!(score >= 0.0);
        assert!(score <= 1.0);
    }

    #[test]
    fn heuristic_keyword_match_boosts_score() {
        let scorer = HeuristicScorer::new("auth");
        let with_match = scorer.score(
            "src/auth/handler.rs",
            atlas_core::FileRole::Implementation,
            500,
        );
        let without_match = scorer.score(
            "src/utils/helper.rs",
            atlas_core::FileRole::Implementation,
            500,
        );
        assert!(with_match > without_match);
    }

    #[test]
    fn heuristic_impl_scores_higher_than_test() {
        let scorer = HeuristicScorer::new("handler");
        let impl_score = scorer.score("src/handler.rs", atlas_core::FileRole::Implementation, 500);
        let test_score = scorer.score("tests/handler_test.rs", atlas_core::FileRole::Test, 500);
        assert!(impl_score > test_score);
    }

    #[test]
    fn heuristic_shallow_files_score_higher() {
        let scorer = HeuristicScorer::new("main");
        let shallow = scorer.score("src/main.rs", atlas_core::FileRole::Implementation, 500);
        let deep = scorer.score(
            "src/deeply/nested/path/main.rs",
            atlas_core::FileRole::Implementation,
            500,
        );
        assert!(shallow > deep);
    }

    #[test]
    fn heuristic_large_files_penalized() {
        let scorer = HeuristicScorer::new("utils");
        let small = scorer.score("src/utils.rs", atlas_core::FileRole::Implementation, 500);
        let large = scorer.score(
            "src/utils.rs",
            atlas_core::FileRole::Implementation,
            500_000,
        );
        assert!(small > large);
    }

    #[test]
    fn heuristic_wellknown_paths_boosted() {
        let scorer = HeuristicScorer::new("module");
        let src = scorer.score("src/module.rs", atlas_core::FileRole::Implementation, 500);
        let random = scorer.score(
            "random/module.rs",
            atlas_core::FileRole::Implementation,
            500,
        );
        assert!(src > random);
    }

    #[test]
    fn heuristic_empty_query() {
        let scorer = HeuristicScorer::new("");
        let score = scorer.score("src/main.rs", atlas_core::FileRole::Implementation, 500);
        assert!(score >= 0.0);
    }

    #[test]
    fn heuristic_generated_files_penalized() {
        let scorer = HeuristicScorer::new("errors");
        let impl_score = scorer.score("src/errors.rs", atlas_core::FileRole::Implementation, 500);
        let gen_score = scorer.score("generated/errors.rs", atlas_core::FileRole::Generated, 500);
        assert!(impl_score > gen_score);
    }
}
