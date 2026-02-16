use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

/// Number of days to look back for git activity.
const LOOKBACK_DAYS: u32 = 90;

/// Compute git recency scores for files in a repository.
///
/// Runs `git log` to count commits per file in the last N days.
/// Returns normalized scores in [0.0, 1.0] where 1.0 = most recently active.
pub fn git_recency_scores(repo_root: &Path) -> anyhow::Result<HashMap<String, f64>> {
    let commit_counts = git_commit_counts(repo_root, LOOKBACK_DAYS)?;

    if commit_counts.is_empty() {
        return Ok(HashMap::new());
    }

    let max_count = commit_counts.values().copied().max().unwrap_or(1) as f64;

    let scores = commit_counts
        .into_iter()
        .map(|(path, count)| {
            // Log-scale normalization: log(1 + count) / log(1 + max_count)
            let score = (1.0 + count as f64).ln() / (1.0 + max_count).ln();
            (path, score)
        })
        .collect();

    Ok(scores)
}

/// Count commits per file in the last N days using git log.
fn git_commit_counts(repo_root: &Path, days: u32) -> anyhow::Result<HashMap<String, u32>> {
    let output = Command::new("git")
        .args([
            "log",
            "--format=",
            "--name-only",
            &format!("--since={days}.days"),
        ])
        .current_dir(repo_root)
        .output()?;

    if !output.status.success() {
        // Not a git repo or git not available — return empty
        return Ok(HashMap::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut counts: HashMap<String, u32> = HashMap::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            *counts.entry(trimmed.to_string()).or_default() += 1;
        }
    }

    Ok(counts)
}

/// Score a single file's recency given the full recency map.
/// Returns 0.0 if the file has no recent git activity.
pub fn file_recency(scores: &HashMap<String, f64>, path: &str) -> f64 {
    scores.get(path).copied().unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn init_git_repo(dir: &Path) {
        Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .unwrap();
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(dir)
            .output()
            .unwrap();
    }

    #[test]
    fn recency_non_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        let scores = git_recency_scores(dir.path()).unwrap();
        assert!(scores.is_empty());
    }

    #[test]
    fn recency_empty_git_repo() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());
        let scores = git_recency_scores(dir.path()).unwrap();
        assert!(scores.is_empty());
    }

    #[test]
    fn recency_with_commits() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        // Create and commit a file
        fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        Command::new("git")
            .args(["add", "main.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add main"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        let scores = git_recency_scores(dir.path()).unwrap();
        assert!(scores.contains_key("main.rs"));
        assert!(*scores.get("main.rs").unwrap() > 0.0);
    }

    #[test]
    fn recency_multiple_commits_higher_score() {
        let dir = tempfile::tempdir().unwrap();
        init_git_repo(dir.path());

        // File with 1 commit
        fs::write(dir.path().join("once.rs"), "fn once() {}").unwrap();
        Command::new("git")
            .args(["add", "once.rs"])
            .current_dir(dir.path())
            .output()
            .unwrap();
        Command::new("git")
            .args(["commit", "-m", "add once"])
            .current_dir(dir.path())
            .output()
            .unwrap();

        // File with 3 commits
        for i in 0..3 {
            fs::write(dir.path().join("active.rs"), format!("fn v{}() {{}}", i)).unwrap();
            Command::new("git")
                .args(["add", "active.rs"])
                .current_dir(dir.path())
                .output()
                .unwrap();
            Command::new("git")
                .args(["commit", "-m", &format!("update active v{}", i)])
                .current_dir(dir.path())
                .output()
                .unwrap();
        }

        let scores = git_recency_scores(dir.path()).unwrap();
        let active_score = scores.get("active.rs").copied().unwrap_or(0.0);
        let once_score = scores.get("once.rs").copied().unwrap_or(0.0);

        assert!(active_score > once_score);
    }

    #[test]
    fn file_recency_missing_file() {
        let scores = HashMap::new();
        assert_eq!(file_recency(&scores, "nonexistent.rs"), 0.0);
    }

    #[test]
    fn file_recency_known_file() {
        let mut scores = HashMap::new();
        scores.insert("main.rs".to_string(), 0.8);
        assert_eq!(file_recency(&scores, "main.rs"), 0.8);
    }
}
