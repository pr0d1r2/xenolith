//! `xnl migrate`: legacy per-file allowlists into `xenolith.toml`
//! (`src/cli:T97`, `src/cli` §I). A stub until the tests are in place.

use std::io::Write;
use std::path::Path;

use super::EXIT_OK;
use crate::config::Allow;

#[cfg(test)]
mod tests;

/// The name every legacy list ends with, after `.<lang>`.
pub const LEGACY_SUFFIX: &str = "-embedded-shell-allowlist";

/// Warning code: a listed path that is not a file in the tree.
pub const LEGACY_MISSING: &str = "legacy-missing";

/// Warning code: a listed file holding no site xenolith flags.
pub const LEGACY_NO_SITE: &str = "legacy-no-site";

/// One path line of a legacy list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The listed path, repo-root relative.
    pub path: String,
    /// 1-based line in the list.
    pub line: usize,
    /// The comment block directly above the entry's group, if any.
    pub comment: Option<String>,
}

/// Whether `name` is a legacy list's file name.
#[must_use]
pub fn is_legacy(_name: &str) -> bool {
    false
}

/// The entries of a legacy list.
#[must_use]
pub fn parse_legacy(_text: &str) -> Vec<Entry> {
    Vec::new()
}

/// The legacy lists at `root`, sorted.
///
/// # Errors
///
/// The message to refuse with when `root` cannot be listed.
pub fn legacy_lists(_root: &Path) -> Result<Vec<String>, String> {
    Ok(Vec::new())
}

/// `text` as a TOML basic string.
#[must_use]
pub fn toml_string(_text: &str) -> String {
    String::new()
}

/// `xenolith.toml` holding `allows`.
#[must_use]
pub fn render(_allows: &[Allow]) -> String {
    String::new()
}

/// The unified diff creating `name` with `text`.
#[must_use]
pub fn creation_diff(_name: &str, _text: &str) -> String {
    String::new()
}

/// Run `xnl migrate` from `root`.
pub fn run(
    _root: &Path,
    _write: bool,
    _verbose: bool,
    _strict_hosts: bool,
    _out: &mut impl Write,
    _err: &mut impl Write,
) -> u8 {
    EXIT_OK
}
