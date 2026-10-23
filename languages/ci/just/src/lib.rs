//! just support for xenolith.
//!
//! just is a host. A recipe body is shell escaped from its file: each
//! line a command just hands to a fresh shell, or -- under a `#!` first
//! line -- one script for the interpreter it names
//! (`languages/ci/just` §I). Past one simple command it is a script
//! nobody's shellcheck will ever see.
//!
//! The parser is this crate's own, in pure Rust: a lexer and a
//! recursive descent over just's own grammar ([`parse`]), in place of a
//! vendored tree-sitter grammar that rejected just 1.51 syntax and
//! misread `{{{{` (`languages/ci/just:B2`, `languages/ci/just:B3`).

pub mod host;
mod lexer;
mod lines;
mod parse;
mod placement;
mod recipe;
mod shell;
mod state;

pub use crate::host::JustHost;
pub use crate::lines::{escape, unescape};
