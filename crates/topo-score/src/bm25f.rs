use crate::tokenizer::Tokenizer;
use std::collections::HashMap;
use topo_core::TermFreqs;

/// BM25F field weights.
const W_FILENAME: f64 = 5.0;
const W_SYMBOLS: f64 = 3.0;
const W_BODY: f64 = 1.0;

/// BM25F parameters.
const K1: f64 = 1.2;
const B: f64 = 0.75;

/// Precomputed corpus statistics needed for IDF calculation.
pub struct CorpusStats {
    pub total_docs: usize,
    pub avg_doc_length: f64,
    pub doc_frequencies: HashMap<String, usize>,
}

impl CorpusStats {
    /// Build corpus stats from a set of documents.
    ///
    /// Each document is represented as (path, term_frequencies, doc_length).
    pub fn from_documents<'a>(
        docs: impl Iterator<Item = (&'a str, &'a HashMap<String, TermFreqs>, u32)>,
    ) -> Self {
        let mut total_docs = 0usize;
        let mut total_length = 0u64;
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();

        for (_path, term_freqs, doc_length) in docs {
            total_docs += 1;
            total_length += doc_length as u64;
            for term in term_freqs.keys() {
                *doc_frequencies.entry(term.clone()).or_default() += 1;
            }
        }

        let avg_doc_length = if total_docs > 0 {
            total_length as f64 / total_docs as f64
        } else {
            1.0
        };

        Self {
            total_docs,
            avg_doc_length,
            doc_frequencies,
        }
    }

    /// Build corpus stats from shallow metadata (file paths only).
    ///
    /// In shallow mode, we tokenize just the file path to produce term frequencies
    /// for the filename field only. This enables BM25F scoring before the deep index
    /// is built.
    pub fn from_paths(paths: &[&str]) -> Self {
        let mut doc_frequencies: HashMap<String, usize> = HashMap::new();
        let mut total_length = 0u64;

        for path in paths {
            let tokens = Tokenizer::tokenize(path);
            let unique: std::collections::HashSet<&String> = tokens.iter().collect();
            for token in &unique {
                *doc_frequencies.entry((*token).clone()).or_default() += 1;
            }
            total_length += tokens.len() as u64;
        }

        let avg_doc_length = if paths.is_empty() {
            1.0
        } else {
            total_length as f64 / paths.len() as f64
        };

        Self {
            total_docs: paths.len(),
            avg_doc_length,
            doc_frequencies,
        }
    }
}

/// BM25F scorer using field-weighted term frequencies.
///
/// Field weights: filename=5.0, symbols=3.0, body=1.0.
/// Parameters: k1=1.2, b=0.75.
pub struct Bm25fScorer {
    query_tokens: Vec<String>,
    stats: CorpusStats,
}

impl Bm25fScorer {
    pub fn new(query: &str, stats: CorpusStats) -> Self {
        Self {
            query_tokens: Tokenizer::tokenize(query),
            stats,
        }
    }

    /// Compute BM25F score for a document given its term frequencies and doc length.
    pub fn score(&self, term_freqs: &HashMap<String, TermFreqs>, doc_length: u32) -> f64 {
        if self.query_tokens.is_empty() || self.stats.total_docs == 0 {
            return 0.0;
        }

        let n = self.stats.total_docs as f64;
        let avgdl = self.stats.avg_doc_length;
        let dl = doc_length as f64;

        // Length normalization factor
        let length_norm = 1.0 - B + B * (dl / avgdl);

        let mut score = 0.0;
        for token in &self.query_tokens {
            let df = self.stats.doc_frequencies.get(token).copied().unwrap_or(0) as f64;

            // IDF: log((N - df + 0.5) / (df + 0.5) + 1)
            let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();

            // Weighted term frequency across fields
            let tf = term_freqs
                .get(token)
                .map(|f| {
                    W_FILENAME * f.filename as f64
                        + W_SYMBOLS * f.symbols as f64
                        + W_BODY * f.body as f64
                })
                .unwrap_or(0.0);

            // BM25F formula: IDF * tf_weighted / (tf_weighted + k1 * length_norm)
            if tf > 0.0 {
                score += idf * tf / (tf + K1 * length_norm);
            }
        }

        score
    }

    /// Score a file using only its path (shallow mode).
    ///
    /// Tokenizes the path and puts all term frequencies into the filename field.
    pub fn score_path(&self, path: &str) -> f64 {
        let tokens = Tokenizer::tokenize(path);
        let mut term_freqs: HashMap<String, TermFreqs> = HashMap::new();
        for token in &tokens {
            term_freqs.entry(token.clone()).or_default().filename += 1;
        }
        let doc_length = tokens.len() as u32;
        self.score(&term_freqs, doc_length)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_paths() -> Vec<&'static str> {
        vec![
            "src/auth/handler.rs",
            "src/auth/middleware.rs",
            "src/db/connection.rs",
            "src/db/query.rs",
            "src/main.rs",
            "tests/auth_test.rs",
            "README.md",
        ]
    }

    #[test]
    fn bm25f_empty_query_returns_zero() {
        let stats = CorpusStats::from_paths(&sample_paths());
        let scorer = Bm25fScorer::new("", stats);
        let score = scorer.score_path("src/auth/handler.rs");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn bm25f_matching_term_scores_positive() {
        let stats = CorpusStats::from_paths(&sample_paths());
        let scorer = Bm25fScorer::new("auth", stats);
        let score = scorer.score_path("src/auth/handler.rs");
        assert!(score > 0.0);
    }

    #[test]
    fn bm25f_no_match_scores_zero() {
        let stats = CorpusStats::from_paths(&sample_paths());
        let scorer = Bm25fScorer::new("zebra", stats);
        let score = scorer.score_path("src/auth/handler.rs");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn bm25f_rarer_terms_score_higher() {
        let paths = sample_paths();
        let stats = CorpusStats::from_paths(&paths);

        // "connection" appears in 1 doc, "src" appears in 5 docs
        let scorer_rare = Bm25fScorer::new("connection", CorpusStats::from_paths(&paths));
        let scorer_common = Bm25fScorer::new("src", stats);

        let rare_score = scorer_rare.score_path("src/db/connection.rs");
        let common_score = scorer_common.score_path("src/db/connection.rs");

        // Rarer term should produce higher IDF and thus higher score
        assert!(rare_score > common_score);
    }

    #[test]
    fn bm25f_with_term_freqs() {
        let paths = sample_paths();
        let stats = CorpusStats::from_paths(&paths);
        let scorer = Bm25fScorer::new("auth", stats);

        let mut term_freqs = HashMap::new();
        term_freqs.insert(
            "auth".to_string(),
            TermFreqs {
                filename: 2,
                symbols: 3,
                body: 5,
            },
        );

        let score = scorer.score(&term_freqs, 100);
        assert!(score > 0.0);
    }

    #[test]
    fn bm25f_field_weights_matter() {
        let paths = sample_paths();

        // Same term, but in different fields
        let scorer = Bm25fScorer::new("auth", CorpusStats::from_paths(&paths));

        let mut filename_heavy = HashMap::new();
        filename_heavy.insert(
            "auth".to_string(),
            TermFreqs {
                filename: 3,
                symbols: 0,
                body: 0,
            },
        );

        let mut body_heavy = HashMap::new();
        body_heavy.insert(
            "auth".to_string(),
            TermFreqs {
                filename: 0,
                symbols: 0,
                body: 3,
            },
        );

        let filename_score = scorer.score(&filename_heavy, 10);
        let body_score = scorer.score(&body_heavy, 10);

        // filename weight (5.0) > body weight (1.0), so filename-heavy should score higher
        assert!(filename_score > body_score);
    }

    #[test]
    fn bm25f_multi_term_query() {
        let paths = sample_paths();
        let stats = CorpusStats::from_paths(&paths);
        let scorer = Bm25fScorer::new("auth handler", stats);

        let auth_handler = scorer.score_path("src/auth/handler.rs");
        let auth_only = scorer.score_path("src/auth/middleware.rs");

        // File matching both query terms should score higher
        assert!(auth_handler > auth_only);
    }

    #[test]
    fn bm25f_corpus_stats_from_paths() {
        let paths = sample_paths();
        let stats = CorpusStats::from_paths(&paths);

        assert_eq!(stats.total_docs, 7);
        assert!(stats.avg_doc_length > 0.0);
        // "auth" appears in 3 docs
        assert_eq!(stats.doc_frequencies.get("auth"), Some(&3));
    }

    #[test]
    fn bm25f_empty_corpus() {
        let stats = CorpusStats::from_paths(&[]);
        let scorer = Bm25fScorer::new("auth", stats);
        let score = scorer.score_path("src/auth/handler.rs");
        assert_eq!(score, 0.0);
    }

    #[test]
    fn bm25f_idf_correctness() {
        // With N=7 and df=3 for "auth":
        // IDF = ln((7 - 3 + 0.5) / (3 + 0.5) + 1) = ln(4.5/3.5 + 1) = ln(2.2857...)
        let paths = sample_paths();
        let stats = CorpusStats::from_paths(&paths);
        assert_eq!(stats.total_docs, 7);
        let df = *stats.doc_frequencies.get("auth").unwrap() as f64;
        let n = stats.total_docs as f64;
        let idf = ((n - df + 0.5) / (df + 0.5) + 1.0).ln();
        assert!(idf > 0.0);
        assert!(idf < 3.0); // Sanity check
    }
}
