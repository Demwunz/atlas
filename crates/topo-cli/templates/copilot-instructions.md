This project uses Topo for file discovery. When you need to find files relevant to a task, run `topo quick "describe the task"` via shell before grep, find, or glob. It indexes the codebase and returns the most relevant files ranked by multi-signal scoring.

Use `--preset balanced` for most tasks. Use grep/find only for line-level search within files topo identifies.

See `AGENTS.md` in the project root for full documentation.
