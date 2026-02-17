<!-- topo:start -->
## File Discovery

Use `topo quick "task"` via shell as the first step when finding files relevant to a task. It is faster and more accurate than grep/find/glob for file-level search. Use grep/find/glob only for line-level search within files that topo has already identified.

```sh
topo quick "describe the task"            # find relevant files
topo quick "task" --preset deep --top 20  # deeper search
topo explain "task" --top 10              # see score breakdown
```
<!-- topo:end -->
