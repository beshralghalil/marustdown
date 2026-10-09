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
Set Theme "GitHub Dark"
Set TypingSpeed 90ms

# Hidden setup: the fresh build, a copy of the demo, and a config with half-blocks,
# since VHS screenshots can't capture sixel or iTerm2 images.
Hide
Type `export PATH=$PWD/target/release:$PATH XDG_CONFIG_HOME=$(mktemp -d) && cd $(mktemp -d)` Enter
Type `mkdir docs $XDG_CONFIG_HOME/marustdown && cp -r $OLDPWD/docs/demo.md $OLDPWD/docs/images docs` Enter
Type `printf '[layout]\nimages = "blocks"\n' > $XDG_CONFIG_HOME/marustdown/config.toml && clear` Enter
Show

Type "mar docs/demo.md" Enter
Sleep 1s

# Highlighted code
Sleep 2s
Screenshot docs/screenshots/code.png
Sleep 1s

# Search for a task and check it off
Type "/Check off" Sleep 400ms Enter
Sleep 1s
Type "x"
Sleep 1.5s
Screenshot docs/screenshots/tasks.png
Sleep 1s

# Outline: filter the headings, then jump
Type "o"
Sleep 1.2s
Screenshot docs/screenshots/outline.png
Sleep 1s
Type "key"
Sleep 1s
Enter
Sleep 2s

# Math and diagrams
Type "o"
Sleep 500ms
Type "math"
Sleep 800ms
Enter
Sleep 1.5s
Screenshot docs/screenshots/math-diagrams.png
Sleep 1s

# Images
Type "o"
Sleep 500ms
Type "images"
Sleep 800ms
Enter
Sleep 2.5s

Type "q"
Sleep 300ms
