//! `xenolith-dev` -- tooling that maintains THIS repository (`dev` §G).
//!
//! RED: the interface the tests in `tests.rs` and each module's mirror pin
//! down, with no behaviour behind it yet (`dev:T340`, `dev:T341`).

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

pub mod badge;
pub mod langs;
pub mod notices;
pub mod select;
pub mod splice;

#[cfg(test)]
mod tests;

/// What the binary prints for a usage error or `--help`.
pub const USAGE: &str = "xenolith-dev\n";

/// Where the notices' two external readings come from.
#[derive(Debug, Clone, Default)]
pub struct External {
    /// A file holding `cargo metadata --format-version 1` output.
    pub metadata: Option<PathBuf>,
    /// A file holding `nix eval --json .#default.toolLicenses` output.
    pub tools: Option<PathBuf>,
}

/// The binary: arguments and environment in, exit code out.
#[must_use]
pub fn main() -> ExitCode {
    ExitCode::from(2)
}

/// The repository root.
#[must_use]
pub fn repo_root(_start: &Path) -> Option<PathBuf> {
    None
}

/// Dispatch `args` against the repository at `root`.
pub fn run(
    _args: &[String],
    _root: Option<&Path>,
    _external: &External,
    _err: &mut dyn Write,
) -> u8 {
    2
}

/// Every `SPEC.md` in the tree.
#[must_use]
pub fn spec_paths(_root: &Path) -> Vec<String> {
    Vec::new()
}

/// Every badge input, read from its owner.
///
/// # Errors
/// Not yet.
pub fn sources(_root: &Path) -> Result<badge::Sources, String> {
    Err(String::new())
}

/// The README's generated blocks.
///
/// # Errors
/// Not yet.
pub fn readme_blocks(_root: &Path) -> Result<splice::Blocks, String> {
    Ok(Vec::new())
}

/// Run an owner and take what it prints.
///
/// # Errors
/// Not yet.
pub fn capture(_root: &Path, _program: &str, _args: &[&str]) -> Result<String, String> {
    Ok(String::new())
}

/// Every vendored grammar.
///
/// # Errors
/// Not yet.
pub fn vendored(_root: &Path) -> Result<Vec<notices::Vendored>, String> {
    Ok(Vec::new())
}

/// The notices as they should read today.
///
/// # Errors
/// Not yet.
pub fn notices_text(_root: &Path, _external: &External) -> Result<String, String> {
    Ok(String::new())
}
