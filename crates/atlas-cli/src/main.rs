mod commands;
mod preset;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use std::io::IsTerminal;
use std::path::PathBuf;

/// Atlas — fast codebase indexer and file selector for LLMs.
#[derive(Parser, Debug)]
#[command(name = "atlas", version, about)]
pub struct Cli {
    /// Increase log verbosity
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long, global = true)]
    quiet: bool,

    /// Output format (default: auto-detect)
    #[arg(long, value_enum, default_value = "auto", global = true)]
    format: OutputFormat,

    /// Disable color output
    #[arg(long, global = true)]
    no_color: bool,

    /// Repository root (default: current directory)
    #[arg(long, global = true)]
    root: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Auto,
    Json,
    Jsonl,
    Human,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Build or update the file index
    Index {
        /// Enable deep indexing with AST chunking
        #[arg(long)]
        deep: bool,

        /// Rebuild index from scratch (ignore cache)
        #[arg(long)]
        force: bool,
    },

    /// Score and select files for a query
    Query {
        /// The task or query to search for
        task: String,

        /// Preset: fast, balanced, deep, thorough
        #[arg(long, value_enum, default_value = "balanced")]
        preset: preset::Preset,

        /// Maximum bytes for token budget
        #[arg(long)]
        max_bytes: Option<u64>,

        /// Maximum tokens for token budget
        #[arg(long)]
        max_tokens: Option<u64>,

        /// Minimum score threshold
        #[arg(long)]
        min_score: Option<f64>,

        /// Return top N files
        #[arg(long)]
        top: Option<usize>,
    },

    /// One-shot: index + query in a single command
    Quick {
        /// The task or query to search for
        task: String,

        /// Preset: fast, balanced, deep, thorough
        #[arg(long, value_enum, default_value = "balanced")]
        preset: preset::Preset,

        /// Maximum bytes for token budget
        #[arg(long)]
        max_bytes: Option<u64>,

        /// Maximum tokens for token budget
        #[arg(long)]
        max_tokens: Option<u64>,

        /// Minimum score threshold
        #[arg(long)]
        min_score: Option<f64>,

        /// Return top N files
        #[arg(long)]
        top: Option<usize>,
    },

    /// Convert JSONL selection to formatted output
    Render {
        /// Path to JSONL file
        file: PathBuf,

        /// Maximum tokens for budget
        #[arg(long)]
        max_tokens: Option<u64>,
    },

    /// Show per-file score breakdown
    Explain {
        /// The task or query to explain scoring for
        task: String,

        /// Return top N files
        #[arg(long, default_value = "10")]
        top: usize,
    },

    /// Inspect the index (file count, size, stats)
    Inspect,

    /// Print machine-readable tool capabilities
    Describe,
}

impl Cli {
    /// Resolve the repository root path.
    pub fn repo_root(&self) -> Result<PathBuf> {
        if let Some(ref root) = self.root {
            Ok(root.clone())
        } else if let Ok(root) = std::env::var("ATLAS_ROOT") {
            Ok(PathBuf::from(root))
        } else {
            Ok(std::env::current_dir()?)
        }
    }

    /// Determine the effective output format.
    pub fn effective_format(&self) -> OutputFormat {
        match self.format {
            OutputFormat::Auto => {
                if std::io::stdout().is_terminal() {
                    OutputFormat::Human
                } else {
                    OutputFormat::Jsonl
                }
            }
            ref f => f.clone(),
        }
    }

    pub fn is_quiet(&self) -> bool {
        self.quiet
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Index { deep, force }) => {
            commands::index::run(&cli, deep, force)?;
        }
        Some(Command::Query {
            ref task,
            preset,
            max_bytes,
            max_tokens,
            min_score,
            top,
        }) => {
            commands::query::run(&cli, task, preset, max_bytes, max_tokens, min_score, top)?;
        }
        Some(Command::Quick {
            ref task,
            preset,
            max_bytes,
            max_tokens,
            min_score,
            top,
        }) => {
            commands::quick::run(&cli, task, preset, max_bytes, max_tokens, min_score, top)?;
        }
        Some(Command::Render {
            ref file,
            max_tokens,
        }) => {
            commands::render::run(&cli, file, max_tokens)?;
        }
        Some(Command::Explain { ref task, top }) => {
            commands::explain::run(&cli, task, top)?;
        }
        Some(Command::Inspect) => {
            commands::inspect::run(&cli)?;
        }
        Some(Command::Describe) => {
            commands::describe::run(&cli)?;
        }
        None => {
            // No subcommand: print version info
            if !cli.is_quiet() {
                println!("atlas v{}", env!("CARGO_PKG_VERSION"));
                println!("Run 'atlas --help' for usage information.");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_no_args() {
        let cli = Cli::try_parse_from(["atlas"]);
        assert!(cli.is_ok());
    }

    #[test]
    fn cli_parses_verbose() {
        let cli = Cli::try_parse_from(["atlas", "-v"]).unwrap();
        assert_eq!(cli.verbose, 1);
    }

    #[test]
    fn cli_parses_quiet() {
        let cli = Cli::try_parse_from(["atlas", "--quiet"]).unwrap();
        assert!(cli.quiet);
    }

    #[test]
    fn cli_parses_index() {
        let cli = Cli::try_parse_from(["atlas", "index"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Index {
                deep: false,
                force: false
            })
        ));
    }

    #[test]
    fn cli_parses_index_deep() {
        let cli = Cli::try_parse_from(["atlas", "index", "--deep"]).unwrap();
        assert!(matches!(
            cli.command,
            Some(Command::Index {
                deep: true,
                force: false
            })
        ));
    }

    #[test]
    fn cli_parses_query() {
        let cli = Cli::try_parse_from(["atlas", "query", "auth middleware"]).unwrap();
        match cli.command {
            Some(Command::Query { ref task, .. }) => {
                assert_eq!(task, "auth middleware");
            }
            _ => panic!("expected Query"),
        }
    }

    #[test]
    fn cli_parses_quick_with_preset() {
        let cli = Cli::try_parse_from(["atlas", "quick", "auth", "--preset", "fast"]).unwrap();
        match cli.command {
            Some(Command::Quick {
                ref task, preset, ..
            }) => {
                assert_eq!(task, "auth");
                assert!(matches!(preset, preset::Preset::Fast));
            }
            _ => panic!("expected Quick"),
        }
    }

    #[test]
    fn cli_parses_explain() {
        let cli = Cli::try_parse_from(["atlas", "explain", "auth", "--top", "5"]).unwrap();
        match cli.command {
            Some(Command::Explain { ref task, top }) => {
                assert_eq!(task, "auth");
                assert_eq!(top, 5);
            }
            _ => panic!("expected Explain"),
        }
    }

    #[test]
    fn cli_parses_describe() {
        let cli = Cli::try_parse_from(["atlas", "describe"]).unwrap();
        assert!(matches!(cli.command, Some(Command::Describe)));
    }

    #[test]
    fn cli_parses_format_json() {
        let cli = Cli::try_parse_from(["atlas", "--format", "json"]).unwrap();
        assert!(matches!(cli.format, OutputFormat::Json));
    }

    #[test]
    fn cli_parses_root() {
        let cli = Cli::try_parse_from(["atlas", "--root", "/tmp/myrepo"]).unwrap();
        assert_eq!(cli.root, Some(PathBuf::from("/tmp/myrepo")));
    }

    #[test]
    fn cli_parses_query_with_budget() {
        let cli = Cli::try_parse_from([
            "atlas",
            "query",
            "auth",
            "--max-bytes",
            "100000",
            "--min-score",
            "0.1",
            "--top",
            "20",
        ])
        .unwrap();
        match cli.command {
            Some(Command::Query {
                max_bytes,
                min_score,
                top,
                ..
            }) => {
                assert_eq!(max_bytes, Some(100_000));
                assert_eq!(min_score, Some(0.1));
                assert_eq!(top, Some(20));
            }
            _ => panic!("expected Query"),
        }
    }
}
