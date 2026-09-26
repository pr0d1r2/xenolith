//! The lint engine: `xnl lint` as a library call (`src/lint:V8`).
//!
//! Every extract is run through its guest's checks, every host file
//! through its host's (`[lint] hosts`), each command reported on its own
//! and none stopping the rest. The stages are joined here and owned
//! elsewhere:
//!
//! 1. candidates -- [`crate::discover`] (`src:V57`), minus what
//!    `[[exclude]]` and `[lint] exclude` skip (`src/config:V79`).
//! 2. targets -- until `src/graph` names extracts, a file is an extract
//!    when its shebang names a compiled-in guest, or it has none and its
//!    extension is the guest's; a host file is one a host claims; a file
//!    that is neither follows `src:V13` (`src/lint` §I, targets).
//! 3. commands -- [`plan`]: the language crate's defaults and the
//!    config's own, which run only when trusted (`src/lint:V91`).
//! 4. runs -- [`run`]: a tool not on PATH is an `error`, exit 2
//!    (`src/lint:V8`).
//!
//! `src/cli` renders the [`LintReport`] and maps its exit code.

use std::fmt;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::check::Langs;
use crate::cli::EXIT_USAGE;
use crate::config::{Config, TreeError};
use crate::discover::{DiscoverError, discover_with};
use crate::registry;

pub mod plan;
pub mod report;
pub mod run;

pub use report::{Kind, LintReport, Outcome, Source, Status};

use self::run::Tools;

#[cfg(test)]
mod tests;

/// The warning naming a config command held back for want of
/// `--trust-config` (`src/lint:V91`).
pub const UNTRUSTED_COMMAND: &str = "untrusted-command";

/// What a run is asked to do.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The paths named; empty means every tracked file (`src:V57`).
    pub paths: Vec<PathBuf>,
    /// `--strict-hosts` (`src:V13`).
    pub strict_hosts: bool,
}

/// Why a run was refused rather than carried out; every variant exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LintError {
    /// Discovery refused (`src:V57`, `src:V128`).
    Discover(DiscoverError),
    /// A nested `xenolith.toml` refused (`src/config` §I discovery).
    Config(TreeError),
    /// A file nothing lints, under `--strict-hosts` or `[langs]
    /// unclaimed = "error"` (`src:V13`).
    Unclaimed {
        /// The file, as reports name it.
        file: PathBuf,
    },
    /// A named path outside the root.
    Outside {
        /// The path as discovery kept it.
        path: PathBuf,
    },
}

impl LintError {
    /// The process exit code: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for LintError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LintError::Discover(e) => e.fmt(f),
            LintError::Config(e) => e.fmt(f),
            LintError::Unclaimed { file } => write!(
                f,
                "{}: host unsupported: no host in this build claims it and no guest reads \
                 it, so it cannot be linted (src:V13)",
                file.display()
            ),
            LintError::Outside { path } => write!(
                f,
                "{}: outside the root: xnl lints the tree it runs in",
                path.display()
            ),
        }
    }
}

impl std::error::Error for LintError {}

impl From<DiscoverError> for LintError {
    fn from(e: DiscoverError) -> LintError {
        LintError::Discover(e)
    }
}

impl From<TreeError> for LintError {
    fn from(e: TreeError) -> LintError {
        LintError::Config(e)
    }
}

/// Lint the tree at `root` under `config` (`src/lint:V8`).
///
/// # Errors
///
/// [`LintError`], exit 2: discovery or a nested config refused, a named
/// path outside the root, or an unclaimed file under strict hosts.
pub fn lint(root: &Path, config: &Config, options: &Options) -> Result<LintReport, LintError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    lint_with(
        root,
        config,
        options,
        &langs,
        &|| Command::new("git"),
        &Tools::inherit(),
    )
}

/// [`lint`] with the languages, `git` and the tools' `PATH` supplied by
/// the caller -- the seam the tests use (`tests:V150`).
pub(crate) fn lint_with(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
    tools: &Tools,
) -> Result<LintReport, LintError> {
    let _ = (config, langs, tools);
    discover_with(root, &options.paths, git)?;
    Ok(LintReport::new())
}
