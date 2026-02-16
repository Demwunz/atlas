use crate::Cli;
use anyhow::Result;
use std::fs;
use std::path::Path;

/// Read a JSONL file and re-render it.
pub fn run(cli: &Cli, file: &Path, _max_tokens: Option<u64>) -> Result<()> {
    let content = fs::read_to_string(file)?;

    // For now, pass through the JSONL content.
    // A future version could re-render with different format or budget.
    match cli.effective_format() {
        crate::OutputFormat::Human => {
            let lines: Vec<&str> = content.trim().lines().collect();
            if lines.is_empty() {
                println!("Empty JSONL file.");
                return Ok(());
            }

            // Parse and display
            for line in &lines {
                let v: serde_json::Value = serde_json::from_str(line)?;
                if v.get("Version").is_some() {
                    // Header
                    println!(
                        "Topo JSONL v{} — Query: \"{}\" — Preset: {}",
                        v["Version"], v["Query"], v["Preset"]
                    );
                    println!();
                } else if v.get("TotalFiles").is_some() {
                    // Footer
                    println!();
                    println!(
                        "Total: {} files, {} tokens (scanned {})",
                        v["TotalFiles"], v["TotalTokens"], v["ScannedFiles"]
                    );
                } else if v.get("Path").is_some() {
                    // File entry
                    println!(
                        "  {:<50} score={:.4} tokens={} lang={}",
                        v["Path"].as_str().unwrap_or("?"),
                        v["Score"].as_f64().unwrap_or(0.0),
                        v["Tokens"],
                        v["Language"].as_str().unwrap_or("?"),
                    );
                }
            }
        }
        _ => {
            // JSONL or JSON: pass through
            print!("{content}");
        }
    }

    Ok(())
}
