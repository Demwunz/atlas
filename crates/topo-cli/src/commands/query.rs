use crate::preset::Preset;
use crate::{Cli, OutputFormat};
use anyhow::Result;
use topo_core::{DeepIndex, ScoredFile, TokenBudget};
use topo_render::JsonlWriter;
use topo_scanner::BundleBuilder;
use topo_score::{HybridScorer, RrfFusion};

pub fn run(
    cli: &Cli,
    task: &str,
    preset: Preset,
    max_bytes: Option<u64>,
    max_tokens: Option<u64>,
    min_score: Option<f64>,
    top: Option<usize>,
) -> Result<()> {
    let root = cli.repo_root()?;

    // Scan files
    let bundle = BundleBuilder::new(&root).build()?;

    // Load deep index for PageRank when using structural signals
    let deep_index = if preset.use_structural_signals() {
        topo_index::load(&root)?
    } else {
        None
    };

    // Score files
    let scored = score_files(task, &bundle.files, preset, deep_index.as_ref());

    // Apply score filter
    let effective_min_score = min_score.unwrap_or(preset.default_min_score());
    let mut filtered: Vec<ScoredFile> = scored
        .into_iter()
        .filter(|f| f.score >= effective_min_score)
        .collect();

    // Apply top-N filter
    if let Some(n) = top {
        filtered.truncate(n);
    }

    // Enforce token budget
    let effective_max_bytes = max_bytes.unwrap_or(preset.default_max_bytes());
    let budget = TokenBudget {
        max_bytes: Some(effective_max_bytes),
        max_tokens,
    };
    let budgeted = budget.enforce(&filtered);

    // Output
    output_results(
        cli,
        task,
        preset,
        &budgeted,
        bundle.file_count(),
        effective_max_bytes,
        effective_min_score,
    )?;

    Ok(())
}

pub fn score_files(
    task: &str,
    files: &[topo_core::FileInfo],
    _preset: Preset,
    deep_index: Option<&DeepIndex>,
) -> Vec<ScoredFile> {
    let scorer = HybridScorer::new(task);
    let mut scored = scorer.score(files);

    // Apply PageRank via RRF fusion when available
    if let Some(index) = deep_index
        && !index.pagerank_scores.is_empty()
    {
        // Populate SignalBreakdown.pagerank for each scored file
        for file in &mut scored {
            file.signals.pagerank = index.pagerank_scores.get(&file.path).copied();
        }

        // Build PageRank-sorted ranking (owned strings to avoid borrow conflict)
        let mut pr_ranked: Vec<(String, f64)> = scored
            .iter()
            .filter_map(|f| f.signals.pagerank.map(|pr| (f.path.clone(), pr)))
            .collect();
        pr_ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        let pr_ranking: Vec<&str> = pr_ranked.iter().map(|(p, _)| p.as_str()).collect();

        // Fuse base ranking with PageRank ranking via RRF
        if !pr_ranking.is_empty() {
            let fusion = RrfFusion::new();
            fusion.fuse_scored(&mut scored, &[pr_ranking]);
        }
    }

    scored
}

pub fn output_results(
    cli: &Cli,
    task: &str,
    preset: Preset,
    files: &[ScoredFile],
    scanned_count: usize,
    max_bytes: u64,
    min_score: f64,
) -> Result<()> {
    match cli.effective_format() {
        OutputFormat::Jsonl | OutputFormat::Auto => {
            let output = JsonlWriter::new(task, preset.as_str())
                .max_bytes(Some(max_bytes))
                .min_score(min_score)
                .render(files, scanned_count)?;
            print!("{output}");
        }
        OutputFormat::Json => {
            let json_output = serde_json::json!({
                "version": "0.3",
                "query": task,
                "preset": preset.as_str(),
                "files": files.iter().map(|f| serde_json::json!({
                    "path": f.path,
                    "score": f.score,
                    "tokens": f.tokens,
                    "language": f.language.as_str(),
                    "role": f.role.as_str(),
                })).collect::<Vec<_>>(),
                "total_files": files.len(),
                "scanned_files": scanned_count,
            });
            println!("{}", serde_json::to_string_pretty(&json_output)?);
        }
        OutputFormat::Human => {
            if !files.is_empty() {
                println!(
                    "{:<60} {:>8} {:>8} {:>8}",
                    "PATH", "SCORE", "TOKENS", "LANG"
                );
                println!("{}", "-".repeat(88));
                for f in files {
                    println!(
                        "{:<60} {:>8.4} {:>8} {:>8}",
                        truncate_path(&f.path, 60),
                        f.score,
                        f.tokens,
                        f.language.as_str(),
                    );
                }
                println!("{}", "-".repeat(88));
            }
            println!(
                "{} files selected (of {} scanned) for query: \"{}\"",
                files.len(),
                scanned_count,
                task
            );
        }
    }

    Ok(())
}

fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else {
        format!("...{}", &path[path.len() - max_len + 3..])
    }
}
