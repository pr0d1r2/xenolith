# xenolith-lang-api

The contract every xenolith language crate implements: the `Host` and
`Guest` traits, `LangId`, the shared types, and the harness that checks
a language's lens law.

Part of [xenolith](https://github.com/pr0d1r2/xenolith), which finds
code of one language embedded in a file of another and moves it into
its own file. Most users want the `xenolith` crate or its `xnl` binary,
not this crate on its own.

License: MIT
