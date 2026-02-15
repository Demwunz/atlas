use crate::bm25f::{Bm25fScorer, CorpusStats};
use crate::heuristic::HeuristicScorer;
use atlas_core::{FileInfo, ScoredFile, SignalBreakdown};
use std::collections::HashMap;

/// Default weight for BM25F in hybrid scoring.
const DEFAULT_BM25F_WEIGHT: f64 = 0.6;
/// Default weight for heuristic in hybrid scoring.
const DEFAULT_HEURISTIC_WEIGHT: f64 = 0.4;

/// Hybrid scorer combining BM25F (content relevance) and heuristic (path-based) signals.
pub struct HybridScorer {
    bm25f_weight: f64,
    heuristic_weight: f64,
    query: String,
}

impl HybridScorer {
    pub fn new(query: &str) -> Self {
        Self {
            bm25f_weight: DEFAULT_BM25F_WEIGHT,
            heuristic_weight: DEFAULT_HEURISTIC_WEIGHT,
            query: query.to_string(),
        }
    }

    /// Set custom weights. They will be normalized to sum to 1.0.
    pub fn weights(mut self, bm25f: f64, heuristic: f64) -> Self {
        let total = bm25f + heuristic;
        if total > 0.0 {
            self.bm25f_weight = bm25f / total;
            self.heuristic_weight = heuristic / total;
        }
        self
    }

    /// Score a set of files and return them sorted by score (descending).
    pub fn score(&self, files: &[FileInfo]) -> Vec<ScoredFile> {
        if files.is_empty() {
            return Vec::new();
        }

        // Build BM25F corpus stats from file paths (shallow mode)
        let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
        let stats = CorpusStats::from_paths(&paths);
        let bm25f = Bm25fScorer::new(&self.query, stats);
        let heuristic = HeuristicScorer::new(&self.query);

        let mut scored: Vec<ScoredFile> = files
            .iter()
            .map(|f| {
                let bm25f_score = bm25f.score_path(&f.path);
                let heuristic_score = heuristic.score(&f.path, f.role, f.size);

                let combined =
                    self.bm25f_weight * bm25f_score + self.heuristic_weight * heuristic_score;

                ScoredFile {
                    path: f.path.clone(),
                    score: combined,
                    signals: SignalBreakdown {
                        bm25f: bm25f_score,
                        heuristic: heuristic_score,
                        pagerank: None,
                        git_recency: None,
                        embedding: None,
                    },
                    tokens: f.estimated_tokens(),
                    language: f.language,
                    role: f.role,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }

    /// Score files with full term frequencies from the deep index.
    pub fn score_with_index(
        &self,
        files: &[FileInfo],
        term_freqs: &HashMap<String, (HashMap<String, atlas_core::TermFreqs>, u32)>,
        stats: CorpusStats,
    ) -> Vec<ScoredFile> {
        if files.is_empty() {
            return Vec::new();
        }

        let bm25f = Bm25fScorer::new(&self.query, stats);
        let heuristic = HeuristicScorer::new(&self.query);

        let mut scored: Vec<ScoredFile> = files
            .iter()
            .map(|f| {
                let bm25f_score = if let Some((tf, dl)) = term_freqs.get(&f.path) {
                    bm25f.score(tf, *dl)
                } else {
                    bm25f.score_path(&f.path)
                };
                let heuristic_score = heuristic.score(&f.path, f.role, f.size);

                let combined =
                    self.bm25f_weight * bm25f_score + self.heuristic_weight * heuristic_score;

                ScoredFile {
                    path: f.path.clone(),
                    score: combined,
                    signals: SignalBreakdown {
                        bm25f: bm25f_score,
                        heuristic: heuristic_score,
                        pagerank: None,
                        git_recency: None,
                        embedding: None,
                    },
                    tokens: f.estimated_tokens(),
                    language: f.language,
                    role: f.role,
                }
            })
            .collect();

        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{FileRole, Language};

    fn sample_files() -> Vec<FileInfo> {
        vec![
            FileInfo {
                path: "src/auth/handler.rs".to_string(),
                size: 2000,
                language: Language::Rust,
                role: FileRole::Implementation,
                sha256: [0u8; 32],
            },
            FileInfo {
                path: "src/auth/middleware.rs".to_string(),
                size: 1500,
                language: Language::Rust,
                role: FileRole::Implementation,
                sha256: [0u8; 32],
            },
            FileInfo {
                path: "src/db/connection.rs".to_string(),
                size: 3000,
                language: Language::Rust,
                role: FileRole::Implementation,
                sha256: [0u8; 32],
            },
            FileInfo {
                path: "tests/auth_test.rs".to_string(),
                size: 800,
                language: Language::Rust,
                role: FileRole::Test,
                sha256: [0u8; 32],
            },
            FileInfo {
                path: "README.md".to_string(),
                size: 500,
                language: Language::Markdown,
                role: FileRole::Documentation,
                sha256: [0u8; 32],
            },
        ]
    }

    #[test]
    fn hybrid_returns_sorted_results() {
        let scorer = HybridScorer::new("auth handler");
        let results = scorer.score(&sample_files());

        assert_eq!(results.len(), 5);
        // Results should be sorted descending by score
        for w in results.windows(2) {
            assert!(w[0].score >= w[1].score);
        }
    }

    #[test]
    fn hybrid_relevant_files_rank_higher() {
        let scorer = HybridScorer::new("auth");
        let results = scorer.score(&sample_files());

        // Auth files should be in top positions
        let top_paths: Vec<&str> = results.iter().take(3).map(|f| f.path.as_str()).collect();
        assert!(top_paths.contains(&"src/auth/handler.rs"));
        assert!(top_paths.contains(&"src/auth/middleware.rs"));
    }

    #[test]
    fn hybrid_signals_populated() {
        let scorer = HybridScorer::new("auth");
        let results = scorer.score(&sample_files());

        for result in &results {
            // Heuristic should always have a value
            assert!(result.signals.heuristic >= 0.0);
            // BM25F may be 0 for non-matching files
            assert!(result.signals.bm25f >= 0.0);
            // Optional signals should be None in shallow mode
            assert!(result.signals.pagerank.is_none());
            assert!(result.signals.git_recency.is_none());
        }
    }

    #[test]
    fn hybrid_custom_weights() {
        let files = sample_files();

        // All BM25F weight
        let bm25f_only = HybridScorer::new("auth").weights(1.0, 0.0).score(&files);

        // All heuristic weight
        let heuristic_only = HybridScorer::new("auth").weights(0.0, 1.0).score(&files);

        // Scores should differ between the two weighting schemes
        // Both should rank auth files highly, but ordering may differ
        assert!(bm25f_only[0].score > 0.0);
        assert!(heuristic_only[0].score > 0.0);

        // Verify the signal breakdown matches the weighting
        assert_eq!(bm25f_only[0].signals.bm25f, bm25f_only[0].score);
        assert_eq!(heuristic_only[0].signals.heuristic, heuristic_only[0].score);
    }

    #[test]
    fn hybrid_empty_files() {
        let scorer = HybridScorer::new("auth");
        let results = scorer.score(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn hybrid_empty_query() {
        let scorer = HybridScorer::new("");
        let results = scorer.score(&sample_files());
        // Should still return files, scored by heuristic only
        assert_eq!(results.len(), 5);
    }

    #[test]
    fn hybrid_tokens_from_file_size() {
        let scorer = HybridScorer::new("auth");
        let results = scorer.score(&sample_files());

        let auth_file = results
            .iter()
            .find(|f| f.path == "src/auth/handler.rs")
            .unwrap();
        assert_eq!(auth_file.tokens, 2000 / 4); // size / 4 heuristic
    }
}
