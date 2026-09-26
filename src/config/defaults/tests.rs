//! Unit tests for the defaults table (`src:C139`, `src/config:T73`,
//! `src/config:V73`).
//!
//! The table itself, not the parser that reads it: that its rows are
//! sorted and complete, that each row states the constant the parser
//! uses, that a closed-set row's default is one of its own choices, and
//! that nothing outside `src/config` spells a default. Whether the parser
//! resolves and overrides every row is `src/config/tests.rs`' question.

use std::fs;
use std::path::{Path, PathBuf};

use super::{
    EXTRACT_DEPTH, EXTRACT_INACTIVE_RULES, EXTRACT_LAYOUT, EXTRACT_ROOT, EXTRACT_SHELL_STRICT,
    Entry, LANGS_MISSING_GUEST, LANGS_UNCLAIMED, LAYOUTS, LINT_EXTEND, LINT_HOSTS, LINT_TIMEOUT,
    PARSE_HOST_ERRORS, POLICIES, STRICTNESS, Setting, TABLE, THRESHOLD_EXEC_MAX_ARGS,
    THRESHOLD_EXEC_MAX_LEN, THRESHOLD_GUEST_MAX_BYTES, THRESHOLD_GUEST_MAX_LINES,
    THRESHOLD_LOAD_MAX_PARAMS, THRESHOLD_LOAD_PARAM_PREFIX, THRESHOLD_SHELL_ALLOW, choice, row,
};

fn entry(key: &str) -> Entry {
    TABLE
        .iter()
        .copied()
        .find(|e| e.key == key)
        .unwrap_or_else(|| panic!("no row for {key}"))
}

// ---------------------------------------------------------------------
// row / choice
// ---------------------------------------------------------------------

#[test]
fn row_is_a_free_value_with_no_choices() {
    let built = row("a.b", Setting::Int(3));
    assert_eq!(
        built,
        Entry {
            key: "a.b",
            value: Setting::Int(3),
            choices: &[],
        }
    );
}

#[test]
fn choice_is_a_string_default_with_its_closed_set() {
    let built = choice("a.b", "warn", POLICIES);
    assert_eq!(built.key, "a.b");
    assert_eq!(built.value, Setting::Str("warn"));
    assert_eq!(built.choices, POLICIES);
}

// ---------------------------------------------------------------------
// the table's shape
// ---------------------------------------------------------------------

#[test]
fn the_table_keys_are_unique_and_sorted() {
    // Sorted so the generated reference (`src/config:V85`) and
    // `--verbose` have one order without a sort anyone could forget.
    let keys: Vec<&str> = TABLE.iter().map(|e| e.key).collect();
    let mut sorted = keys.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(keys, sorted);
}

#[test]
fn the_table_holds_every_default_the_spec_names() {
    // `src/config` §I's defaults list, restated as keys. The ONE place
    // that lists them: it guards against a row being dropped, while the
    // other tests walk the table itself.
    let spec = [
        "extract.depth",
        "extract.inactive_rules",
        "extract.layout",
        "extract.root",
        "extract.rule.base",
        "extract.shell.strict",
        "langs.missing_guest",
        "langs.unclaimed",
        "lint.<guest>.extend",
        "lint.hosts",
        "lint.timeout",
        "parse.host_errors",
        "threshold.<guest>.max_bytes",
        "threshold.<guest>.max_lines",
        "threshold.exec.max_args",
        "threshold.exec.max_len",
        "threshold.load.max_params",
        "threshold.load.param_prefix",
        "threshold.shell.allow",
    ];
    let keys: Vec<&str> = TABLE.iter().map(|e| e.key).collect();
    assert_eq!(keys, spec);
}

#[test]
fn every_key_is_dotted_lowercase() {
    for e in TABLE {
        assert!(e.key.contains('.'), "{} names no table", e.key);
        assert!(
            e.key.split('.').all(|part| !part.is_empty()
                && (part == "<guest>"
                    || part.bytes().all(|b| b.is_ascii_lowercase() || b == b'_'))),
            "{}",
            e.key
        );
    }
}

#[test]
fn a_closed_set_default_is_one_of_its_own_choices() {
    // A default the parser would refuse if a user wrote it is a table
    // contradicting itself.
    for e in TABLE.iter().filter(|e| !e.choices.is_empty()) {
        let Setting::Str(default) = e.value else {
            panic!("{}: a closed set with a non-string default", e.key);
        };
        assert!(e.choices.contains(&default), "{}: {default}", e.key);
    }
}

#[test]
fn only_string_rows_carry_choices() {
    for e in TABLE {
        if !matches!(e.value, Setting::Str(_)) {
            assert!(e.choices.is_empty(), "{}", e.key);
        }
    }
}

#[test]
fn the_only_derived_row_is_the_rule_base() {
    // A derived default has no literal to show; every other row must.
    let derived: Vec<&str> = TABLE
        .iter()
        .filter(|e| matches!(e.value, Setting::Derived(_)))
        .map(|e| e.key)
        .collect();
    assert_eq!(derived, vec!["extract.rule.base"]);
    assert_eq!(
        entry("extract.rule.base").value,
        Setting::Derived("Host::runtime_base")
    );
}

#[test]
fn every_closed_set_is_distinct() {
    for set in [POLICIES, LAYOUTS, STRICTNESS] {
        let mut sorted = set.to_vec();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), set.len(), "{set:?}");
        assert!(set.iter().all(|c| !c.is_empty()), "{set:?}");
    }
}

// ---------------------------------------------------------------------
// each row states the constant the parser uses
// ---------------------------------------------------------------------

#[test]
fn each_row_holds_its_named_constant() {
    // The parser defaults typed fields from the constants and `--verbose`
    // reads the table: a row with its own literal would drift from what
    // the parser applies.
    let expected = [
        ("extract.depth", Setting::Int(EXTRACT_DEPTH)),
        (
            "extract.inactive_rules",
            Setting::Str(EXTRACT_INACTIVE_RULES),
        ),
        ("extract.layout", Setting::Str(EXTRACT_LAYOUT)),
        ("extract.root", Setting::Str(EXTRACT_ROOT)),
        ("extract.shell.strict", Setting::Str(EXTRACT_SHELL_STRICT)),
        ("langs.missing_guest", Setting::Str(LANGS_MISSING_GUEST)),
        ("langs.unclaimed", Setting::Str(LANGS_UNCLAIMED)),
        ("lint.<guest>.extend", Setting::Bool(LINT_EXTEND)),
        ("lint.hosts", Setting::Bool(LINT_HOSTS)),
        ("lint.timeout", Setting::Int(LINT_TIMEOUT)),
        ("parse.host_errors", Setting::Str(PARSE_HOST_ERRORS)),
        (
            "threshold.<guest>.max_bytes",
            Setting::Int(THRESHOLD_GUEST_MAX_BYTES),
        ),
        (
            "threshold.<guest>.max_lines",
            Setting::Int(THRESHOLD_GUEST_MAX_LINES),
        ),
        (
            "threshold.exec.max_args",
            Setting::Int(THRESHOLD_EXEC_MAX_ARGS),
        ),
        (
            "threshold.exec.max_len",
            Setting::Int(THRESHOLD_EXEC_MAX_LEN),
        ),
        (
            "threshold.load.max_params",
            Setting::Int(THRESHOLD_LOAD_MAX_PARAMS),
        ),
        (
            "threshold.load.param_prefix",
            Setting::Str(THRESHOLD_LOAD_PARAM_PREFIX),
        ),
        (
            "threshold.shell.allow",
            Setting::List(THRESHOLD_SHELL_ALLOW),
        ),
    ];
    for (key, value) in expected {
        assert_eq!(entry(key).value, value, "{key}");
    }
}

#[test]
fn the_defaults_are_the_values_the_spec_states() {
    // `src/config` §I, value by value: a constant edited without the spec
    // is a changed default nobody announced.
    assert_eq!(EXTRACT_LAYOUT, "host");
    assert_eq!(EXTRACT_ROOT, "scripts");
    assert_eq!(EXTRACT_DEPTH, 5);
    assert_eq!(EXTRACT_INACTIVE_RULES, "warn");
    assert_eq!(EXTRACT_SHELL_STRICT, "preserve");
    assert_eq!(LANGS_MISSING_GUEST, "error");
    assert_eq!(LANGS_UNCLAIMED, "ignore");
    assert_eq!((LINT_EXTEND, LINT_HOSTS), (true, true));
    assert_eq!(PARSE_HOST_ERRORS, "error");
    assert_eq!(THRESHOLD_GUEST_MAX_BYTES, 80);
    assert_eq!(THRESHOLD_GUEST_MAX_LINES, 1);
    assert_eq!(THRESHOLD_EXEC_MAX_ARGS, 8);
    assert_eq!(THRESHOLD_EXEC_MAX_LEN, 120);
    assert_eq!(THRESHOLD_LOAD_MAX_PARAMS, 6);
    assert_eq!(THRESHOLD_LOAD_PARAM_PREFIX, "");
    assert!(THRESHOLD_SHELL_ALLOW.is_empty());
}

// ---------------------------------------------------------------------
// nothing else spells a default (`src/config:V73`)
// ---------------------------------------------------------------------

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn engines_hold_no_literal_defaults() {
    // `src/config:V73`: engines read the resolved config only. The
    // checkable proxy: an engine that names a config key as a STRING is
    // reading raw TOML, and raw TOML is the only place a fallback literal
    // can hide -- a typed field on `Config` has already been defaulted.
    // So no `.rs` outside `src/config` may spell a table key's leaf as a
    // string literal. An engine that needs the name (for a message) takes
    // it from `defaults::TABLE`.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let config_dir = src.join("config");
    let mut offences = Vec::new();
    for file in files.iter().filter(|f| !f.starts_with(&config_dir)) {
        let Ok(text) = fs::read_to_string(file) else {
            continue;
        };
        for entry in TABLE {
            let leaf = entry.key.rsplit('.').next().unwrap_or(entry.key);
            if text.contains(&format!("\"{leaf}\"")) {
                offences.push(format!("{}: \"{leaf}\"", file.display()));
            }
        }
    }
    assert!(
        offences.is_empty(),
        "literal config keys in engines: {offences:?}"
    );
}

#[test]
fn the_literal_guard_walks_nested_modules() {
    // The guard above is only as good as its walk: a module under a
    // subdirectory (`src/model/tests.rs`) must be found, or an engine in
    // one would escape it.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.contains(&src.join("lib.rs")));
    assert!(files.contains(&src.join("model").join("tests.rs")));
    assert!(files.contains(&src.join("config").join("defaults.rs")));
}

#[test]
fn the_literal_guard_skips_a_missing_directory() {
    let mut files = Vec::new();
    rust_files(Path::new("/nonexistent/xenolith/src"), &mut files);
    assert!(files.is_empty());
}
