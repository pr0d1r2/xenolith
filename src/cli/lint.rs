//! `xnl lint`: read `xenolith.toml`, run the lint engine, render its
//! results (`src/lint:V8`, `src/cli` §I).
//!
//! Input and output only, as `xnl check`'s module is: the engine
//! ([`crate::lint`]) decides what ran and how it ended; this module
//! decides the stream, the format and the exit code -- 0 every check
//! passed, 1 one failed, 2 one could not run or the run was refused
//! (`src/lint:V92`, `src/cli:V24`).

use std::io::Write;
use std::path::Path;

use super::args::{OutputFormat, Scan};
use super::not_yet;
use crate::lint::LintReport;

#[cfg(test)]
mod tests;

/// The flags `xnl lint` takes beyond the scan (`src/cli` §I).
///
/// Four switches, each a flag the user typed; an enum would only rename
/// them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--fix`.
    pub fix: bool,
    /// `--trust-config`.
    pub trust_config: bool,
    /// `--verbose`.
    pub verbose: bool,
    /// `--strict-hosts`.
    pub strict_hosts: bool,
}

/// Run `xnl lint` from `root` over `scan`, writing to `out` and `err`.
pub fn run(
    root: &Path,
    scan: &Scan,
    flags: Flags,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let _ = (root, scan, flags, out);
    not_yet(err, "lint", "src/lint:T24")
}

/// Write `report` in `format` and return its exit code.
pub fn render(
    report: &LintReport,
    format: OutputFormat,
    verbose: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let _ = (report, format, verbose, out, err);
    0
}
