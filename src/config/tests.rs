//! `xenolith.toml` parsing: the mirror of `src/config/mod.rs`
//! (`src:C139`).
//!
//! Two concerns, tested separately because they fail separately:
//!
//! * the PARSER (`src/config:T10`): what a file may say, and the refusal
//!   when it says something else. A config the tool half-understands is
//!   worse than one it rejects -- the half it skipped is a rule the user
//!   believes is in force.
//! * RESOLUTION against the defaults table (`src/config:V73`): every row
//!   is what an empty config resolves to, and every row can be
//!   overridden. Tested by walking the table rather than by listing keys
//!   here, so a row added later is covered the day it lands. The table's
//!   own shape is `src/config/defaults/tests.rs`'.

use xenolith_lang_api::LangId;

use super::defaults::{Setting, TABLE};
use super::{Base, ConfigError, Effective, Layout, Policy, Source, Strict, parse};

fn ok(text: &str) -> super::Config {
    match parse(text) {
        Ok(config) => config,
        Err(err) => panic!("expected a config, got: {err}"),
    }
}

fn nth<T>(items: &[T], i: usize) -> &T {
    items
        .get(i)
        .unwrap_or_else(|| panic!("expected an item at {i}, found {}", items.len()))
}

fn err(text: &str) -> ConfigError {
    match parse(text) {
        Ok(config) => panic!("expected a refusal, got: {config:?}"),
        Err(err) => err,
    }
}

// ---------------------------------------------------------------------
// version (`src/config` §I, `src/config:V70`)
// ---------------------------------------------------------------------

#[test]
fn a_file_without_version_is_refused_naming_the_supported_one() {
    // `version` is required rather than defaulted: a file written for a
    // future schema and read as v1 would be half-understood in silence.
    let e = err("[langs]\nunclaimed = \"warn\"\n");
    assert_eq!(e.key, "version");
    assert!(e.message.contains('1'), "{e}");
}

#[test]
fn an_unknown_version_is_refused_naming_the_supported_one() {
    let e = err("version = 2\n");
    assert_eq!(e.key, "version");
    assert!(e.message.contains('2'), "{e}");
    assert!(e.message.contains("supported: 1"), "{e}");
}

#[test]
fn a_file_that_is_not_toml_is_refused() {
    let e = err("version = = 1\n");
    assert!(!e.message.is_empty());
}

#[test]
fn a_minimal_file_is_the_defaults() {
    // Convention over configuration (`src/config:V88`): `version = 1`
    // alone means every default, and a missing file means the same.
    let config = ok("version = 1\n");
    assert_eq!(config, super::Config::default());
    for entry in TABLE {
        let key = concrete(entry.key);
        assert_eq!(config.source(&key), Some(Source::Default), "{key}");
    }
}

// ---------------------------------------------------------------------
// shape: unknown keys and wrong types are errors, never ignored
// ---------------------------------------------------------------------

#[test]
fn an_unknown_top_level_key_is_refused() {
    let e = err("version = 1\n[lints]\nhosts = false\n");
    assert_eq!(e.key, "lints");
}

#[test]
fn an_unknown_key_inside_a_known_table_is_refused() {
    let e = err("version = 1\n[extract]\nlayuot = \"mirror\"\n");
    assert_eq!(e.key, "extract.layuot");
}

#[test]
fn a_wrong_type_is_refused_naming_the_key() {
    let e = err("version = 1\n[extract]\ndepth = \"five\"\n");
    assert_eq!(e.key, "extract.depth");
}

#[test]
fn sections_owned_by_later_tasks_are_accepted_not_rejected() {
    // `[[exclude]]` (`src/config:T80`), `[[detect]]` and the per-verb
    // exclude lists are schema (`src/config` §I). Rejecting them here
    // would make a valid file fail on a task that has not landed.
    ok("version = 1\n\
        [[exclude]]\nglob = \"vendor/**\"\nreason = \"third party\"\n\
        [[detect]]\nguest = \"shell\"\npath = \"ci/*.nix\"\n\
        [check]\nexclude = []\n[graph]\nexclude = []\n");
}

// ---------------------------------------------------------------------
// [extract] and [[extract.rule]] (`src/extract` §I)
// ---------------------------------------------------------------------

#[test]
fn extract_layout_and_rules_parse() {
    let config = ok(r##"
version = 1

[extract]
layout = "mirror"
root = "tools"
depth = 3
inactive_rules = "error"

[extract.shell]
strict = "enforce"

[[extract.rule]]
host = "nix"
sink = "systemd.services.*.script"
guest = "shell"
path = "{host_dir}/{name}.{ext}"
base = "root"
invoke = ["bash", "{path}"]
prelude = { shebang = "#!/usr/bin/env bash", strict = "enforce" }
executable = true
companion = "tests/{path_stem}.bats"

[[extract.rule]]
guest = "python"
base = "ci/scripts"
"##);
    assert_eq!(config.extract.layout, Layout::Mirror);
    assert_eq!(config.extract.root, "tools");
    assert_eq!(config.extract.depth, 3);
    assert_eq!(config.extract.inactive_rules, Policy::Error);
    assert_eq!(config.extract.shell.strict, Strict::Enforce);

    assert_eq!(config.extract.rules.len(), 2);
    let first = nth(&config.extract.rules, 0);
    assert_eq!(first.host, Some(LangId::Nix));
    assert_eq!(first.sink.as_deref(), Some("systemd.services.*.script"));
    assert_eq!(first.guest, Some(LangId::Shell));
    assert_eq!(first.path.as_deref(), Some("{host_dir}/{name}.{ext}"));
    assert_eq!(first.base, Some(Base::Root));
    assert_eq!(
        first.invoke.as_deref(),
        Some(&["bash".to_owned(), "{path}".to_owned()][..])
    );
    let prelude = first
        .prelude
        .as_ref()
        .map(|p| (p.shebang.as_deref(), p.strict));
    assert_eq!(
        prelude,
        Some((Some("#!/usr/bin/env bash"), Some(Strict::Enforce)))
    );
    assert_eq!(first.executable, Some(true));
    assert_eq!(first.companion.as_deref(), Some("tests/{path_stem}.bats"));

    let second = nth(&config.extract.rules, 1);
    assert_eq!(second.host, None);
    assert_eq!(second.guest, Some(LangId::Python));
    assert_eq!(second.base, Some(Base::Dir("ci/scripts".to_owned())));
}

#[test]
fn a_rule_matching_on_nothing_is_refused() {
    // ≥1 of host, sink, guest (`src/extract` §I): a rule with no match
    // keys matches every site, which is a layout, not a rule.
    let e = err("version = 1\n[[extract.rule]]\npath = \"x/{name}.{ext}\"\n");
    assert_eq!(e.key, "extract.rule[0]");
}

#[test]
fn a_rule_naming_an_unknown_language_is_refused() {
    // `bash` is a dialect of `shell`, not a language id. Accepting the
    // near-miss would make a rule that silently never matches.
    let e = err("version = 1\n[[extract.rule]]\nguest = \"bash\"\n");
    assert_eq!(e.key, "extract.rule[0].guest");
}

#[test]
fn a_layout_outside_its_choices_is_refused_listing_them() {
    let e = err("version = 1\n[extract]\nlayout = \"flat\"\n");
    assert_eq!(e.key, "extract.layout");
    for choice in ["host", "mirror", "sibling", "central"] {
        assert!(e.message.contains(choice), "{e}");
    }
}

#[test]
fn a_depth_below_one_is_refused() {
    let e = err("version = 1\n[extract]\ndepth = 0\n");
    assert_eq!(e.key, "extract.depth");
}

// ---------------------------------------------------------------------
// [[allow]] (`src/config:V9`, `src/config:V10`)
// ---------------------------------------------------------------------

const ALLOW: &str = r#"
version = 1

[[allow]]
path = "hosts/web/default.nix"
sink = "systemd.services.foo.script"
hash = "3f2a9c"
reason = "upstream module expects an inline script; tracked in #12"
"#;

#[test]
fn an_allow_entry_is_keyed_by_path_sink_and_hash() {
    let config = ok(ALLOW);
    assert_eq!(config.allow.len(), 1);
    let allow = nth(&config.allow, 0);
    assert_eq!(allow.path, "hosts/web/default.nix");
    assert_eq!(allow.sink, "systemd.services.foo.script");
    assert_eq!(allow.hash, "3f2a9c");
    assert!(allow.reason.starts_with("upstream module"));
}

#[test]
fn an_allow_entry_without_a_reason_is_refused() {
    let text = ALLOW.replace(
        "reason = \"upstream module expects an inline script; tracked in #12\"\n",
        "",
    );
    let e = err(&text);
    assert_eq!(e.key, "allow[0].reason");
}

#[test]
fn an_allow_entry_with_a_blank_reason_is_refused() {
    // A reason of spaces is the same as none, and is what a reviewer
    // waves through when the field is merely present.
    let text = ALLOW.replace(
        "upstream module expects an inline script; tracked in #12",
        "   ",
    );
    let e = err(&text);
    assert_eq!(e.key, "allow[0].reason");
}

#[test]
fn an_allow_entry_missing_any_key_part_is_refused() {
    for part in ["path", "sink", "hash"] {
        let prefix = format!("{part} =");
        let kept: Vec<&str> = ALLOW
            .lines()
            .filter(|line| !line.starts_with(&prefix))
            .collect();
        let text = kept.join("\n");
        let e = err(&text);
        assert_eq!(e.key, format!("allow[0].{part}"));
    }
}

#[test]
fn an_allow_entry_keyed_by_position_is_refused() {
    // `src/config:V10`: a line number or byte span breaks the moment
    // anything above the site is edited, and survives an edit to the body
    // itself -- exactly backwards.
    for key in ["line = 12", "span = [10, 40]", "col = 3"] {
        let text = format!("{ALLOW}{key}\n");
        let e = err(&text);
        assert!(e.key.starts_with("allow[0]."), "{e}");
        assert!(e.message.contains("V10"), "{e}");
    }
}

#[test]
fn an_allow_entry_with_a_wildcard_path_is_refused() {
    // `src/config:V9`: ⊥ wildcard path allow. A glob allow is a silent
    // blanket over sites nobody has looked at.
    for path in [
        "hosts/*.nix",
        "hosts/**/default.nix",
        "hosts/web/default.ni?",
        "hosts/[ab].nix",
    ] {
        let text = ALLOW.replace("hosts/web/default.nix", path);
        let e = err(&text);
        assert_eq!(e.key, "allow[0].path", "{path}");
    }
}

// ---------------------------------------------------------------------
// [lint] and [lint.<guest>] (`src/lint` §I)
// ---------------------------------------------------------------------

#[test]
fn the_lint_map_parses() {
    let config = ok(r#"
version = 1

[lint]
hosts = false
all = ["typos {file}"]
timeout = 30

[lint.shell]
checks = ["shellcheck -x {file}"]
fixers = ["shfmt -w {file}"]
extend = false

[lint.python]
checks = ["ruff check {file}"]
"#);
    assert!(!config.lint.hosts);
    assert_eq!(config.lint.all, vec!["typos {file}".to_owned()]);
    assert_eq!(config.lint.timeout, 30);

    let shell = config.lint.guests.get(&LangId::Shell);
    assert_eq!(
        shell.map(|g| (g.checks.clone(), g.fixers.clone(), g.extend)),
        Some((
            vec!["shellcheck -x {file}".to_owned()],
            vec!["shfmt -w {file}".to_owned()],
            Some(false)
        ))
    );
    // `extend` unset stays unset here; the default is resolved through
    // the table, not copied into every entry.
    let python = config.lint.guests.get(&LangId::Python);
    assert_eq!(python.and_then(|g| g.extend), None);
    assert_eq!(
        config.effective("lint.python.extend"),
        Some(Effective::Bool(true))
    );
    assert_eq!(
        config.effective("lint.shell.extend"),
        Some(Effective::Bool(false))
    );
}

#[test]
fn a_lint_table_for_an_unknown_guest_is_refused() {
    let e = err("version = 1\n[lint.bash]\nchecks = []\n");
    assert_eq!(e.key, "lint.bash");
}

// ---------------------------------------------------------------------
// [langs] (`src:C1`, `src:V13`, `src:V42`)
// ---------------------------------------------------------------------

#[test]
fn the_langs_toggles_parse() {
    let config = ok("version = 1\n[langs]\nunclaimed = \"error\"\nmissing_guest = \"warn\"\n");
    assert_eq!(config.langs.unclaimed, Policy::Error);
    assert_eq!(config.langs.missing_guest, Policy::Warn);
}

#[test]
fn a_langs_value_outside_its_choices_is_refused_listing_them() {
    let e = err("version = 1\n[langs]\nunclaimed = \"fatal\"\n");
    assert_eq!(e.key, "langs.unclaimed");
    for choice in ["ignore", "warn", "error"] {
        assert!(e.message.contains(choice), "{e}");
    }
}

// ---------------------------------------------------------------------
// resolution against the defaults table (`src/config:V73`)
// ---------------------------------------------------------------------

/// A table key with its `<guest>` placeholder filled in. `nix` for
/// thresholds because `[threshold.shell]` has its own shape
/// (`src/config` §I: `allow`, not `max_lines`); `shell` for lint.
fn concrete(key: &str) -> String {
    if key.starts_with("threshold.") {
        key.replace("<guest>", "nix")
    } else {
        key.replace("<guest>", "shell")
    }
}

fn expected(setting: &Setting) -> Option<Effective> {
    match setting {
        Setting::Int(n) => Some(Effective::Int(*n)),
        Setting::Bool(b) => Some(Effective::Bool(*b)),
        Setting::Str(s) => Some(Effective::Str((*s).to_owned())),
        Setting::List(items) => Some(Effective::List(
            items.iter().map(|s| (*s).to_owned()).collect(),
        )),
        // Resolved per site from the host, so there is no single
        // effective value to report here.
        Setting::Derived(_) => None,
    }
}

#[test]
fn every_default_is_what_an_empty_config_resolves_to() {
    let config = super::Config::default();
    for entry in TABLE {
        let key = concrete(entry.key);
        assert_eq!(config.effective(&key), expected(&entry.value), "{key}");
    }
}

/// A value for `entry` different from its default and valid for its key.
fn override_for(entry: &super::defaults::Entry) -> Option<(String, Effective)> {
    match &entry.value {
        Setting::Int(n) => Some(((n + 1).to_string(), Effective::Int(n + 1))),
        Setting::Bool(b) => Some(((!b).to_string(), Effective::Bool(!b))),
        Setting::List(_) => Some((
            "[\"pipe\"]".to_owned(),
            Effective::List(vec!["pipe".to_owned()]),
        )),
        Setting::Str(default) => {
            let value = entry
                .choices
                .iter()
                .copied()
                .find(|c| c != default)
                .map_or_else(|| format!("{default}override"), str::to_owned);
            Some((format!("\"{value}\""), Effective::Str(value)))
        }
        Setting::Derived(_) => None,
    }
}

/// `[a.b]\nc = v` for key `a.b.c`.
fn toml_setting(key: &str, value: &str) -> String {
    let (table, leaf) = key.rsplit_once('.').unwrap_or(("", key));
    format!("version = 1\n[{table}]\n{leaf} = {value}\n")
}

#[test]
fn every_default_is_parsed_and_overridable() {
    // `src/config:V73`: ∀ default ∃ config key. Proven by writing each
    // key with a non-default value and reading the override back, with
    // its source, rather than by trusting the parser's match arms.
    for entry in TABLE {
        let key = concrete(entry.key);
        let Some((literal, want)) = override_for(entry) else {
            continue;
        };
        let config = ok(&toml_setting(&key, &literal));
        assert_eq!(config.effective(&key), Some(want), "{key}");
        assert_eq!(config.source(&key), Some(Source::File), "{key}");
    }
}

#[test]
fn the_derived_rule_base_is_overridable_per_rule() {
    // `extract.rule.base` defaults to the host's `runtime_base`, which
    // no table can hold as a literal; the override is per rule.
    let entry = TABLE.iter().find(|e| e.key == "extract.rule.base");
    assert!(matches!(entry.map(|e| &e.value), Some(Setting::Derived(_))));
    let config = ok("version = 1\n[[extract.rule]]\nhost = \"nix\"\nbase = \"host\"\n");
    assert_eq!(nth(&config.extract.rules, 0).base, Some(Base::Host));
}

#[test]
fn an_unknown_key_has_no_effective_value_or_source() {
    let config = super::Config::default();
    assert_eq!(config.effective("extract.dpeth"), None);
    assert_eq!(config.source("extract.dpeth"), None);
}
