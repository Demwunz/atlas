# Hero demo — end-to-end topo showcase
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

# Show version
Type "topo --version"
Sleep 300ms
Enter
Sleep 1s

# Set up AI assistants + Claude Code hooks
Type "topo init --force"
Sleep 300ms
Enter
Sleep 3s

# Run quick command — the star of the show
Type 'topo quick "refactor the scoring engine" --format human --top 10'
Sleep 300ms
Enter
Sleep 4s

# Show compact format (what hooks inject)
Type 'topo quick "scoring engine" --format compact --top 5'
Sleep 300ms
Enter
Sleep 3s

# Show context savings
Type "topo gain"
Sleep 300ms
Enter
Sleep 3s

Sleep 2s
