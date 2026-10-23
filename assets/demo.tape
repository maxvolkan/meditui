# Records assets/demo.gif for the README.
#
#   cargo build --release
#   vhs assets/demo.tape

Output assets/demo.gif

Set Shell "bash"
Set FontSize 18
Set Width 1200
Set Height 760
Set Padding 20
Set Framerate 24
Set Theme "Catppuccin Mocha"
Set WindowBar Colorful
Set BorderRadius 10
Set Margin 16
Set MarginFill "#11111b"
Set TypingSpeed 90ms

# Throwaway data and config folders with the tour already seen and sounds
# off, so recording never touches your real stats or makes any noise.
Hide
Type `export XDG_DATA_HOME="$(mktemp -d)" XDG_CONFIG_HOME="$(mktemp -d)"` Enter
Type `mkdir -p "$XDG_DATA_HOME/meditui"` Enter
Type `echo '{"settings":{"show_tutorial":false,"sound":false}}' > "$XDG_DATA_HOME/meditui/data.json"` Enter
Type `export PATH="$PWD/target/release:$PATH" && clear` Enter
Show

Sleep 600ms
Type "medi"
Sleep 500ms
Enter
Sleep 2s

# Start here says when to use each one. Then Nadi Shuddhi.
Up
Sleep 3s
Down
Sleep 1.2s

# Keep the length, skip the checklist.
Enter
Sleep 1.2s
Enter
Sleep 1.2s
Type "s"

# The first-time walkthrough, one short step at a time.
Sleep 1.8s
Enter
Sleep 1.8s
Enter
Sleep 1.8s
Enter
Sleep 1.8s
Enter

# The warm-up: long, slow breaths at first. In through the left, out the right.
Sleep 12s

# Skip ahead. Eyes-closed practices start dim, with just what to do.
Enter
Sleep 2.5s
Enter
Sleep 5s
