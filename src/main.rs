//! `xnl`, the xenolith command line.
//!
//! A shim (`src:C139`): `fn main` hands over to the library and nothing
//! else lives here. A `mod tests;` in this file would resolve to the same
//! `src/tests.rs` as the library's, so a `main.rs` with logic is logic
//! with no mirror -- the dispatch is `xenolith::cli` (`src/cli`).

fn main() -> std::process::ExitCode {
    xenolith::cli::main()
}
