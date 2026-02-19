# Init + hooks demo — AI assistant setup
# Generates: vhs/init.gif

Output vhs/init.gif

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

# Set up AI assistants with hooks
Type "topo init"
Sleep 300ms
Enter
Sleep 3s

# Show what hooks were installed
Type "ls -la .claude/hooks/"
Sleep 300ms
Enter
Sleep 2s

# Show the settings that were patched
Type "cat .claude/settings.json | jq .hooks"
Sleep 300ms
Enter
Sleep 3s

# Simulate what a hook returns
Type 'echo '"'"'{"prompt":"refactor auth middleware"}'"'"' | .claude/hooks/topo-context.sh | jq .'
Sleep 300ms
Enter
Sleep 4s

Sleep 2s
