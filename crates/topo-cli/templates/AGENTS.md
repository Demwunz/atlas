# Topo — File Discovery for AI Assistants

## Rule #1: Always run `topo quick` first

When you need to find files relevant to a task, **run `topo quick "task"` via shell before grep, find, or glob**. Topo indexes the entire codebase and returns the most relevant files ranked by multi-signal scoring. It is faster and more accurate than manual file discovery for file-level search.

Use grep/find/glob only for line-level search within files that topo has already identified.

## Workflow

1. **Receive a task** — understand what the user is asking
2. **`topo quick "describe the task"`** — find relevant files (auto-indexes on first use)
3. **Read files** — open the top-scored files returned by topo
4. **Line-level search** — use grep/find within those files if needed
5. **Implement** — make changes with full context

## CLI Reference

### topo quick

Find the most relevant files for a task.

```sh
topo quick "refactor authentication middleware"
topo quick "refactor authentication middleware" --preset deep --top 20
```

### topo explain

Show per-file score breakdown to understand why files were selected.

```sh
topo explain "refactor authentication middleware" --top 10
```

### topo index

Rebuild the index manually. Usually not needed — `topo quick` auto-indexes.

```sh
topo index --deep
```

## Presets

| Preset | Best for | Index depth |
|--------|----------|-------------|
| `fast` | Quick lookups, small changes | Shallow |
| `balanced` | Most tasks (default) | Deep (cached) |
| `deep` | Complex refactors, architecture | Deep (fresh) |
| `thorough` | Full codebase analysis | Deep + all signals |

## MCP Alternative

If your tool doesn't have shell access, Topo also runs as an MCP server with `topo_query`, `topo_explain`, and `topo_index` tools. See Topo documentation for MCP setup.
