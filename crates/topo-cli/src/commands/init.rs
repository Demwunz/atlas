use crate::Cli;
use anyhow::Result;
use std::fs;
use std::path::Path;

const AGENTS_MD: &str = include_str!("../../templates/AGENTS.md");
const CURSOR_TOPO_MD: &str = include_str!("../../templates/cursor-topo.md");
const COPILOT_INSTRUCTIONS_MD: &str = include_str!("../../templates/copilot-instructions.md");
const CLAUDE_MD_SECTION: &str = include_str!("../../templates/claude-md-section.md");

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

const TOPO_START: &str = "<!-- topo:start -->";
const TOPO_END: &str = "<!-- topo:end -->";

fn inject_claude_md(path: &Path, section: &str, force: bool) -> Result<WriteResult> {
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else {
        String::new()
    };

    if let Some(start) = content.find(TOPO_START) {
        if !force {
            return Ok(WriteResult::Skipped);
        }
        // Replace existing section (inclusive of markers)
        let end = content[start..]
            .find(TOPO_END)
            .map(|i| start + i + TOPO_END.len())
            .unwrap_or(content.len());
        let mut new_content = String::with_capacity(content.len());
        new_content.push_str(&content[..start]);
        new_content.push_str(section.trim_end());
        // Preserve anything after the old end marker
        let after = &content[end..];
        if !after.is_empty() {
            new_content.push_str(after);
        } else {
            new_content.push('\n');
        }
        fs::write(path, new_content)?;
    } else if content.is_empty() {
        // New file — just write the section
        fs::write(path, section)?;
    } else {
        // Existing file without markers — append
        let mut new_content = content;
        if !new_content.ends_with('\n') {
            new_content.push('\n');
        }
        new_content.push('\n');
        new_content.push_str(section);
        fs::write(path, new_content)?;
    }

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

    // CLAUDE.md — inject topo section (never overwrite user content)
    let claude_path = root.join("CLAUDE.md");
    match inject_claude_md(&claude_path, CLAUDE_MD_SECTION, force)? {
        WriteResult::Created => {
            if !quiet {
                println!("  Created CLAUDE.md (topo section)");
            }
        }
        WriteResult::Skipped => {
            if !quiet {
                println!("  Skipped CLAUDE.md (topo section already present, use --force to update)");
            }
        }
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

    #[test]
    fn inject_claude_md_creates_new_file() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.contains(TOPO_START));
        assert!(content.contains(TOPO_END));
        assert!(content.contains("topo quick"));
    }

    #[test]
    fn inject_claude_md_appends_to_existing() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, "# My Project\n\nExisting content.\n").unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("# My Project"));
        assert!(content.contains(TOPO_START));
        assert!(content.contains(TOPO_END));
    }

    #[test]
    fn inject_claude_md_skips_when_present() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        fs::write(&path, format!("# Project\n\n{CLAUDE_MD_SECTION}")).unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, false).unwrap();
        assert!(matches!(result, WriteResult::Skipped));
    }

    #[test]
    fn inject_claude_md_force_replaces() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("CLAUDE.md");
        let old_section = "<!-- topo:start -->\nold content\n<!-- topo:end -->\n";
        fs::write(&path, format!("# Project\n\n{old_section}")).unwrap();
        let result = inject_claude_md(&path, CLAUDE_MD_SECTION, true).unwrap();
        assert!(matches!(result, WriteResult::Created));
        let content = fs::read_to_string(&path).unwrap();
        assert!(!content.contains("old content"));
        assert!(content.contains("topo quick"));
        assert!(content.starts_with("# Project"));
    }
}
