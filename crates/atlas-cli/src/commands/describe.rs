use crate::Cli;
use anyhow::Result;

pub fn run(cli: &Cli) -> Result<()> {
    let description = serde_json::json!({
        "name": "atlas",
        "version": env!("CARGO_PKG_VERSION"),
        "replaces": "repo-context",
        "commands": ["index", "query", "quick", "render", "explain", "inspect", "describe"],
        "formats": ["jsonl", "json", "human"],
        "languages": [
            "rust", "go", "python", "javascript", "typescript",
            "java", "ruby", "c", "cpp", "shell", "swift", "kotlin",
            "scala", "haskell", "elixir", "lua", "php", "r"
        ],
        "scoring": ["heuristic", "content", "hybrid"],
        "presets": ["fast", "balanced", "deep", "thorough"],
    });

    match cli.effective_format() {
        crate::OutputFormat::Human => {
            println!("atlas v{}", env!("CARGO_PKG_VERSION"));
            println!();
            println!("Commands:  index, query, quick, render, explain, inspect, describe");
            println!("Formats:   jsonl, json, human");
            println!(
                "Languages: rust, go, python, javascript, typescript, java, ruby, c, cpp, shell, swift, kotlin, scala, haskell, elixir, lua, php, r"
            );
            println!("Scoring:   heuristic, content, hybrid");
            println!("Presets:   fast, balanced, deep, thorough");
        }
        _ => {
            println!("{}", serde_json::to_string_pretty(&description)?);
        }
    }

    Ok(())
}
