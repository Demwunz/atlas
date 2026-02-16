use crate::tokenizer::Tokenizer;
use atlas_core::FileRole;

/// Path-based heuristic scorer.
///
/// Scoring signals:
/// - Directory depth penalty (deeper = less relevant)
/// - Keyword match bonus (query terms in path segments)
/// - File role bonus (implementation > test > config > docs)
/// - Size penalty (very large files penalized)
/// - Well-known path bonus (src/, lib/, cmd/ get boost)
pub struct HeuristicScorer {
    query_tokens: Vec<String>,
}

impl HeuristicScorer {
    pub fn new(query: &str) -> Self {
        Self {
            query_tokens: Tokenizer::tokenize(query),
        }
    }

    /// Score a file path. Returns a value in [0.0, 1.0].
    pub fn score(&self, path: &str, role: FileRole, size: u64) -> f64 {
        let mut score = 0.0;

        // 1. Keyword match bonus (0.0 - 0.4)
        score += self.keyword_score(path) * 0.4;

        // 2. File role bonus (0.0 - 0.25)
        score += role_score(role) * 0.25;

        // 3. Depth penalty (0.0 - 0.15)
        score += depth_score(path) * 0.15;

        // 4. Well-known path bonus (0.0 - 0.1)
        score += wellknown_score(path) * 0.1;

        // 5. Size penalty (0.0 - 0.1)
        score += size_score(size) * 0.1;

        score.clamp(0.0, 1.0)
    }

    /// Fraction of query tokens found in the path.
    fn keyword_score(&self, path: &str) -> f64 {
        if self.query_tokens.is_empty() {
            return 0.0;
        }

        let path_tokens = Tokenizer::tokenize(path);
        let matches = self
            .query_tokens
            .iter()
            .filter(|qt| path_tokens.iter().any(|pt| pt == *qt))
            .count();

        matches as f64 / self.query_tokens.len() as f64
    }
}

/// Score based on file role. Implementation scores highest.
fn role_score(role: FileRole) -> f64 {
    match role {
        FileRole::Implementation => 1.0,
        FileRole::Build => 0.6,
        FileRole::Test => 0.5,
        FileRole::Config => 0.3,
        FileRole::Documentation => 0.2,
        FileRole::Other => 0.1,
        FileRole::Generated => 0.05,
    }
}

/// Score inversely proportional to directory depth. Shallower = better.
fn depth_score(path: &str) -> f64 {
    let depth = path.matches(['/', '\\']).count();
    match depth {
        0 => 1.0,
        1 => 0.9,
        2 => 0.7,
        3 => 0.5,
        4 => 0.3,
        _ => 0.1,
    }
}

/// Bonus for well-known source directories.
fn wellknown_score(path: &str) -> f64 {
    let first_component = path.split(['/', '\\']).next().unwrap_or("");
    match first_component {
        "src" | "lib" | "cmd" | "pkg" | "app" | "internal" | "crates" => 1.0,
        "bin" | "server" | "api" | "core" | "modules" => 0.8,
        "test" | "tests" | "spec" | "e2e" => 0.5,
        "docs" | "doc" | "examples" | "scripts" => 0.3,
        "vendor" | "node_modules" | "third_party" => 0.0,
        _ => 0.4,
    }
}

/// Penalty for very large files. Small/medium files score best.
fn size_score(size: u64) -> f64 {
    match size {
        0..=1_000 => 0.9,
        1_001..=5_000 => 1.0,
        5_001..=20_000 => 0.8,
        20_001..=100_000 => 0.5,
        100_001..=500_000 => 0.2,
        _ => 0.05,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_score_windows_paths() {
        // Backslash separators should count the same as forward slashes
        assert_eq!(depth_score("file.rs"), depth_score("file.rs"));
        assert_eq!(depth_score(r"src\file.rs"), depth_score("src/file.rs"));
        assert_eq!(
            depth_score(r"src\auth\middleware.rs"),
            depth_score("src/auth/middleware.rs")
        );
    }

    #[test]
    fn wellknown_score_windows_paths() {
        assert_eq!(
            wellknown_score(r"src\main.rs"),
            wellknown_score("src/main.rs")
        );
        assert_eq!(
            wellknown_score(r"lib\utils.rs"),
            wellknown_score("lib/utils.rs")
        );
        assert_eq!(
            wellknown_score(r"vendor\dep.rs"),
            wellknown_score("vendor/dep.rs")
        );
    }
}
