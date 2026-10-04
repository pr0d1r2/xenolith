//! Shell support for xenolith.
//!
//! Shell is the language this tool was built for. It is the guest that
//! ends up inside nix strings, hk steps, `run:` blocks and just recipes,
//! and it is also a host in its own right -- a heredoc fed to `python`
//! encloses python the same way a nix string encloses bash.
//!
//! At its centre is the classifier: the answer to "is this one command,
//! or is it a script?" (`languages/shells/shell:V3`). Every shell sink in every
//! host asks it, so it is shared rather than reimplemented per host --
//! twelve implementations of one rule would be twelve places for the
//! answer to differ. Around it: shell as a guest ([`ShellGuest`]) and as
//! a host ([`ShellHost`]), whose sinks are read in `sinks`.

#![forbid(unsafe_code)]

pub mod classify;
pub mod guest;
pub mod host;
mod params;
mod sinks;

pub use crate::classify::{Classification, Construct, ZSH_UNSUPPORTED, classify, classify_in};
pub use crate::guest::ShellGuest;
pub use crate::host::ShellHost;
