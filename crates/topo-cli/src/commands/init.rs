use crate::Cli;
use anyhow::Result;
use std::fs;
use std::path::Path;

const AGENTS_MD: &str = include_str!("../../templates/AGENTS.md");
const CURSOR_TOPO_MD: &str = include_str!("../../templates/cursor-topo.md");
const COPILOT_INSTRUCTIONS_MD: &str = include_str!("../../templates/copilot-instructions.md");

enum WriteResult {
    Created,
    Skipped,
}

fn write_template(path: &Path, content: &str, force: bool) -> Result<WriteResult> {
    if path.exists() && !force {
        return Ok(WriteResult::Skipped);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(WriteResult::Created)
}

fn check_topo_on_path() {
    let cmd = if cfg!(windows) {
        std::process::Command::new("where.exe")
            .arg("topo")
            .output()
    } else {
        std::process::Command::new("which")
            .arg("topo")
            .output()
    };

    match cmd {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout)
                .lines()
                .next()
                .unwrap_or_default()
                .to_string();
            println!("topo found on PATH: {path}");
            println!("Your AI assistant can now run `topo quick \"task\"` via shell.");
        }
        _ => {
            println!("Warning: topo is not on PATH.");
            println!("Install it so your AI assistant can run `topo quick \"task\"`:");
            println!();
            if cfg!(target_os = "macos") {
                println!("  brew install demwunz/tap/topo    # Homebrew");
            }
            println!("  cargo install topo-cli            # Cargo");
            println!("  curl -fsSL https://topo.sh | sh   # Shell script");
        }
    }

    println!();
    println!("Optional: for tools without shell access, topo also runs as an MCP server.");
    println!("See https://github.com/demwunz/topo#mcp for setup instructions.");
}

pub fn run(cli: &Cli, force: bool) -> Result<()> {
    let root = cli.repo_root()?;
    let quiet = cli.is_quiet();

    // AGENTS.md at repo root
    let agents_path = root.join("AGENTS.md");
    match write_template(&agents_path, AGENTS_MD, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created AGENTS.md");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!("  Skipped AGENTS.md (already exists, use --force to overwrite)");
            }
        }
    }

    // .cursor/rules/topo.md
    let cursor_path = root.join(".cursor/rules/topo.md");
    match write_template(&cursor_path, CURSOR_TOPO_MD, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created .cursor/rules/topo.md");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!(
                    "  Skipped .cursor/rules/topo.md (already exists, use --force to overwrite)"
                );
            }
        }
    }

    // .github/copilot-instructions.md (only if .github/ exists)
    let github_dir = root.join(".github");
    if github_dir.is_dir() {
        let copilot_path = github_dir.join("copilot-instructions.md");
        match write_template(&copilot_path, COPILOT_INSTRUCTIONS_MD, force)? {
            WriteResult::Created => {
                if !quiet {
                    println!("  Created .github/copilot-instructions.md");
                }
            }
            WriteResult::Skipped => {
                if !quiet {
                    println!(
                        "  Skipped .github/copilot-instructions.md (already exists, use --force to overwrite)"
                    );
                }
            }
        }
    } else if !quiet {
        println!("  Skipped .github/copilot-instructions.md (no .github/ directory)");
    }

    if !quiet {
        println!();
        check_topo_on_path();
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn templates_are_non_empty() {
        assert!(!AGENTS_MD.is_empty());
        assert!(!CURSOR_TOPO_MD.is_empty());
        assert!(!COPILOT_INSTRUCTIONS_MD.is_empty());
    }

    #[test]
    fn write_template_creates_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        let result = write_template(&path, "hello", false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
    }

    #[test]
    fn write_template_skips_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original").unwrap();
        let result = write_template(&path, "new content", false).unwrap();
        assert!(matches!(result, WriteResult::Skipped));
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
    }

    #[test]
    fn write_template_force_overwrites() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.md");
        fs::write(&path, "original").unwrap();
        let result = write_template(&path, "new content", true).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "new content");
    }

    #[test]
    fn write_template_creates_parent_dirs() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("a/b/c/test.md");
        let result = write_template(&path, "nested", false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        assert_eq!(fs::read_to_string(&path).unwrap(), "nested");
    }
}
