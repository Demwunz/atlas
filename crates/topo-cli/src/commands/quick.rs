use crate::Cli;
use crate::preset::Preset;
use anyhow::Result;

/// One-shot command: index + query in a single invocation.
pub fn run(
    cli: &Cli,
    task: &str,
    preset: Preset,
    max_bytes: Option<u64>,
    max_tokens: Option<u64>,
    min_score: Option<f64>,
    top: Option<usize>,
) -> Result<()> {
    // Step 1: Index (if needed)
    if preset.needs_deep_index() {
        if !cli.is_quiet() {
            eprintln!("Building index (preset: {preset})...");
        }
        super::index::run(cli, true, preset.force_rebuild())?;
    } else if !cli.is_quiet() {
        eprintln!("Scanning (preset: {preset}, shallow mode)...");
        // Shallow scan happens inside query
    }

    // Step 2: Query
    super::query::run(cli, task, preset, max_bytes, max_tokens, min_score, top)?;

    Ok(())
}
