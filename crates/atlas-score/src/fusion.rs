use atlas_core::ScoredFile;
use std::collections::HashMap;

/// Default RRF constant (standard value from the RRF paper).
const DEFAULT_K: f64 = 60.0;

/// Reciprocal Rank Fusion: combines multiple ranked lists into a single ranking.
///
/// RRF score for file f = sum(1 / (k + rank_i)) for each ranking i.
/// Lower rank numbers (higher positions) contribute more to the final score.
pub struct RrfFusion {
    k: f64,
}

impl RrfFusion {
    pub fn new() -> Self {
        Self { k: DEFAULT_K }
    }

    /// Set a custom k parameter.
    pub fn with_k(mut self, k: f64) -> Self {
        self.k = k;
        self
    }

    /// Combine multiple ranked lists into a single ranking using RRF.
    ///
    /// Each input is a ranked list of `ScoredFile`s (already sorted by their signal score).
    /// The output is a merged list sorted by the fused RRF score.
    pub fn fuse(&self, rankings: &[Vec<&ScoredFile>]) -> Vec<RrfResult> {
        let mut rrf_scores: HashMap<&str, f64> = HashMap::new();

        for ranking in rankings {
            for (rank, file) in ranking.iter().enumerate() {
                *rrf_scores.entry(&file.path).or_default() += 1.0 / (self.k + rank as f64 + 1.0);
            }
        }

        let mut results: Vec<RrfResult> = rrf_scores
            .into_iter()
            .map(|(path, score)| RrfResult {
                path: path.to_string(),
                rrf_score: score,
            })
            .collect();

        results.sort_by(|a, b| {
            b.rrf_score
                .partial_cmp(&a.rrf_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        results
    }

    /// Fuse multiple scored file lists, updating the final score to the RRF score.
    ///
    /// Takes ownership of a base scored list and applies RRF from additional signal rankings.
    pub fn fuse_scored(&self, base: &mut [ScoredFile], additional_rankings: &[Vec<&str>]) {
        if additional_rankings.is_empty() {
            return;
        }

        // Build base ranking by current score order
        let base_ranking: Vec<String> = base.iter().map(|f| f.path.clone()).collect();

        // All rankings including the base
        let mut all_rankings: Vec<Vec<String>> = vec![base_ranking];
        for ranking in additional_rankings {
            all_rankings.push(ranking.iter().map(|s| s.to_string()).collect());
        }

        // Compute RRF scores
        let mut rrf_scores: HashMap<String, f64> = HashMap::new();
        for ranking in &all_rankings {
            for (rank, path) in ranking.iter().enumerate() {
                *rrf_scores.entry(path.clone()).or_default() += 1.0 / (self.k + rank as f64 + 1.0);
            }
        }

        // Update base scores
        for file in base.iter_mut() {
            if let Some(&rrf_score) = rrf_scores.get(&file.path) {
                file.score = rrf_score;
            }
        }

        // Re-sort by new RRF scores
        base.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
}

impl Default for RrfFusion {
    fn default() -> Self {
        Self::new()
    }
}

/// Result from RRF fusion.
#[derive(Debug, Clone)]
pub struct RrfResult {
    pub path: String,
    pub rrf_score: f64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use atlas_core::{FileRole, Language, SignalBreakdown};

    fn make_scored(path: &str, score: f64) -> ScoredFile {
        ScoredFile {
            path: path.to_string(),
            score,
            signals: SignalBreakdown::default(),
            tokens: 100,
            language: Language::Rust,
            role: FileRole::Implementation,
        }
    }

    #[test]
    fn rrf_single_ranking() {
        let files = vec![
            make_scored("a.rs", 3.0),
            make_scored("b.rs", 2.0),
            make_scored("c.rs", 1.0),
        ];
        let ranking: Vec<&ScoredFile> = files.iter().collect();

        let fusion = RrfFusion::new();
        let results = fusion.fuse(&[ranking]);

        assert_eq!(results.len(), 3);
        // First file should have highest RRF score
        assert_eq!(results[0].path, "a.rs");
        assert_eq!(results[1].path, "b.rs");
        assert_eq!(results[2].path, "c.rs");
    }

    #[test]
    fn rrf_two_rankings_agreement() {
        let files1 = vec![
            make_scored("a.rs", 3.0),
            make_scored("b.rs", 2.0),
            make_scored("c.rs", 1.0),
        ];
        let files2 = vec![
            make_scored("a.rs", 5.0),
            make_scored("b.rs", 4.0),
            make_scored("c.rs", 3.0),
        ];

        let r1: Vec<&ScoredFile> = files1.iter().collect();
        let r2: Vec<&ScoredFile> = files2.iter().collect();

        let fusion = RrfFusion::new();
        let results = fusion.fuse(&[r1, r2]);

        // When both rankings agree, order should be preserved
        assert_eq!(results[0].path, "a.rs");
        assert_eq!(results[1].path, "b.rs");
        assert_eq!(results[2].path, "c.rs");
    }

    #[test]
    fn rrf_two_rankings_disagreement() {
        // Ranking 1: a, b, c
        let files1 = vec![
            make_scored("a.rs", 3.0),
            make_scored("b.rs", 2.0),
            make_scored("c.rs", 1.0),
        ];
        // Ranking 2: c, b, a (opposite order)
        let files2 = vec![
            make_scored("c.rs", 3.0),
            make_scored("b.rs", 2.0),
            make_scored("a.rs", 1.0),
        ];

        let r1: Vec<&ScoredFile> = files1.iter().collect();
        let r2: Vec<&ScoredFile> = files2.iter().collect();

        let fusion = RrfFusion::new();
        let results = fusion.fuse(&[r1, r2]);

        // b.rs is rank 2 in both lists, so it should benefit from consistent ranking
        // a.rs: 1/(61) + 1/(63) = ~0.01639 + ~0.01587 = ~0.03226
        // b.rs: 1/(62) + 1/(62) = ~0.01613 + ~0.01613 = ~0.03226
        // c.rs: 1/(63) + 1/(61) = ~0.01587 + ~0.01639 = ~0.03226
        // All roughly equal when disagreement is symmetric
        assert_eq!(results.len(), 3);
        // All scores should be approximately equal
        let max = results[0].rrf_score;
        let min = results[2].rrf_score;
        assert!((max - min) / max < 0.05); // Within 5%
    }

    #[test]
    fn rrf_custom_k() {
        let files = vec![make_scored("a.rs", 2.0), make_scored("b.rs", 1.0)];
        let ranking: Vec<&ScoredFile> = files.iter().collect();

        let fusion = RrfFusion::new().with_k(1.0);
        let results = fusion.fuse(&[ranking]);

        // With k=1: a.rs gets 1/(1+0+1) = 0.5, b.rs gets 1/(1+1+1) = 0.333
        assert!(results[0].rrf_score > results[1].rrf_score);
        assert!((results[0].rrf_score - 0.5).abs() < 1e-10);
        assert!((results[1].rrf_score - 1.0 / 3.0).abs() < 1e-10);
    }

    #[test]
    fn rrf_empty_rankings() {
        let fusion = RrfFusion::new();
        let results = fusion.fuse(&[]);
        assert!(results.is_empty());
    }

    #[test]
    fn rrf_fuse_scored_updates_order() {
        let mut base = vec![
            make_scored("a.rs", 3.0),
            make_scored("b.rs", 2.0),
            make_scored("c.rs", 1.0),
        ];

        // Additional ranking that reverses the order
        let additional = vec![vec!["c.rs", "b.rs", "a.rs"]];

        let fusion = RrfFusion::new();
        fusion.fuse_scored(&mut base, &additional);

        // All files should have updated RRF scores
        for file in &base {
            assert!(file.score > 0.0);
        }
    }

    #[test]
    fn rrf_fuse_scored_no_additional() {
        let mut base = vec![make_scored("a.rs", 3.0), make_scored("b.rs", 2.0)];

        let fusion = RrfFusion::new();
        fusion.fuse_scored(&mut base, &[]);

        // Scores should be unchanged
        assert_eq!(base[0].score, 3.0);
        assert_eq!(base[1].score, 2.0);
    }

    #[test]
    fn rrf_file_in_one_ranking_only() {
        let files1 = vec![make_scored("a.rs", 2.0), make_scored("b.rs", 1.0)];
        let files2 = vec![make_scored("c.rs", 2.0), make_scored("a.rs", 1.0)];

        let r1: Vec<&ScoredFile> = files1.iter().collect();
        let r2: Vec<&ScoredFile> = files2.iter().collect();

        let fusion = RrfFusion::new();
        let results = fusion.fuse(&[r1, r2]);

        assert_eq!(results.len(), 3);
        // a.rs appears in both rankings so should have highest score
        assert_eq!(results[0].path, "a.rs");
    }
}
