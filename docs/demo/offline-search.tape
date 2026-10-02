Output docs/demo/offline-search.gif
Set Shell "bash"
Set FontSize 14
Set Width 1400
Set Height 190
Set Padding 24
Set Theme "Catppuccin Mocha"
Set TypingSpeed 40ms
Hide
Type `export PS1='$ ' PATH="$PWD/target/release:$PATH"` Enter
Type "clear" Enter
Show
Type `repository-intelligence --hybrid evaluation/corpus "index header" --top 3`
Sleep 300ms
Enter
Sleep 4s
