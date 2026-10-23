# Records assets/banner.gif, the animated banner at the top of the README.
#
#   cargo build --release --example banner
#   vhs assets/banner.tape

Output assets/banner.gif

Set Shell "bash"
Set FontSize 22
Set Width 1200
Set Height 400
Set Padding 24
Set Framerate 24
Set Theme "Catppuccin Mocha"

Hide
Type "clear && ./target/release/examples/banner" Enter
Sleep 1s
Show

# One breath (BREATH in examples/banner.rs), so the GIF loops cleanly. VHS
# drops about 0.2 seconds at the end, hence the extra.
Sleep 8.2s
