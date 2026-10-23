//! Pkl support for xenolith.
//!
//! Pkl is a host. The case this crate exists for is a hk config: a pkl
//! module whose steps carry shell in `check`, `fix` and friends
//! (`languages/ci/pkl` §I), where a multi-line string is a script that has
//! escaped its file and nobody's shellcheck will ever see it.
//!
//! Pkl is read with `pklr`, a pure-Rust Pkl parser built here without its
//! evaluator: it decides what is valid Pkl (as Pkl 0.31.1 and hk 2.x do),
//! and its tokens, which carry byte offsets, give the object tree the
//! host walks ([`syntax`]). No grammar is vendored and no C is compiled.

pub mod host;
mod placement;
mod shell;
pub mod string;
mod syntax;

pub use crate::host::{PklHost, SINKS};
pub use crate::string::{escape, multiline, unescape};
