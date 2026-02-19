# Quick command demo — one-command context generation
# Generates: vhs/quick.gif

Output vhs/quick.gif

Require topo

Set Shell "bash"
Set FontSize 16
Set Width 1000
Set Height 600
Set LineHeight 1.2
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set WindowBarSize 40
Set Padding 20
Set TypingSpeed 40ms

# Basic quick command — human-readable output
Type 'topo quick "add health check endpoint" --format human'
Sleep 300ms
Enter
Sleep 4s

# Quick with fast preset and top limit
Type 'topo quick "fix authentication bug" --preset fast --top 5 --format human'
Sleep 300ms
Enter
Sleep 3s

# Compact output — minimal tokens, used by Claude Code hooks
Type 'topo quick "add retry logic to API calls" --format compact --top 5'
Sleep 300ms
Enter
Sleep 3s

# Quick with JSONL output for piping
Type 'topo quick "refactor error handling" --format jsonl --top 5'
Sleep 300ms
Enter
Sleep 3s

Sleep 2s
