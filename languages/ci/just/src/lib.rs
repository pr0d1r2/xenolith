//! just support for xenolith.
//!
//! just is a host. A recipe body is shell escaped from its file: each
//! line a command just hands to a fresh shell, or -- under a `#!` first
//! line -- one script for the interpreter it names
//! (`languages/ci/just` §I). Past one simple command it is a script
//! nobody's shellcheck will ever see.
//!
//! The grammar is `tree-sitter-just`, VENDORED as generated C under
//! `vendor/tree-sitter-just`: the published crate pins a tree-sitter
//! runtime the workspace cannot link beside its own (`languages:V121`).
//! That makes [`grammar`] the one module allowed FFI.

#![deny(unsafe_code)]

pub mod grammar;
pub mod host;
mod lines;
mod placement;
mod recipe;
mod shell;
mod state;

pub use crate::host::JustHost;
pub use crate::lines::{escape, unescape};
