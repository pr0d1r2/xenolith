//! The defaults table: every value an engine acts on without being told
//! (`src/config:V73`, `src/config:T73`).
//!
//! ONE module, and nothing else in the crate spells a default. An engine
//! reads the resolved [`super::Config`]; the parser fills that config
//! from the constants below; and [`TABLE`] lists the same constants with
//! their keys, so `--verbose` and the generated reference
//! (`src/config:V85`) describe exactly what the parser applies rather
//! than a second copy that drifts from it.
//!
//! The named constants exist for the parser's sake: a typed field is
//! defaulted from `EXTRACT_DEPTH`, not from a lookup in the table by
//! string, so a renamed key is a compile error rather than a silent
//! fallback.

#[cfg(test)]
mod tests;

/// `[extract] layout` (`src/extract` §I): host placement only.
pub const EXTRACT_LAYOUT: &str = "host";
/// `[extract] root` (`src/extract` §I): where `mirror` and `central`
/// place extracts.
pub const EXTRACT_ROOT: &str = "scripts";
/// `[extract] depth` (`src/extract:V65`): nesting levels per run.
pub const EXTRACT_DEPTH: u64 = 5;
/// `[extract] inactive_rules` (`src/config:V44`).
pub const EXTRACT_INACTIVE_RULES: &str = "warn";
/// `[extract.shell] strict` (`languages/shell` §I): reproduce the
/// options the site ran under, add none.
pub const EXTRACT_SHELL_STRICT: &str = "preserve";
/// `[langs] missing_guest` (`src:V42`): a site whose guest is compiled
/// out is an error, never a guess.
pub const LANGS_MISSING_GUEST: &str = "error";
/// `[langs] unclaimed` (`src:V13`): a file no host claims is not
/// scanned and not reported.
pub const LANGS_UNCLAIMED: &str = "ignore";
/// `[lint.<guest>] extend` (`src/lint` §I): configured checks append to
/// the guest's defaults rather than replace them.
pub const LINT_EXTEND: bool = true;
/// `[lint] hosts` (`src/lint` §I): host files get `Host::checks`.
pub const LINT_HOSTS: bool = true;
/// `[lint] timeout` in seconds (`src/lint:V126`); 0 = no limit.
pub const LINT_TIMEOUT: u64 = 60;
/// `[parse] host_errors` (`src/config` §I): a claimed file its host
/// cannot parse was not checked, and that is a finding, not a pass.
pub const PARSE_HOST_ERRORS: &str = "error";
/// `[threshold.<guest>] max_bytes` (`src/config:V55`).
pub const THRESHOLD_GUEST_MAX_BYTES: u64 = 80;
/// `[threshold.<guest>] max_lines` (`src/config:V55`).
pub const THRESHOLD_GUEST_MAX_LINES: u64 = 1;
/// `[threshold.exec] max_args` (`src/config` §I).
pub const THRESHOLD_EXEC_MAX_ARGS: u64 = 8;
/// `[threshold.exec] max_len` (`src/config` §I).
pub const THRESHOLD_EXEC_MAX_LEN: u64 = 120;
/// `[threshold.load] max_params` (`languages/api/src/holes:V40`).
pub const THRESHOLD_LOAD_MAX_PARAMS: u64 = 6;
/// `[threshold.load] param_prefix` (`src/config` §I).
pub const THRESHOLD_LOAD_PARAM_PREFIX: &str = "";
/// `[threshold.shell] allow` (`languages/shell:V3`): no construct is
/// tolerated inline until a config says so.
pub const THRESHOLD_SHELL_ALLOW: &[&str] = &[];

/// The three-way policy used by every `ignore | warn | error` key.
pub const POLICIES: &[&str] = &["ignore", "warn", "error"];
/// `[extract] layout` choices (`src/extract` §I).
pub const LAYOUTS: &[&str] = &["host", "mirror", "sibling", "central"];
/// `[extract.shell] strict` choices (`languages/shell` §I).
pub const STRICTNESS: &[&str] = &["preserve", "enforce"];
/// `[threshold.shell] allow` choices (`src/config` §I, `src/config:V55`):
/// the constructs `languages/shell:V3` flags, spelled exactly as the
/// shell classifier's `Construct::as_str` emits them (`src/config:B1`:
/// one vocabulary, or a threshold can never match). Here rather than in
/// the shell crate so the config validates the same with `lang-shell`
/// compiled out (`src:V30`).
pub const SHELL_CONSTRUCTS: &[&str] = &[
    "and-or",
    "case",
    "command-substitution",
    "for",
    "function-definition",
    "heredoc",
    "if",
    "pipeline",
    "redirect",
    "sequence",
    "subshell",
    "while",
];

/// A default's value, as the table states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    /// A non-negative count or duration.
    Int(u64),
    /// A string: a policy, a path, a prefix.
    Str(&'static str),
    /// A switch.
    Bool(bool),
    /// A list of strings.
    List(&'static [&'static str]),
    /// Not a literal: resolved per site from the named source. Listed so
    /// the key is still discoverable (`src/config:V73`).
    Derived(&'static str),
}

/// One row: a key, its default, and -- for a closed set -- the values
/// the key accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Dotted key. `<guest>` stands for any guest language id.
    pub key: &'static str,
    /// The default.
    pub value: Setting,
    /// The accepted values when the key is a closed set; empty = free.
    pub choices: &'static [&'static str],
}

const fn row(key: &'static str, value: Setting) -> Entry {
    Entry {
        key,
        value,
        choices: &[],
    }
}

const fn choice(key: &'static str, value: &'static str, choices: &'static [&'static str]) -> Entry {
    Entry {
        key,
        value: Setting::Str(value),
        choices,
    }
}

/// Every default, sorted by key (`src/config:V73`).
///
/// Sorted so `--verbose` and the generated reference have one order
/// without a sorting pass anybody could forget; a test holds it sorted.
pub const TABLE: &[Entry] = &[
    row("extract.depth", Setting::Int(EXTRACT_DEPTH)),
    choice("extract.inactive_rules", EXTRACT_INACTIVE_RULES, POLICIES),
    choice("extract.layout", EXTRACT_LAYOUT, LAYOUTS),
    row("extract.root", Setting::Str(EXTRACT_ROOT)),
    row("extract.rule.base", Setting::Derived("Host::runtime_base")),
    choice("extract.shell.strict", EXTRACT_SHELL_STRICT, STRICTNESS),
    choice("langs.missing_guest", LANGS_MISSING_GUEST, POLICIES),
    choice("langs.unclaimed", LANGS_UNCLAIMED, POLICIES),
    row("lint.<guest>.extend", Setting::Bool(LINT_EXTEND)),
    row("lint.hosts", Setting::Bool(LINT_HOSTS)),
    row("lint.timeout", Setting::Int(LINT_TIMEOUT)),
    choice("parse.host_errors", PARSE_HOST_ERRORS, POLICIES),
    row(
        "threshold.<guest>.max_bytes",
        Setting::Int(THRESHOLD_GUEST_MAX_BYTES),
    ),
    row(
        "threshold.<guest>.max_lines",
        Setting::Int(THRESHOLD_GUEST_MAX_LINES),
    ),
    row(
        "threshold.exec.max_args",
        Setting::Int(THRESHOLD_EXEC_MAX_ARGS),
    ),
    row(
        "threshold.exec.max_len",
        Setting::Int(THRESHOLD_EXEC_MAX_LEN),
    ),
    row(
        "threshold.load.max_params",
        Setting::Int(THRESHOLD_LOAD_MAX_PARAMS),
    ),
    row(
        "threshold.load.param_prefix",
        Setting::Str(THRESHOLD_LOAD_PARAM_PREFIX),
    ),
    row(
        "threshold.shell.allow",
        Setting::List(THRESHOLD_SHELL_ALLOW),
    ),
];
