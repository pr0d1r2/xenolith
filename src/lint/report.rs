//! What `xnl lint` found: one [`Outcome`] per command run on a file,
//! each reported on its own (`src/lint:V8`), and the JSON envelope they
//! render to (`src/lint` §I).
//!
//! Only data and its rendering live here. The engine (`src/lint/mod.rs`)
//! decides what ran; `src/cli/lint.rs` decides which stream it goes to.

use std::cmp::Ordering;
use std::path::PathBuf;

use serde_json::{Map, Value, json};
use xenolith_lang_api::LangId;

use super::findings::Finding;
use crate::model::{SCHEMA, Warning};

#[cfg(test)]
mod tests;

/// What kind of file a command ran on (`src/lint` §I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    /// A file holding one guest language: linted by its guest's checks.
    Extract,
    /// A host file: linted by `Host::checks` (`[lint] hosts`).
    Host,
    /// A site inside a host file, linted in place under `--sites`
    /// (`src/lint:V93`): the file is the host, findings in its lines.
    Site,
}

impl Kind {
    /// The JSON name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Kind::Extract => "extract",
            Kind::Host => "host",
            Kind::Site => "site",
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
    /// What the tool's machine-readable output said, sorted; empty when
    /// it has none or it did not parse (`src/lint:V92`).
    pub findings: Vec<Finding>,
}

impl Outcome {
    fn to_value(&self) -> Value {
        json!({
            "file": self.file.display().to_string(),
            "kind": self.kind.as_str(),
            "guest": self.guest.map(LangId::as_str),
            "dialect": self.dialect,
            "check": self.check,
            "argv": self.argv,
            "source": self.source.as_str(),
            "status": self.status.as_str(),
            "exit": self.exit,
            "findings": self.findings.iter().map(finding_value).collect::<Vec<Value>>(),
            "raw_tail": self.raw_tail,
        })
    }
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

    /// Add a warning, sorted as `Report::warn` sorts (`src/check:B1`); an
    /// identical warning is kept once.
    pub fn warn(&mut self, warning: Warning) {
        let key = |w: &Warning| (w.code.clone(), w.file.clone(), w.message.clone());
        let at = self
            .warnings
            .partition_point(|existing| key(existing).cmp(&key(&warning)) == Ordering::Less);
        if self.warnings.get(at) != Some(&warning) {
            self.warnings.insert(at, warning);
        }
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
        self.outcomes
            .iter()
            .map(|o| o.status.exit_code())
            .max()
            .unwrap_or(0)
    }

    /// The JSON envelope (`src/lint` §I), pretty-printed and
    /// newline-terminated, keys sorted by `serde_json`'s map.
    #[must_use]
    pub fn to_json(&self) -> String {
        let envelope = json!({
            "schema": SCHEMA,
            "results": self.outcomes.iter().map(Outcome::to_value).collect::<Vec<Value>>(),
            "violations": Vec::<Value>::new(),
            "warnings": self.warnings.iter().map(warning_value).collect::<Vec<Value>>(),
        });
        let mut out = serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|_| String::from("{\"schema\": 1}"));
        out.push('\n');
        out
    }
}

/// A warning in the envelope's shape (`src/cli` §I): `code`, `message`,
/// and `file` when there is one.
fn warning_value(warning: &Warning) -> Value {
    let mut map = Map::new();
    map.insert("code".to_owned(), Value::String(warning.code.clone()));
    map.insert("message".to_owned(), Value::String(warning.message.clone()));
    if let Some(file) = &warning.file {
        map.insert("file".to_owned(), Value::String(file.display().to_string()));
    }
    Value::Object(map)
}

/// A finding in the envelope's shape (`src/lint` §I).
fn finding_value(finding: &Finding) -> Value {
    json!({
        "line": finding.line,
        "col": finding.col,
        "code": finding.code,
        "severity": finding.severity,
        "message": finding.message,
    })
}
