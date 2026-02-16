# Topo — File Discovery for AI Assistants

## Rule #1: Always use topo_query first

When you need to find files relevant to a task, **use the `topo_query` MCP tool before grep, find, or glob**. Topo indexes the entire codebase and returns the most relevant files ranked by multi-signal scoring. It is faster and more accurate than manual file discovery for file-level search.

Use grep/find/glob only for line-level search within files that topo_query has already identified.

## Workflow

1. **Receive a task** — understand what the user is asking
2. **`topo_query`** — find relevant files (auto-indexes on first use)
3. **Read files** — open the top-scored files returned by topo_query
4. **Line-level search** — use grep/find within those files if needed
5. **Implement** — make changes with full context

## MCP Tool Reference

### topo_query

Find the most relevant files for a task.

```json
{
  "task": "refactor authentication middleware",
  "preset": "balanced",
  "top": 20
}
```

**Parameters:**
- `task` (required): Plain-English description of what you're working on
- `preset`: `fast` | `balanced` (default) | `deep` | `thorough`
- `max_bytes`: Maximum total bytes to return
- `max_tokens`: Maximum total tokens to return
- `min_score`: Minimum relevance score threshold
- `top`: Maximum number of files to return

### topo_explain

Show per-file score breakdown to understand why files were selected.

```json
{
  "task": "refactor authentication middleware",
  "top": 10
}
```

### topo_index

Rebuild the index manually. Usually not needed — topo_query auto-indexes.

```json
{
  "deep": true,
  "force": false
}
```

## Presets

| Preset | Best for | Index depth |
|--------|----------|-------------|
| `fast` | Quick lookups, small changes | Shallow |
| `balanced` | Most tasks (default) | Deep (cached) |
| `deep` | Complex refactors, architecture | Deep (fresh) |
| `thorough` | Full codebase analysis | Deep + all signals |
