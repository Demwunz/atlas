# Hero demo — matches README intro: vague input, precise output, fast
# Generates: vhs/hero.gif

Output vhs/hero.gif

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

# "Vague input, precise output" — the exact example from the README intro
Type 'topo quick "auth middleware" --format human --top 10'
Sleep 300ms
Enter
Sleep 4s

# Precise token budgets — fill your context window with signal
Type 'topo quick "auth middleware" --max-tokens 8000 --format compact'
Sleep 300ms
Enter
Sleep 3s

# Score breakdown — shows how files keywords miss get surfaced
Type 'topo explain "auth middleware" --top 5'
Sleep 300ms
Enter
Sleep 4s

# Re-index is instant — unchanged files are skipped
Type 'topo quick "add health check endpoint" --format human --top 5'
Sleep 300ms
Enter
Sleep 3s

# Works with every AI tool — one command sets them all up
Type "topo init --force"
Sleep 300ms
Enter
Sleep 3s

Sleep 2s
