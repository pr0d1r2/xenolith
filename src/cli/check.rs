//! `xnl check`: read `xenolith.toml`, run the engine, render the report
//! (`src:V152`, `src/cli` §I).
//!
//! RED stub (`src:T153`): the shape `check/tests.rs` is written against.
//! The dispatch still refuses `check`; the GREEN commit routes it here.

use std::io::Write;
use std::path::Path;

use super::args::{OutputFormat, Scan};
use super::{EXIT_OK, not_yet};
use crate::config::Config;
use crate::model::Report;

#[cfg(test)]
mod tests;

/// Run `xnl check` from `root` over `scan`. Stub: refuses.
pub fn run(
    _root: &Path,
    _scan: &Scan,
    _verbose: bool,
    _out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    not_yet(err, "check", "src:T153")
}

/// The config at `root`. Stub: the defaults.
///
/// # Errors
///
/// None yet.
pub fn load(_root: &Path) -> Result<Config, String> {
    Ok(Config::default())
}

/// Write `report` in `format` and return its exit code. Stub: nothing.
pub fn render(
    _report: &Report,
    _format: OutputFormat,
    _verbose: bool,
    _out: &mut impl Write,
    _err: &mut impl Write,
) -> u8 {
    EXIT_OK
}
