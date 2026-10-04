//! xenolith: one language per file.
//!
//! A xenolith is a foreign rock fragment enclosed in host rock. This crate
//! finds the software equivalent -- shell inside a nix string, a jq
//! program inside a pkl step, a SQL statement inside Ruby -- extracts it
//! to a file of its own, rewrites the host to load that file, and then
//! proves the load resolves and the extract passes its own language's
//! linter.
//!
//! The crate is a LIBRARY first (`src:C1`): the `xnl` binary is one
//! consumer of it, and everything the binary can do a caller can do
//! without spawning a process.
//!
//! # What exists today
//!
//! The version constant below and nothing else. The spec federates before
//! the code (`.:C22`), so the shape of the model (`src:T8`), the verbs
//! (`src/cli:T9`) and the engines each arrive with their own tasks and
//! their own tests. This file is the crate root those land in, not a
//! placeholder for them.

pub mod check;
pub mod cli;
pub mod config;
pub mod discover;
pub mod extract;
pub mod graph;
pub mod lint;
pub mod model;
pub mod registry;

pub use check::check;

/// The package version, from Cargo at compile time.
///
/// One source, so the library and the `xnl` binary cannot disagree about
/// what they are -- which they would the moment a second literal existed
/// to be updated by hand.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
