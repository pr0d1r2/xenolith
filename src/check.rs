//! The check engine: `xnl check` as a library call (`src:V152`).
//!
//! RED stub (`src:T153`): the types and seams `check/tests.rs` is written
//! against, with a pipeline that finds nothing. The GREEN commit joins
//! discovery, the registry, the verdict, `[[allow]]` and the report.

use std::fmt;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

use xenolith_lang_api::{Guest, Host, Site};

use crate::cli::EXIT_USAGE;
use crate::config::Config;
use crate::discover::DiscoverError;
use crate::model::Report;
use crate::registry::{self, MissingGuest};

#[cfg(test)]
mod tests;

/// The config file `xnl check` reads at the root (`src/config:C16`).
pub const CONFIG_FILE: &str = "xenolith.toml";

/// The warning code for a claimed file its host could not parse.
pub const HOST_PARSE_ERROR: &str = "host-parse-error";

/// What stands in for a host interpolation when a body is handed to its
/// guest.
const HOLE: &str = "XNL_HOLE";

/// What a run is asked to look at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The paths named; empty means every tracked file (`src:V57`).
    pub paths: Vec<PathBuf>,
}

/// Why a run was refused. Every variant is exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// Discovery refused.
    Discover(DiscoverError),
    /// A site's guest is compiled out (`src:V42`).
    MissingGuest(MissingGuest),
}

impl CheckError {
    /// The process exit code: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Discover(e) => e.fmt(f),
            CheckError::MissingGuest(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for CheckError {}

/// Check the tree at `root` under `config` (`src:V152`).
///
/// # Errors
///
/// None yet: this stub finds nothing.
pub fn check(root: &Path, config: &Config, options: &Options) -> Result<Report, CheckError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    check_with(root, config, options, &langs, &|| Command::new("git"))
}

/// The languages a run judges with.
pub(crate) struct Langs<'a> {
    pub(crate) hosts: &'a [&'a dyn Host],
    pub(crate) guests: &'a [&'a dyn Guest],
}

/// [`check`], with the languages and `git` supplied. Stub: an empty
/// report.
pub(crate) fn check_with(
    _root: &Path,
    _config: &Config,
    _options: &Options,
    _langs: &Langs<'_>,
    _git: &dyn Fn() -> Command,
) -> Result<Report, CheckError> {
    Ok(Report::new())
}

/// The `[[allow]]` content hash. Stub.
#[must_use]
pub fn body_hash(_body: &str) -> String {
    String::new()
}

/// The body as the guest reads it. Stub.
fn guest_text(_src: &str, _site: &Site) -> String {
    String::new()
}

/// 1-based line and column. Stub.
fn position(_src: &str, _offset: usize) -> (usize, usize) {
    (0, 0)
}

/// The first line of a file. Stub.
fn head(_path: &Path) -> String {
    String::new()
}

/// A candidate's repo-relative name. Stub.
fn repo_name(_path: &Path) -> String {
    String::new()
}
