//! Pkl support for xenolith.
//!
//! Pkl is a host. The case this crate exists for is a hk config: a pkl
//! module whose steps carry shell in `check`, `fix` and friends
//! (`languages/pkl` §I), where a multi-line string is a script that has
//! escaped its file and nobody's shellcheck will ever see it.
//!
//! The grammar is `apple/tree-sitter-pkl`, VENDORED as generated C under
//! `vendor/tree-sitter-pkl` because it is not published on crates.io
//! (`languages:V121`). That makes this the one language crate that
//! reaches FFI itself, and [`grammar`] is the only module allowed to.

#![deny(unsafe_code)]

pub mod grammar;
pub mod host;
mod placement;
pub mod string;

pub use crate::host::{PklHost, SINKS};
pub use crate::string::{escape, multiline, unescape};
