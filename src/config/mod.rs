//! `xenolith.toml`: parse, validate, and resolve against the defaults
//! table (`src/config`).
//!
//! One file at the consumer's repo root replaces every per-language
//! allowlist (`src/config:C16`). This module turns its text into a typed
//! [`Config`] and nothing else: APPLYING the config is each verb's job
//! (`src` §F), and discovery and merge across nested files is `.:T91`.
//!
//! Two rules shape every function below:
//!
//! * REFUSE, never skip. An unknown key, a wrong type or a value outside
//!   a closed set is a [`ConfigError`] naming the key. A config the tool
//!   half-understands is worse than one it rejects: the half it skipped
//!   is a rule the user believes is in force.
//! * NO LITERAL DEFAULTS outside [`defaults`] (`src/config:V73`). A field
//!   the file leaves unset is filled from the named constant there, and
//!   [`Config::source`] says which of the two it came from, which is what
//!   `--verbose` prints.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use toml::{Table, Value};
use xenolith_lang_api::LangId;

mod allow;
pub mod defaults;

pub use allow::SiteKey;

#[cfg(test)]
mod tests;

/// The schema versions this build reads (`src/config:V70`).
pub const VERSIONS: &[i64] = &[1];

/// Why a config was refused. The CLI maps it to exit 2 (`src/cli` §I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigError {
    /// The dotted key at fault (`extract.depth`, `allow[0].reason`), or
    /// empty when the text is not TOML at all.
    pub key: String,
    /// What is wrong, and what would be accepted.
    pub message: String,
}

impl ConfigError {
    fn new(key: impl Into<String>, message: impl Into<String>) -> ConfigError {
        ConfigError {
            key: key.into(),
            message: message.into(),
        }
    }

    fn unknown(key: &str) -> ConfigError {
        ConfigError::new(key, "unknown key")
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.key.is_empty() {
            write!(f, "xenolith.toml: {}", self.message)
        } else {
            write!(f, "xenolith.toml: {}: {}", self.key, self.message)
        }
    }
}

impl std::error::Error for ConfigError {}

/// The `ignore | warn | error` policy shared by several keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Policy {
    /// Say nothing.
    Ignore,
    /// A warning, which never changes the exit code.
    Warn,
    /// Exit 2.
    Error,
}

impl Policy {
    fn from_choice(choice: &str) -> Policy {
        match choice {
            "ignore" => Policy::Ignore,
            "warn" => Policy::Warn,
            _ => Policy::Error,
        }
    }

    /// The name used in `xenolith.toml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Policy::Ignore => "ignore",
            Policy::Warn => "warn",
            Policy::Error => "error",
        }
    }
}

/// `[extract] layout` (`src/extract` §I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layout {
    /// The host's own placement only.
    Host,
    /// `<root>/<host path sans ext>/<name>.<ext>`.
    Mirror,
    /// `<host_dir>/<host_stem>.<name>.<ext>`.
    Sibling,
    /// `<root>/<guest>/<name>.<ext>`.
    Central,
}

impl Layout {
    fn from_choice(choice: &str) -> Layout {
        match choice {
            "mirror" => Layout::Mirror,
            "sibling" => Layout::Sibling,
            "central" => Layout::Central,
            _ => Layout::Host,
        }
    }

    /// The name used in `xenolith.toml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Layout::Host => "host",
            Layout::Mirror => "mirror",
            Layout::Sibling => "sibling",
            Layout::Central => "central",
        }
    }
}

/// `strict` for a shell prelude (`languages/shell` §I).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Strict {
    /// Reproduce the options the site ran under, add none.
    Preserve,
    /// `set -euo pipefail` regardless, marked as a `Judgment`.
    Enforce,
}

impl Strict {
    fn from_choice(choice: &str) -> Strict {
        if choice == "enforce" {
            Strict::Enforce
        } else {
            Strict::Preserve
        }
    }

    /// The name used in `xenolith.toml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Strict::Preserve => "preserve",
            Strict::Enforce => "enforce",
        }
    }
}

/// A rule's `base` (`src/extract` §I): what an extract path is relative
/// to at runtime. Overrides `Host::runtime_base`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Base {
    /// The host file's directory.
    Host,
    /// The repo root.
    Root,
    /// A named directory.
    Dir(String),
}

/// `[extract.shell]` (`languages/shell` §I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractShell {
    /// Prelude policy.
    pub strict: Strict,
}

/// A rule's `prelude = { shebang, strict }`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Prelude {
    /// The first line of the extract.
    pub shebang: Option<String>,
    /// Prelude policy for this rule.
    pub strict: Option<Strict>,
}

/// One `[[extract.rule]]` (`src/extract` §I, layer B).
///
/// Every field optional: a rule sets only what it overrides, and
/// resolution per field is `src/extract:V45`. Templates are carried as
/// written; their validation is `src/config:T49`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtractRule {
    /// Match: host language.
    pub host: Option<LangId>,
    /// Match: sink glob, `*` = one dotted segment.
    pub sink: Option<String>,
    /// Match: guest language.
    pub guest: Option<LangId>,
    /// Path template.
    pub path: Option<String>,
    /// Runtime base.
    pub base: Option<Base>,
    /// Argv template.
    pub invoke: Option<Vec<String>>,
    /// Prelude override.
    pub prelude: Option<Prelude>,
    /// Whether the extract is written executable.
    pub executable: Option<bool>,
    /// Companion path template.
    pub companion: Option<String>,
}

/// `[extract]` (`src/extract` §I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extract {
    /// Placement layout (layer C).
    pub layout: Layout,
    /// Root for `mirror` and `central`.
    pub root: String,
    /// Max nesting levels per run, ≥ 1.
    pub depth: u64,
    /// Rules for a compiled-out or disabled language.
    pub inactive_rules: Policy,
    /// `[extract.shell]`.
    pub shell: ExtractShell,
    /// `[[extract.rule]]`, in file order.
    pub rules: Vec<ExtractRule>,
}

/// One `[[allow]]` (`src/config:V9`, `src/config:V10`).
///
/// Keyed by host path, sink path and the body's content hash -- never
/// by position, so an edit above the site keeps the allow and an edit to
/// the body invalidates it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Allow {
    /// Host file, repo-root relative, no wildcard.
    pub path: String,
    /// Sink path inside the host, e.g. `systemd.services.foo.script`.
    pub sink: String,
    /// Content hash of the body.
    pub hash: String,
    /// Why this site stays inline. Non-empty.
    pub reason: String,
}

/// `[lint.<guest>]` (`src/lint` §I).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LintGuest {
    /// Extra or replacement checks.
    pub checks: Vec<String>,
    /// Extra or replacement fixers.
    pub fixers: Vec<String>,
    /// Append (`true`) or replace; unset = the table default, resolved
    /// by [`Config::effective`].
    pub extend: Option<bool>,
}

/// `[lint]` (`src/lint` §I).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Lint {
    /// Run `Host::checks` on host files.
    pub hosts: bool,
    /// Checks for every extract regardless of guest.
    pub all: Vec<String>,
    /// Per check wall clock, seconds; 0 = no limit.
    pub timeout: u64,
    /// The per-guest linter map.
    pub guests: BTreeMap<LangId, LintGuest>,
}

/// `[langs]`: the runtime policies over the compiled-in set (`src:C1`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Langs {
    /// A file no host claims (`src:V13`).
    pub unclaimed: Policy,
    /// A site whose guest is compiled out (`src:V42`).
    pub missing_guest: Policy,
}

/// `[parse]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parse {
    /// A host file with parse `ERROR` nodes.
    pub host_errors: Policy,
}

/// `[threshold.<guest>]` for a guest other than shell. Unset fields
/// resolve through the table.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuestThreshold {
    /// Inline ceiling, lines.
    pub max_lines: Option<u64>,
    /// Inline ceiling, bytes.
    pub max_bytes: Option<u64>,
}

/// `[threshold.*]` (`src/config:V55`), validated at parse: every guest
/// a known language, every construct one of
/// [`defaults::SHELL_CONSTRUCTS`], every count non-negative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Threshold {
    /// `[threshold.shell] allow`: constructs tolerated inline.
    pub shell_allow: Vec<String>,
    /// `[threshold.exec] max_args`.
    pub exec_max_args: u64,
    /// `[threshold.exec] max_len`.
    pub exec_max_len: u64,
    /// `[threshold.load] max_params`.
    pub load_max_params: u64,
    /// `[threshold.load] param_prefix`.
    pub load_param_prefix: String,
    /// `[threshold.<guest>]`, every guest but shell.
    pub guests: BTreeMap<LangId, GuestThreshold>,
}

/// A resolved `xenolith.toml`.
///
/// [`Config::default`] is the config of a repo with no file at all
/// (`src/config:V88`): every field from [`defaults`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Schema version.
    pub version: i64,
    /// `[extract]`, `[extract.shell]`, `[[extract.rule]]`.
    pub extract: Extract,
    /// `[[allow]]`, in file order.
    pub allow: Vec<Allow>,
    /// `[lint]` and the linter map.
    pub lint: Lint,
    /// `[langs]`.
    pub langs: Langs,
    /// `[parse]`.
    pub parse: Parse,
    /// `[threshold.*]`.
    pub threshold: Threshold,
    /// Concrete keys the file set, for [`Config::source`].
    set: BTreeSet<String>,
}

impl Default for Config {
    fn default() -> Config {
        use defaults as d;
        Config {
            version: 1,
            extract: Extract {
                layout: Layout::from_choice(d::EXTRACT_LAYOUT),
                root: d::EXTRACT_ROOT.to_owned(),
                depth: d::EXTRACT_DEPTH,
                inactive_rules: Policy::from_choice(d::EXTRACT_INACTIVE_RULES),
                shell: ExtractShell {
                    strict: Strict::from_choice(d::EXTRACT_SHELL_STRICT),
                },
                rules: Vec::new(),
            },
            allow: Vec::new(),
            lint: Lint {
                hosts: d::LINT_HOSTS,
                all: Vec::new(),
                timeout: d::LINT_TIMEOUT,
                guests: BTreeMap::new(),
            },
            langs: Langs {
                unclaimed: Policy::from_choice(d::LANGS_UNCLAIMED),
                missing_guest: Policy::from_choice(d::LANGS_MISSING_GUEST),
            },
            parse: Parse {
                host_errors: Policy::from_choice(d::PARSE_HOST_ERRORS),
            },
            threshold: Threshold {
                shell_allow: d::THRESHOLD_SHELL_ALLOW
                    .iter()
                    .map(|s| (*s).to_owned())
                    .collect(),
                exec_max_args: d::THRESHOLD_EXEC_MAX_ARGS,
                exec_max_len: d::THRESHOLD_EXEC_MAX_LEN,
                load_max_params: d::THRESHOLD_LOAD_MAX_PARAMS,
                load_param_prefix: d::THRESHOLD_LOAD_PARAM_PREFIX.to_owned(),
                guests: BTreeMap::new(),
            },
            set: BTreeSet::new(),
        }
    }
}

/// One effective value, as `--verbose` reports it (`src/config:V73`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effective {
    /// A count or duration.
    Int(u64),
    /// A string or policy name.
    Str(String),
    /// A switch.
    Bool(bool),
    /// A list of strings.
    List(Vec<String>),
}

/// Where an effective value came from (`src/config:V73`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// The defaults table.
    Default,
    /// The file.
    File,
}

impl Config {
    /// The effective value of a concrete dotted key (`threshold.nix.max_lines`,
    /// `lint.shell.extend`), or `None` for a key the table does not hold
    /// or one resolved per site (`extract.rule.base`).
    #[must_use]
    pub fn effective(&self, key: &str) -> Option<Effective> {
        let parts: Vec<&str> = key.split('.').collect();
        let str_ = |s: &str| Some(Effective::Str(s.to_owned()));
        match parts.as_slice() {
            ["extract", "depth"] => Some(Effective::Int(self.extract.depth)),
            ["extract", "inactive_rules"] => str_(self.extract.inactive_rules.as_str()),
            ["extract", "layout"] => str_(self.extract.layout.as_str()),
            ["extract", "root"] => str_(&self.extract.root),
            ["extract", "shell", "strict"] => str_(self.extract.shell.strict.as_str()),
            ["langs", "missing_guest"] => str_(self.langs.missing_guest.as_str()),
            ["langs", "unclaimed"] => str_(self.langs.unclaimed.as_str()),
            ["lint", "hosts"] => Some(Effective::Bool(self.lint.hosts)),
            ["lint", "timeout"] => Some(Effective::Int(self.lint.timeout)),
            ["lint", guest, "extend"] => {
                let guest = LangId::from_name(guest)?;
                let set = self.lint.guests.get(&guest).and_then(|g| g.extend);
                Some(Effective::Bool(set.unwrap_or(defaults::LINT_EXTEND)))
            }
            ["parse", "host_errors"] => str_(self.parse.host_errors.as_str()),
            ["threshold", "shell", "allow"] => {
                Some(Effective::List(self.threshold.shell_allow.clone()))
            }
            ["threshold", "exec", "max_args"] => Some(Effective::Int(self.threshold.exec_max_args)),
            ["threshold", "exec", "max_len"] => Some(Effective::Int(self.threshold.exec_max_len)),
            ["threshold", "load", "max_params"] => {
                Some(Effective::Int(self.threshold.load_max_params))
            }
            ["threshold", "load", "param_prefix"] => str_(&self.threshold.load_param_prefix),
            ["threshold", guest, leaf @ ("max_lines" | "max_bytes")] => {
                let guest = guest_threshold_lang(guest)?;
                let set = self.threshold.guests.get(&guest);
                Some(Effective::Int(if *leaf == "max_lines" {
                    set.and_then(|g| g.max_lines)
                        .unwrap_or(defaults::THRESHOLD_GUEST_MAX_LINES)
                } else {
                    set.and_then(|g| g.max_bytes)
                        .unwrap_or(defaults::THRESHOLD_GUEST_MAX_BYTES)
                }))
            }
            _ => None,
        }
    }

    /// Whether `key` came from the file or the table; `None` for a key
    /// the table does not hold.
    #[must_use]
    pub fn source(&self, key: &str) -> Option<Source> {
        if self.effective(key).is_none() && key != "extract.rule.base" {
            return None;
        }
        Some(if self.set.contains(key) {
            Source::File
        } else {
            Source::Default
        })
    }
}

/// A guest id usable in `[threshold.<guest>]`: any language but shell,
/// whose table has its own shape (`src/config` §I).
fn guest_threshold_lang(name: &str) -> Option<LangId> {
    LangId::from_name(name).filter(|id| *id != LangId::Shell)
}

// ---------------------------------------------------------------------
// value readers: each names the key it refuses
// ---------------------------------------------------------------------

fn type_name(value: &Value) -> &'static str {
    match value {
        Value::String(_) => "a string",
        Value::Integer(_) => "an integer",
        Value::Float(_) => "a float",
        Value::Boolean(_) => "a boolean",
        Value::Datetime(_) => "a datetime",
        Value::Array(_) => "an array",
        Value::Table(_) => "a table",
    }
}

fn wrong(key: &str, want: &str, value: &Value) -> ConfigError {
    ConfigError::new(key, format!("expected {want}, found {}", type_name(value)))
}

fn table<'a>(key: &str, value: &'a Value) -> Result<&'a Table, ConfigError> {
    value.as_table().ok_or_else(|| wrong(key, "a table", value))
}

fn string(key: &str, value: &Value) -> Result<String, ConfigError> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| wrong(key, "a string", value))
}

fn boolean(key: &str, value: &Value) -> Result<bool, ConfigError> {
    value
        .as_bool()
        .ok_or_else(|| wrong(key, "a boolean", value))
}

fn count(key: &str, value: &Value) -> Result<u64, ConfigError> {
    let n = value
        .as_integer()
        .ok_or_else(|| wrong(key, "an integer", value))?;
    u64::try_from(n).map_err(|_| ConfigError::new(key, format!("must not be negative, found {n}")))
}

fn strings(key: &str, value: &Value) -> Result<Vec<String>, ConfigError> {
    let items = value
        .as_array()
        .ok_or_else(|| wrong(key, "an array of strings", value))?;
    items
        .iter()
        .enumerate()
        .map(|(i, item)| string(&format!("{key}[{i}]"), item))
        .collect()
}

fn choice(key: &str, value: &Value, choices: &[&'static str]) -> Result<&'static str, ConfigError> {
    let s = value
        .as_str()
        .ok_or_else(|| wrong(key, "a string", value))?;
    choices.iter().copied().find(|c| *c == s).ok_or_else(|| {
        ConfigError::new(key, format!("`{s}` is not one of: {}", choices.join(", ")))
    })
}

fn lang(key: &str, value: &Value) -> Result<LangId, ConfigError> {
    let s = value
        .as_str()
        .ok_or_else(|| wrong(key, "a string", value))?;
    LangId::from_name(s).ok_or_else(|| unknown_lang(key, s))
}

fn unknown_lang(key: &str, name: &str) -> ConfigError {
    let names: Vec<&str> = LangId::ALL.iter().map(|id| id.as_str()).collect();
    ConfigError::new(
        key,
        format!(
            "unknown language `{name}`; expected one of: {}",
            names.join(", ")
        ),
    )
}

fn tables<'a>(key: &str, value: &'a Value) -> Result<Vec<&'a Table>, ConfigError> {
    let items = value
        .as_array()
        .ok_or_else(|| wrong(key, "an array of tables", value))?;
    items
        .iter()
        .enumerate()
        .map(|(i, item)| table(&format!("{key}[{i}]"), item))
        .collect()
}

// ---------------------------------------------------------------------
// the parser
// ---------------------------------------------------------------------

/// Parse one `xenolith.toml`.
///
/// # Errors
///
/// A [`ConfigError`] naming the key when the text is not TOML, the
/// version is missing or unsupported (`src/config:V70`), a key is
/// unknown, a value has the wrong type or falls outside its closed set,
/// or an `[[allow]]` breaks `src/config:V9` or `src/config:V10`.
pub fn parse(text: &str) -> Result<Config, ConfigError> {
    let root: Table = text
        .parse()
        .map_err(|e: toml::de::Error| ConfigError::new("", e.to_string().trim_end().to_owned()))?;

    let supported: Vec<String> = VERSIONS.iter().map(ToString::to_string).collect();
    let supported = supported.join(", ");
    let version = match root.get("version") {
        None => {
            return Err(ConfigError::new(
                "version",
                format!("missing; supported: {supported}"),
            ));
        }
        Some(Value::Integer(v)) if VERSIONS.contains(v) => *v,
        Some(other) => {
            let found = other
                .as_integer()
                .map_or_else(|| type_name(other).to_owned(), |v| v.to_string());
            return Err(ConfigError::new(
                "version",
                format!("unknown version {found}; supported: {supported}"),
            ));
        }
    };

    let mut config = Config {
        version,
        ..Config::default()
    };
    // `Table` is ordered by key, so the FIRST error reported for a file
    // with several is the same on every run (`src:V11`).
    for (key, value) in &root {
        match key.as_str() {
            "extract" => parse_extract(&mut config, table(key, value)?)?,
            "allow" => config.allow = parse_allow(value)?,
            "lint" => parse_lint(&mut config, table(key, value)?)?,
            "langs" => parse_langs(&mut config, table(key, value)?)?,
            "parse" => parse_parse(&mut config, table(key, value)?)?,
            "threshold" => parse_threshold(&mut config, table(key, value)?)?,
            // Schema owned by later tasks (`src/config:T80` exclude,
            // `[[detect]]`, per-verb exclude lists). Accepted rather than
            // rejected, so a valid file does not fail on a task that has
            // not landed; interpreted when that task does.
            // `version` was read above.
            "version" | "exclude" | "detect" | "check" | "graph" => {}
            other => return Err(ConfigError::unknown(other)),
        }
    }
    Ok(config)
}

fn parse_extract(config: &mut Config, t: &Table) -> Result<(), ConfigError> {
    use defaults as d;
    for (leaf, value) in t {
        let key = format!("extract.{leaf}");
        match leaf.as_str() {
            "layout" => {
                config.extract.layout = Layout::from_choice(choice(&key, value, d::LAYOUTS)?);
            }
            "root" => config.extract.root = string(&key, value)?,
            "depth" => {
                let depth = count(&key, value)?;
                if depth == 0 {
                    return Err(ConfigError::new(key, "must be at least 1"));
                }
                config.extract.depth = depth;
            }
            "inactive_rules" => {
                config.extract.inactive_rules =
                    Policy::from_choice(choice(&key, value, d::POLICIES)?);
            }
            "shell" => {
                for (leaf, value) in table(&key, value)? {
                    let key = format!("extract.shell.{leaf}");
                    if leaf != "strict" {
                        return Err(ConfigError::unknown(&key));
                    }
                    config.extract.shell.strict =
                        Strict::from_choice(choice(&key, value, d::STRICTNESS)?);
                    config.set.insert(key);
                }
                continue;
            }
            "rule" => {
                config.extract.rules = tables(&key, value)?
                    .into_iter()
                    .enumerate()
                    .map(|(i, rule)| parse_rule(&format!("extract.rule[{i}]"), rule))
                    .collect::<Result<_, _>>()?;
                continue;
            }
            // `src/config:T80`.
            "exclude" => continue,
            _ => return Err(ConfigError::unknown(&key)),
        }
        config.set.insert(key);
    }
    Ok(())
}

fn parse_rule(at: &str, t: &Table) -> Result<ExtractRule, ConfigError> {
    let mut rule = ExtractRule::default();
    for (leaf, value) in t {
        let key = format!("{at}.{leaf}");
        match leaf.as_str() {
            "host" => rule.host = Some(lang(&key, value)?),
            "sink" => rule.sink = Some(string(&key, value)?),
            "guest" => rule.guest = Some(lang(&key, value)?),
            "path" => rule.path = Some(string(&key, value)?),
            "base" => {
                rule.base = Some(match string(&key, value)?.as_str() {
                    "host" => Base::Host,
                    "root" => Base::Root,
                    dir => Base::Dir(dir.to_owned()),
                });
            }
            "invoke" => rule.invoke = Some(strings(&key, value)?),
            "prelude" => {
                let mut prelude = Prelude {
                    shebang: None,
                    strict: None,
                };
                for (leaf, value) in table(&key, value)? {
                    let key = format!("{key}.{leaf}");
                    match leaf.as_str() {
                        "shebang" => prelude.shebang = Some(string(&key, value)?),
                        "strict" => {
                            prelude.strict = Some(Strict::from_choice(choice(
                                &key,
                                value,
                                defaults::STRICTNESS,
                            )?));
                        }
                        _ => return Err(ConfigError::unknown(&key)),
                    }
                }
                rule.prelude = Some(prelude);
            }
            "executable" => rule.executable = Some(boolean(&key, value)?),
            "companion" => rule.companion = Some(string(&key, value)?),
            _ => return Err(ConfigError::unknown(&key)),
        }
    }
    // A rule with no match key matches every site: that is a layout, and
    // `[extract] layout` already says it (`src/extract` §I).
    if rule.host.is_none() && rule.sink.is_none() && rule.guest.is_none() {
        return Err(ConfigError::new(
            at,
            "a rule needs at least one of host, sink, guest to match on",
        ));
    }
    Ok(rule)
}

/// Keys that would pin an allow to a POSITION (`src/config:V10`).
const POSITIONAL: &[&str] = &[
    "line", "lines", "col", "column", "span", "start", "end", "offset", "byte", "bytes",
];

/// Characters that make a path a glob (`src/config:V9`).
const WILDCARDS: &[char] = &['*', '?', '[', ']'];

fn parse_allow(value: &Value) -> Result<Vec<Allow>, ConfigError> {
    tables("allow", value)?
        .into_iter()
        .enumerate()
        .map(|(i, t)| parse_allow_entry(&format!("allow[{i}]"), t))
        .collect()
}

fn parse_allow_entry(at: &str, t: &Table) -> Result<Allow, ConfigError> {
    for leaf in t.keys() {
        let key = format!("{at}.{leaf}");
        match leaf.as_str() {
            "path" | "sink" | "hash" | "reason" => {}
            positional if POSITIONAL.contains(&positional) => {
                return Err(ConfigError::new(
                    key,
                    "an allow is keyed by path, sink and content hash, never by position: \
                     a line or span breaks on an edit above the site and survives an edit \
                     to the body (src/config:V10)",
                ));
            }
            _ => return Err(ConfigError::unknown(&key)),
        }
    }
    let required = |leaf: &str| -> Result<String, ConfigError> {
        let key = format!("{at}.{leaf}");
        let value = t
            .get(leaf)
            .ok_or_else(|| ConfigError::new(&key, "required"))?;
        let s = string(&key, value)?;
        if s.trim().is_empty() {
            return Err(ConfigError::new(key, "must not be empty"));
        }
        Ok(s)
    };
    let path = required("path")?;
    if path.contains(WILDCARDS) {
        return Err(ConfigError::new(
            format!("{at}.path"),
            "an allow names one file; a wildcard path would cover sites nobody has \
             looked at (src/config:V9)",
        ));
    }
    let sink = required("sink")?;
    let hash = required("hash")?;
    let reason = required("reason").map_err(|e| ConfigError {
        message: format!(
            "{}: every allow carries its reason (src/config:V9)",
            e.message
        ),
        ..e
    })?;
    Ok(Allow {
        path,
        sink,
        hash,
        reason,
    })
}

fn parse_lint(config: &mut Config, t: &Table) -> Result<(), ConfigError> {
    for (leaf, value) in t {
        let key = format!("lint.{leaf}");
        match leaf.as_str() {
            "hosts" => config.lint.hosts = boolean(&key, value)?,
            "all" => config.lint.all = strings(&key, value)?,
            "timeout" => config.lint.timeout = count(&key, value)?,
            // `src/config:T80`.
            "exclude" => continue,
            name => {
                let guest = LangId::from_name(name).ok_or_else(|| unknown_lang(&key, name))?;
                let entry = config.lint.guests.entry(guest).or_default();
                for (leaf, value) in table(&key, value)? {
                    let key = format!("{key}.{leaf}");
                    match leaf.as_str() {
                        "checks" => entry.checks = strings(&key, value)?,
                        "fixers" => entry.fixers = strings(&key, value)?,
                        "extend" => {
                            entry.extend = Some(boolean(&key, value)?);
                            config.set.insert(key);
                        }
                        _ => return Err(ConfigError::unknown(&key)),
                    }
                }
                continue;
            }
        }
        config.set.insert(key);
    }
    Ok(())
}

fn parse_langs(config: &mut Config, t: &Table) -> Result<(), ConfigError> {
    for (leaf, value) in t {
        let key = format!("langs.{leaf}");
        let policy = || choice(&key, value, defaults::POLICIES).map(Policy::from_choice);
        match leaf.as_str() {
            "unclaimed" => config.langs.unclaimed = policy()?,
            "missing_guest" => config.langs.missing_guest = policy()?,
            _ => return Err(ConfigError::unknown(&key)),
        }
        config.set.insert(key);
    }
    Ok(())
}

fn parse_parse(config: &mut Config, t: &Table) -> Result<(), ConfigError> {
    for (leaf, value) in t {
        let key = format!("parse.{leaf}");
        if leaf != "host_errors" {
            return Err(ConfigError::unknown(&key));
        }
        config.parse.host_errors = Policy::from_choice(choice(&key, value, defaults::POLICIES)?);
        config.set.insert(key);
    }
    Ok(())
}

/// `[threshold.shell] allow`: each item one of
/// [`defaults::SHELL_CONSTRUCTS`], exactly (`src/config:V55`). A name
/// outside the set would tolerate nothing while reading as a rule.
fn constructs(key: &str, value: &Value) -> Result<Vec<String>, ConfigError> {
    let items = value
        .as_array()
        .ok_or_else(|| wrong(key, "an array of strings", value))?;
    items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let key = format!("{key}[{i}]");
            let name = string(&key, item)?;
            if defaults::SHELL_CONSTRUCTS.contains(&name.as_str()) {
                Ok(name)
            } else {
                Err(ConfigError::new(
                    key,
                    format!(
                        "`{name}` is not a shell construct; expected one of: {} \
                         (src/config:V55)",
                        defaults::SHELL_CONSTRUCTS.join(", ")
                    ),
                ))
            }
        })
        .collect()
}

/// `[threshold.*]`, validated per `src/config:V55`: an unknown guest,
/// construct or key, or a negative value, is refused. A threshold only
/// RELAXES; nothing here can make a guest-trivial body flagged, because
/// every value is a tolerance, never a requirement.
fn parse_threshold(config: &mut Config, t: &Table) -> Result<(), ConfigError> {
    let th = &mut config.threshold;
    for (section, value) in t {
        let at = format!("threshold.{section}");
        let fields = table(&at, value)?;
        for (leaf, value) in fields {
            let key = format!("{at}.{leaf}");
            match (section.as_str(), leaf.as_str()) {
                ("shell", "allow") => th.shell_allow = constructs(&key, value)?,
                ("exec", "max_args") => th.exec_max_args = count(&key, value)?,
                ("exec", "max_len") => th.exec_max_len = count(&key, value)?,
                ("load", "max_params") => th.load_max_params = count(&key, value)?,
                ("load", "param_prefix") => th.load_param_prefix = string(&key, value)?,
                ("shell" | "exec" | "load", _) => return Err(ConfigError::unknown(&key)),
                (name, field @ ("max_lines" | "max_bytes")) => {
                    let guest =
                        guest_threshold_lang(name).ok_or_else(|| unknown_lang(&at, name))?;
                    let n = count(&key, value)?;
                    let entry = th.guests.entry(guest).or_default();
                    if field == "max_lines" {
                        entry.max_lines = Some(n);
                    } else {
                        entry.max_bytes = Some(n);
                    }
                }
                (name, _) => {
                    return Err(if guest_threshold_lang(name).is_some() {
                        ConfigError::unknown(&key)
                    } else {
                        unknown_lang(&at, name)
                    });
                }
            }
            config.set.insert(key);
        }
    }
    Ok(())
}
