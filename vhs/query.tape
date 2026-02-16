# Query workflow demo — index, query, explain
# Generates: vhs/query.gif

Output vhs/query.gif

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

# Step 1: Build a deep index
Type "topo index --deep"
Sleep 300ms
Enter
Sleep 3s

# Step 2: Query for relevant files
Type 'topo query "scoring engine" --format human --top 10'
Sleep 300ms
Enter
Sleep 4s

# Step 3: Same query with JSONL for piping
Type 'topo query "scoring engine" --format jsonl --top 5'
Sleep 300ms
Enter
Sleep 3s

Sleep 2s
