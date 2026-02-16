use crate::Cli;
use crate::preset::Preset;
use anyhow::Result;
use topo_scanner::BundleBuilder;

pub fn run(cli: &Cli, task: &str, top: usize, preset: Preset) -> Result<()> {
    let root = cli.repo_root()?;
    let bundle = BundleBuilder::new(&root).build()?;

    // Load deep index for PageRank when using structural signals
    let deep_index = if preset.use_structural_signals() {
        topo_index::load(&root)?
    } else {
        None
    };

    let scored = super::query::score_files(task, &bundle.files, preset, deep_index.as_ref());

    let display_count = top.min(scored.len());
    let results = &scored[..display_count];

    match cli.effective_format() {
        crate::OutputFormat::Json | crate::OutputFormat::Jsonl => {
            let output: Vec<serde_json::Value> = results
                .iter()
                .map(|f| {
                    serde_json::json!({
                        "path": f.path,
                        "score": f.score,
                        "signals": {
                            "bm25f": f.signals.bm25f,
                            "heuristic": f.signals.heuristic,
                            "pagerank": f.signals.pagerank,
                            "git_recency": f.signals.git_recency,
                        },
                        "tokens": f.tokens,
                        "language": f.language.as_str(),
                        "role": f.role.as_str(),
                    })
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&output)?);
        }
        _ => {
            println!("Score breakdown for query: \"{task}\"");
            println!("Showing top {display_count} of {} files\n", scored.len());

            println!(
                "{:<50} {:>8} {:>8} {:>8} {:>8} {:>8}",
                "PATH", "TOTAL", "BM25F", "HEUR", "PR", "ROLE"
            );
            println!("{}", "-".repeat(95));

            for f in results {
                let pr = f
                    .signals
                    .pagerank
                    .map(|v| format!("{v:.4}"))
                    .unwrap_or_else(|| "-".to_string());
                println!(
                    "{:<50} {:>8.4} {:>8.4} {:>8.4} {:>8} {:>8}",
                    truncate(&f.path, 50),
                    f.score,
                    f.signals.bm25f,
                    f.signals.heuristic,
                    pr,
                    f.role.as_str(),
                );
            }
        }
    }

    Ok(())
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("...{}", &s[s.len() - max + 3..])
    }
}
