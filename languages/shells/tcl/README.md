# xenolith-lang-tcl

Tcl and expect support for xenolith: finds commands held in `exec`
and `spawn`. The tree-sitter-tcl grammar is vendored and compiled by
`build.rs`. It also ships `xenolith-tcl-syntax`, a Tcl syntax check that
needs no `tclsh`.

Part of [xenolith](https://github.com/pr0d1r2/xenolith), which finds
code of one language embedded in a file of another and moves it into
its own file. Most users want the `xenolith` crate or its `xnl` binary,
not this crate on its own.

License: MIT
