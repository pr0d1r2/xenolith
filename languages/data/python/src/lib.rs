//! Python support for xenolith.
//!
//! Python is a GUEST only (`languages/data/python:V59`): what a shell
//! feeds `python3` on stdin or in `-c`, a just shebang recipe, a nix
//! `writers.writePython3` body. Those sinks are the host crates' to read;
//! this crate judges the body they hand over and says what an extract of
//! it is. A `.py` file is not claimed by any host, so python as a host is
//! a later question (`languages/data/python` §G).
//!
//! The grammar is `tree-sitter-python` from crates.io, so unlike tcl
//! there is no vendored C and no FFI here.

pub mod guest;

pub use crate::guest::PythonGuest;
