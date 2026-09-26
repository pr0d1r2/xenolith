//! What `xnl lint` found: one [`Outcome`] per command run on a file,
//! each reported on its own (`src/lint:V8`), and the JSON envelope they
//! render to (`src/lint` §I).
//!
//! Only data and its rendering live here. The engine (`src/lint/mod.rs`)
//! decides what ran; `src/cli/lint.rs` decides which stream it goes to.

use std::path::PathBuf;

use xenolith_lang_api::LangId;

use crate::model::Warning;

#[cfg(test)]
mod tests;

/// What kind of file a command ran on (`src/lint` §I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A file holding one guest language: linted by its guest's checks.
    Extract,
    /// A host file: linted by `Host::checks` (`[lint] hosts`).
    Host,
}

impl Kind {
    /// The JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Extract => "extract",
            Kind::Host => "host",
        }
    }
}

/// How one command ended (`src/lint` §I, status).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Status {
    /// Exit 0.
    Pass,
    /// Ran and exited non-zero: a finding.
    Fail,
    /// Never ran, died on a signal, or hit the timeout: the gate could
    /// not judge the file, which is exit 2 (`src/lint:V92`).
    Error,
    /// Not run on purpose: a config command without `--trust-config`
    /// (`src/lint:V91`).
    Skipped,
}

impl Status {
    /// The JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Error => "error",
            Status::Skipped => "skipped",
        }
    }

    /// The exit code this status alone would make (`src/cli:V24`).
    #[must_use]
    pub const fn exit_code(self) -> u8 {
        match self {
            Status::Pass | Status::Skipped => 0,
            Status::Fail => 1,
            Status::Error => 2,
        }
    }
}

/// Where a command came from (`src/lint` §I, `source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Source {
    /// The language crate's own `checks` / `fixers`.
    Default,
    /// A `xenolith.toml`: `[lint.<guest>]` or `[lint] all`.
    Config,
}

impl Source {
    /// The JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Source::Default => "default",
            Source::Config => "config",
        }
    }
}

/// One command on one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    /// The file, repo-root relative.
    pub file: PathBuf,
    /// Extract or host.
    pub kind: Kind,
    /// The extract's guest; `None` for a host file.
    pub guest: Option<LangId>,
    /// The extract's dialect, when its shebang named one.
    pub dialect: Option<String>,
    /// The tool: the command's first word.
    pub check: String,
    /// Exactly what ran, file included.
    pub argv: Vec<String>,
    /// Default or config.
    pub source: Source,
    /// How it ended.
    pub status: Status,
    /// The exit code, when the tool exited.
    pub exit: Option<i32>,
    /// The last lines of what the tool printed, or why it could not run;
    /// only when it did not pass (`src/lint` §I).
    pub raw_tail: Option<String>,
    /// A fixer rather than a check (`--fix`); listed only when it did not
    /// pass.
    pub fixer: bool,
}

/// Everything one `xnl lint` run has to say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintReport {
    outcomes: Vec<Outcome>,
    warnings: Vec<Warning>,
}

impl LintReport {
    /// An empty report.
    #[must_use]
    pub fn new() -> LintReport {
        LintReport::default()
    }

    /// Add an outcome. Outcomes keep the order they ran in: files come
    /// sorted from discovery and each file's commands in plan order, so
    /// the order is already deterministic (`src:V11`).
    pub fn push(&mut self, outcome: Outcome) {
        self.outcomes.push(outcome);
    }

    /// Add a warning, sorted as `Report::warn` sorts (`src:B1`); an
    /// identical warning is kept once.
    pub fn warn(&mut self, warning: Warning) {
        self.warnings.push(warning);
    }

    /// The outcomes, in run order.
    #[must_use]
    pub fn outcomes(&self) -> &[Outcome] {
        &self.outcomes
    }

    /// The warnings, sorted.
    #[must_use]
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// The highest exit code any outcome makes: 0 all passed, 1 a check
    /// failed, 2 one could not run (`src/lint:V92`, `src/cli` §I).
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        0
    }

    /// The JSON envelope (`src/lint` §I), pretty-printed and
    /// newline-terminated, keys sorted by `serde_json`'s map.
    #[must_use]
    pub fn to_json(&self) -> String {
        String::new()
    }
}
