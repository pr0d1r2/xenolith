//! `xnl inline`: read `xenolith.toml`, run the inline engine, print the
//! diff or write it (`src/cli` §I, `src/extract:V101`).
//!
//! Input and output only, as `xnl extract` is: the engine
//! ([`crate::extract::inline`]) decides what goes back where and what it
//! refuses, and the edit it returns is rendered and written exactly as
//! an extraction's is ([`super::extract::render`],
//! [`super::extract::write`]) -- the diff alone on stdout; refusals,
//! warnings and `--verbose` on stderr.
//!
//! Exit (`src/cli:V24`): 2 when anything was refused, else 1 when the
//! diff is not empty, else 0; under `--write`, 0 or 2.

use std::io::Write;
use std::path::{Path, PathBuf};

use super::extract::{render, write};
use super::refuse;
use crate::extract::{self, InlineOptions};

#[cfg(test)]
mod tests;

/// The flags `xnl inline` runs under.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--write`: apply rather than print (`src/extract:C15`).
    pub write: bool,
    /// `--verbose`: say what was written and why a load was left.
    pub verbose: bool,
    /// `--strict-hosts` (`src/check:V13`).
    pub strict_hosts: bool,
}

/// Run `xnl inline` from `root` over `extracts`, writing to `out` and
/// `err`.
pub fn run(
    root: &Path,
    extracts: &[PathBuf],
    flags: Flags,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let config = match super::check::load(root) {
        Ok(config) => config,
        Err(message) => return refuse(err, &message),
    };
    let options = InlineOptions {
        extracts: extracts.to_vec(),
        strict_hosts: flags.strict_hosts,
    };
    let edit = match extract::inline(root, &config, &options) {
        Ok(edit) => edit,
        Err(e) => return refuse(err, &format!("xnl: inline: {e}")),
    };
    if flags.write {
        write(root, &edit, flags.verbose, err)
    } else {
        render(&edit, flags.verbose, out, err)
    }
}
