//! What a finding IS, and the bytes it becomes.
//!
//! Every verb reports through this type (`src:V1`), and a consumer parses
//! the JSON rather than the prose, so the shape carries a version number
//! (`src/cli:V24`). Two properties are load-bearing and both are tested
//! rather than asserted in a comment:
//!
//! * ORDER. The scan runs in parallel (`src:V95`), so the model sorts
//!   rather than the caller; otherwise the same tree renders different
//!   bytes depending on which thread finished first (`src:V11`).
//! * COMPLETENESS. A violation names both languages, the sink, the
//!   delimiter kind, a reason, and at least one direction. Never a bare
//!   "bad" (`src:V1`): a report a reader cannot act on is a report that
//!   gets suppressed.

use std::cmp::Ordering;
use std::path::PathBuf;

use serde_json::{Map, Value, json};
use xenolith_lang_api::{DelimKind, LangId};

/// Whether a machine may apply a direction unattended.
///
/// Mirrors microlith's `Fix`, including the hard-won reading of
/// `Mechanical`: not "easy for a human", but "the tool computes the
/// single correct answer". Anything with more than one defensible
/// outcome is a [`Fix::Judgment`], because an agent applying a guess is
/// worse than an agent stopping.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fix {
    /// Deterministic and reversible. Safe to apply unattended.
    Mechanical,
    /// Accepts a trade or changes intent. Needs a human, or an
    /// instruction naming this specific trade.
    Judgment,
}

impl Fix {
    /// The name used in JSON output.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Fix::Mechanical => "mechanical",
            Fix::Judgment => "judgment",
        }
    }
}

/// One way to resolve a violation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Direction {
    /// Whether a machine may apply it.
    pub kind: Fix,
    /// What to do, in the imperative.
    pub action: String,
}

/// Every rule xenolith can report.
///
/// A closed set, because a rule id is a public contract at a schema
/// version (`src/cli:V24`): a consumer matches on it, and a new id
/// appearing unannounced is a matcher that silently stops covering
/// everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rule {
    /// A load points at a file that does not exist (`src/graph:V7`).
    DanglingLoad,
    /// A claimed host file did not parse (`languages:V78`).
    HostParseError,
    /// An extract nothing loads (`src/graph:V7`).
    OrphanExtract,
    /// An `[[allow]]` entry matching nothing (`src/config:V9`).
    StaleAllow,
    /// An `exclude` pattern matching nothing (`src/config:V79`).
    StaleExclude,
    /// A config rule naming something unknown (`src/config:V44`).
    StaleRule,
    /// Non-trivial guest code in a host sink: the finding this tool
    /// exists for.
    Xenolith,
}

impl Rule {
    /// Every rule, in id order.
    pub const ALL: &'static [Rule] = &[
        Rule::DanglingLoad,
        Rule::HostParseError,
        Rule::OrphanExtract,
        Rule::StaleAllow,
        Rule::StaleExclude,
        Rule::StaleRule,
        Rule::Xenolith,
    ];

    /// The stable, kebab-case id.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Rule::DanglingLoad => "dangling-load",
            Rule::HostParseError => "host-parse-error",
            Rule::OrphanExtract => "orphan-extract",
            Rule::StaleAllow => "stale-allow",
            Rule::StaleExclude => "stale-exclude",
            Rule::StaleRule => "stale-rule",
            Rule::Xenolith => "xenolith",
        }
    }

    /// The inverse, exact-match only.
    #[must_use]
    pub fn from_id(id: &str) -> Option<Rule> {
        Rule::ALL.iter().copied().find(|rule| rule.as_str() == id)
    }
}

/// One finding, with everything a reader needs to act on it (`src:V1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Which rule fired. Match on this, never on `why`.
    pub rule: Rule,
    /// The host file, repo-root relative.
    pub file: PathBuf,
    /// 1-based line of the site.
    pub line: usize,
    /// 1-based column of the site.
    pub col: usize,
    /// The language of the file.
    pub host: LangId,
    /// The language found inside it.
    pub guest: LangId,
    /// The host-syntax context, in the host's own vocabulary.
    pub sink: String,
    /// The delimiter form the guest code sits in.
    pub site: DelimKind,
    /// One clause: what goes wrong if this stands.
    pub why: String,
    /// Ranked ways out, the correct one first, the escape hatch last.
    pub directions: Vec<Direction>,
}

impl Violation {
    /// The human line: `file:line:col rule: guest in host sink (why)`
    /// (`src/cli` §I).
    #[must_use]
    pub fn to_human(&self) -> String {
        format!(
            "{}:{}:{} {}: {} in {} {} ({})",
            self.file.display(),
            self.line,
            self.col,
            self.rule.as_str(),
            self.guest,
            self.host,
            self.sink,
            self.why
        )
    }

    /// The sort key (`src:V11`): file, line, column, then rule so two
    /// findings at one position still have a fixed order.
    fn sort_key(&self) -> (&PathBuf, usize, usize, Rule) {
        (&self.file, self.line, self.col, self.rule)
    }

    fn to_value(&self) -> Value {
        json!({
            "rule": self.rule.as_str(),
            "file": self.file.display().to_string(),
            "line": self.line,
            "col": self.col,
            "host": self.host.as_str(),
            "guest": self.guest.as_str(),
            "sink": self.sink,
            "site": delim_kind_name(&self.site),
            "why": self.why,
            "directions": self
                .directions
                .iter()
                .map(|d| json!({ "kind": d.kind.as_str(), "action": d.action }))
                .collect::<Vec<Value>>(),
        })
    }
}

/// Something worth saying that is not a finding.
///
/// Warnings never change the exit code (`src/cli` §I). A skipped symlink
/// or an unclaimed file is information about the run, and failing a gate
/// on it would teach people to pass `--quiet`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Warning {
    /// Stable kebab-case code, matched like a rule id.
    pub code: String,
    /// The file it concerns, when there is one.
    pub file: Option<PathBuf>,
    /// What happened, in one sentence.
    pub message: String,
}

impl Warning {
    fn to_value(&self) -> Value {
        let mut map = Map::new();
        map.insert("code".to_owned(), Value::String(self.code.clone()));
        map.insert("message".to_owned(), Value::String(self.message.clone()));
        if let Some(file) = &self.file {
            map.insert("file".to_owned(), Value::String(file.display().to_string()));
        }
        Value::Object(map)
    }
}

/// The JSON schema version (`src/cli:V24`).
///
/// ONE number across every verb: a shape change anywhere bumps it, so a
/// consumer checks one field rather than tracking which verb changed
/// when.
pub const SCHEMA: u32 = 1;

/// Everything one run has to say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    violations: Vec<Violation>,
    warnings: Vec<Warning>,
}

impl Report {
    /// An empty report.
    #[must_use]
    pub fn new() -> Report {
        Report::default()
    }

    /// Add a violation, keeping the list sorted.
    ///
    /// Sorted on INSERT rather than before rendering: a caller that
    /// forgets to sort produces output that is correct on one machine and
    /// different on another, which is the failure `src:V11` exists to
    /// make impossible rather than unlikely.
    pub fn push(&mut self, violation: Violation) {
        let at = self
            .violations
            .partition_point(|existing| existing.sort_key() < violation.sort_key());
        self.violations.insert(at, violation);
    }

    /// Add a warning, keeping the list sorted by code then file.
    pub fn warn(&mut self, warning: Warning) {
        let key = |w: &Warning| (w.code.clone(), w.file.clone());
        let at = self
            .warnings
            .partition_point(|existing| key(existing).cmp(&key(&warning)) == Ordering::Less);
        self.warnings.insert(at, warning);
    }

    /// The violations, in report order.
    #[must_use]
    pub fn violations(&self) -> &[Violation] {
        &self.violations
    }

    /// The warnings, in report order.
    #[must_use]
    pub fn warnings(&self) -> &[Warning] {
        &self.warnings
    }

    /// The process exit code: 1 when anything was found, else 0
    /// (`src/cli` §I). Usage and config errors are 2 and are decided by
    /// the CLI, not here -- this type only knows what the scan saw.
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        i32::from(!self.violations.is_empty())
    }

    /// The JSON envelope, pretty-printed, newline-terminated.
    ///
    /// Keys come out sorted because `serde_json`'s map is a `BTreeMap`,
    /// so the bytes are stable without a sorting pass anybody could
    /// forget (`src:V11`).
    #[must_use]
    pub fn to_json(&self) -> String {
        let envelope = json!({
            "schema": SCHEMA,
            "violations": self.violations.iter().map(Violation::to_value).collect::<Vec<Value>>(),
            "warnings": self.warnings.iter().map(Warning::to_value).collect::<Vec<Value>>(),
        });
        let mut out = serde_json::to_string_pretty(&envelope)
            .unwrap_or_else(|_| String::from("{\"schema\": 1}"));
        out.push('\n');
        out
    }
}

/// The JSON name of a delimiter kind.
///
/// Here rather than in the api crate because the api crate has one
/// dependency and serde is not it (`languages/api:V32`) -- and because
/// these names are part of the JSON contract (`src/cli:V24`), which is
/// this crate's to keep.
#[must_use]
pub fn delim_kind_name(kind: &DelimKind) -> &'static str {
    match *kind {
        DelimKind::NixIndented => "nix-indented",
        DelimKind::NixString => "nix-string",
        DelimKind::Heredoc { .. } => "heredoc",
        DelimKind::PklMultiline { .. } => "pkl-multiline",
        DelimKind::YamlBlock { .. } => "yaml-block",
        DelimKind::HtmlElement { .. } => "html-element",
        DelimKind::RustRawString { .. } => "rust-raw-string",
        DelimKind::RubyHeredoc { .. } => "ruby-heredoc",
        DelimKind::ArgvString => "argv-string",
        DelimKind::JustRecipe => "just-recipe",
        DelimKind::JustShebangRecipe => "just-shebang-recipe",
    }
}
