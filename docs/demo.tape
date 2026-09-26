# Regenerates the README screenshots and demo GIF. From the repo root:
#   cargo build --release && vhs docs/demo.tape
# The tape works on a copy of docs/demo.md, so toggling tasks leaves the repo clean.

Output docs/screenshots/demo.gif

Set Shell "bash"
Set FontFamily "JetBrainsMono Nerd Font Mono"
Set FontSize 18
Set Width 1300
Set Height 900
Set Padding 20
Set Margin 30
Set MarginFill "#313244"
Set BorderRadius 12
Set WindowBar Colorful
Set Theme "Catppuccin Mocha"
Set TypingSpeed 90ms

Hide
Type "cp docs/demo.md /tmp/marustdown-demo.md && clear" Enter
Type "./target/release/mar /tmp/marustdown-demo.md" Enter
Sleep 1s
Show

# Highlighted code
Sleep 2s
Screenshot docs/screenshots/code.png

# Search for a task and check it off
Type "/Check off" Sleep 400ms Enter
Sleep 1s
Type "x"
Sleep 1.5s
Screenshot docs/screenshots/tasks.png

# Outline: filter the headings, then jump
Type "o"
Sleep 1.2s
Screenshot docs/screenshots/outline.png
Type "key"
Sleep 1s
Enter
Sleep 2s

Type "q"
Sleep 300ms
