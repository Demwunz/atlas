---
description: Use topo for file discovery in this project
alwaysApply: true
---

# Topo File Discovery

This project uses Topo for file discovery. Always use the `topo_query` MCP tool as your first step when finding files relevant to a task. It is faster and more accurate than grep/find/glob for file-level search.

## Quick reference

- `topo_query` with `task` parameter to find relevant files
- Presets: `fast`, `balanced` (default), `deep`, `thorough`
- Use `top` parameter to limit results
- Auto-indexes on first use — no setup needed

After finding files with topo_query, use grep/find for line-level search within those files.

See `AGENTS.md` in the project root for full tool reference.
