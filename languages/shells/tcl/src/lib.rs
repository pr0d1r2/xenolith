//! Tcl support for xenolith, the expect dialect included.
//!
//! Tcl is both roles. As a HOST, its `exec` and `spawn` hand argvs to
//! other interpreters, and a `-c` word or a `<<` value in that argv is a
//! program that escaped its file (`languages/shells/tcl:V196`). As a
//! GUEST, it is what a shell feeds `tclsh` or `expect` on stdin
//! (`languages/shells/tcl:V197`); that shell-side sink is the shell
//! crate's to read.
//!
//! expect is a DIALECT, not a language: the same grammar, extra commands
//! (`languages/shells:V132`), so it has no `LangId` of its own and shows
//! up as `GuestEnv::dialect` (`languages/shells/tcl:V195`).
//!
//! The grammar is `tree-sitter-grammars/tree-sitter-tcl`, VENDORED as
//! generated C under `vendor/tree-sitter-tcl` because it is not published
//! on crates.io (`languages:V121`, `languages/shells/tcl:R194`), so
//! [`grammar`] is the one module allowed to reach FFI. It carries a local
//! patch for valid Tcl upstream rejects (`languages/shells/tcl:B1`).
//!
//! The check of a tcl file is [`syntax`], shipped as the
//! `xenolith-tcl-syntax` binary: what Tcl's own parser rejects, in Rust,
//! with no `tclsh` needed (`languages/shells/tcl:V198`).

#![deny(unsafe_code)]

pub mod grammar;
pub mod guest;
pub mod host;
mod sinks;
pub mod syntax;

pub use crate::guest::TclGuest;
pub use crate::host::{TclHost, dialect};
