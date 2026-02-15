use anyhow::Result;
use clap::Parser;

/// Atlas — fast codebase indexer and file selector for LLMs.
#[derive(Parser, Debug)]
#[command(name = "atlas", version, about)]
struct Cli {
    /// Increase log verbosity
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Suppress non-essential output
    #[arg(short, long)]
    quiet: bool,
}

fn main() -> Result<()> {
    let _cli = Cli::parse();
    println!("atlas v{}", env!("CARGO_PKG_VERSION"));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_parses_no_args() {
        // Verify the CLI parses with no arguments
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
}
