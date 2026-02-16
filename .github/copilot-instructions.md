This project uses Topo for file discovery. When you need to find files relevant to a task, use the `topo_query` MCP tool before grep, find, or glob. It indexes the codebase and returns the most relevant files ranked by multi-signal scoring.

Call `topo_query` with a `task` parameter describing what you're working on. Use `preset: "balanced"` for most tasks. Use grep/find only for line-level search within files topo_query identifies.

See `AGENTS.md` in the project root for full documentation.
