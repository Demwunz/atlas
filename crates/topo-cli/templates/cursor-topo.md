---
description: Use topo for file discovery in this project
alwaysApply: true
---

# Topo File Discovery

This project uses Topo for file discovery. Always run `topo quick "task"` via shell as your first step when finding files relevant to a task. It is faster and more accurate than grep/find/glob for file-level search.

## Quick reference

- `topo quick "describe the task"` to find relevant files
- Presets: `fast`, `balanced` (default), `deep`, `thorough`
- Use `--top N` to limit results
- Auto-indexes on first use — no setup needed

After finding files with topo, use grep/find for line-level search within those files.

See `AGENTS.md` in the project root for full CLI reference.
