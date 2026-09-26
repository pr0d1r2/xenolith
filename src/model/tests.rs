//! Unit tests for `model` (`src:C139`): each function's own branches,
//! including the private `sort_key` and `to_value` the report is built
//! from.
//!
//! `tests/model.rs` checks the envelope a consumer parses; these check the
//! pieces -- the insertion point `push` and `warn` choose, ties, the
//! optional `file` of a warning, and every delimiter's JSON name -- so a
//! failure names the function at fault.

use std::path::PathBuf;

use serde_json::{Value, json};
use xenolith_lang_api::{DelimKind, LangId};

use super::{Direction, Fix, Report, Rule, SCHEMA, Violation, Warning, delim_kind_name};

fn at(file: &str, line: usize, col: usize, rule: Rule) -> Violation {
    Violation {
        rule,
        file: PathBuf::from(file),
        line,
        col,
        host: LangId::Nix,
        guest: LangId::Shell,
        sink: "packages.default.postBuild".to_owned(),
        site: DelimKind::NixIndented,
        why: "unlinted".to_owned(),
        directions: vec![Direction {
            kind: Fix::Mechanical,
            action: "extract it".to_owned(),
        }],
    }
}

fn warning(code: &str, file: Option<&str>) -> Warning {
    Warning {
        code: code.to_owned(),
        file: file.map(PathBuf::from),
        message: format!("{code} happened"),
    }
}

fn positions(report: &Report) -> Vec<(String, usize, usize, Rule)> {
    report
        .violations()
        .iter()
        .map(|v| (v.file.display().to_string(), v.line, v.col, v.rule))
        .collect()
}

fn warning_keys(report: &Report) -> Vec<(String, Option<PathBuf>, String)> {
    report
        .warnings()
        .iter()
        .map(|w| (w.code.clone(), w.file.clone(), w.message.clone()))
        .collect()
}

fn parse(json: &str) -> Value {
    serde_json::from_str(json).unwrap_or_else(|e| panic!("not JSON ({e}): {json}"))
}

// ---------------------------------------------------------------------
// Fix
// ---------------------------------------------------------------------

#[test]
fn fix_names_are_lowercase_and_distinct() {
    assert_eq!(Fix::Mechanical.as_str(), "mechanical");
    assert_eq!(Fix::Judgment.as_str(), "judgment");
}

#[test]
fn mechanical_sorts_before_judgment() {
    // Directions are ranked, the correct one first.
    assert!(Fix::Mechanical < Fix::Judgment);
}

// ---------------------------------------------------------------------
// Rule
// ---------------------------------------------------------------------

#[test]
fn every_rule_id_round_trips() {
    for &rule in Rule::ALL {
        assert_eq!(Rule::from_id(rule.as_str()), Some(rule), "{rule:?}");
    }
}

#[test]
fn rule_all_is_sorted_by_id_and_by_ord() {
    let ids: Vec<&str> = Rule::ALL.iter().map(|r| r.as_str()).collect();
    let mut sorted_ids = ids.clone();
    sorted_ids.sort_unstable();
    sorted_ids.dedup();
    assert_eq!(ids, sorted_ids);

    let rules = Rule::ALL.to_vec();
    let mut sorted = rules.clone();
    sorted.sort();
    assert_eq!(rules, sorted);
}

#[test]
fn from_id_is_exact_match_only() {
    for near in [
        "Xenolith",
        "stale_allow",
        "staleAllow",
        " xenolith",
        "dangling",
        "",
    ] {
        assert_eq!(Rule::from_id(near), None, "{near:?}");
    }
}

// ---------------------------------------------------------------------
// Violation: to_human, sort_key, to_value
// ---------------------------------------------------------------------

#[test]
fn to_human_is_the_documented_line() {
    let v = at("hosts/web.nix", 12, 3, Rule::Xenolith);
    assert_eq!(
        v.to_human(),
        "hosts/web.nix:12:3 xenolith: shell in nix packages.default.postBuild (unlinted)"
    );
}

#[test]
fn to_human_uses_the_rule_id_not_the_variant_name() {
    let v = at("a.nix", 1, 1, Rule::DanglingLoad);
    assert!(
        v.to_human().contains(" dangling-load: "),
        "{}",
        v.to_human()
    );
}

#[test]
fn sort_key_orders_file_then_line_then_col_then_rule() {
    let base = at("b.nix", 5, 5, Rule::Xenolith);
    assert!(at("a.nix", 9, 9, Rule::Xenolith).sort_key() < base.sort_key());
    assert!(at("b.nix", 4, 9, Rule::Xenolith).sort_key() < base.sort_key());
    assert!(at("b.nix", 5, 4, Rule::Xenolith).sort_key() < base.sort_key());
    assert!(at("b.nix", 5, 5, Rule::DanglingLoad).sort_key() < base.sort_key());
}

#[test]
fn sort_key_compares_lines_numerically_not_as_text() {
    assert!(
        at("a.nix", 9, 1, Rule::Xenolith).sort_key()
            < at("a.nix", 10, 1, Rule::Xenolith).sort_key()
    );
}

#[test]
fn to_value_carries_every_field_by_its_json_name() {
    let mut v = at("flake.nix", 42, 7, Rule::Xenolith);
    v.directions.push(Direction {
        kind: Fix::Judgment,
        action: "allow it".to_owned(),
    });
    assert_eq!(
        v.to_value(),
        json!({
            "rule": "xenolith",
            "file": "flake.nix",
            "line": 42,
            "col": 7,
            "host": "nix",
            "guest": "shell",
            "sink": "packages.default.postBuild",
            "site": "nix-indented",
            "why": "unlinted",
            "directions": [
                { "kind": "mechanical", "action": "extract it" },
                { "kind": "judgment", "action": "allow it" },
            ],
        })
    );
}

#[test]
fn to_value_keeps_direction_order() {
    // Ranked by the producer; the model must not re-sort them.
    let mut v = at("a.nix", 1, 1, Rule::Xenolith);
    v.directions = vec![
        Direction {
            kind: Fix::Judgment,
            action: "z".to_owned(),
        },
        Direction {
            kind: Fix::Mechanical,
            action: "a".to_owned(),
        },
    ];
    let value = v.to_value();
    let kinds: Vec<&str> = value
        .get("directions")
        .and_then(Value::as_array)
        .map(|ds| ds.iter().filter_map(|d| d.get("kind")?.as_str()).collect())
        .unwrap_or_default();
    assert_eq!(kinds, vec!["judgment", "mechanical"]);
}

#[test]
fn to_value_of_no_directions_is_an_empty_array() {
    let mut v = at("a.nix", 1, 1, Rule::Xenolith);
    v.directions.clear();
    assert_eq!(v.to_value().get("directions"), Some(&json!([])));
}

// ---------------------------------------------------------------------
// Warning::to_value
// ---------------------------------------------------------------------

#[test]
fn a_warning_with_a_file_has_three_keys() {
    assert_eq!(
        warning("symlink-skipped", Some("link.nix")).to_value(),
        json!({
            "code": "symlink-skipped",
            "file": "link.nix",
            "message": "symlink-skipped happened",
        })
    );
}

#[test]
fn a_warning_without_a_file_omits_the_key_rather_than_null() {
    // `"file"?` in `src/cli` §I: absent, not `null`.
    let value = warning("slow-scan", None).to_value();
    assert_eq!(
        value,
        json!({ "code": "slow-scan", "message": "slow-scan happened" })
    );
    assert!(value.get("file").is_none());
}

// ---------------------------------------------------------------------
// Report: push, warn, exit_code, to_json
// ---------------------------------------------------------------------

#[test]
fn a_new_report_is_empty_and_exits_zero() {
    let report = Report::new();
    assert!(report.violations().is_empty());
    assert!(report.warnings().is_empty());
    assert_eq!(report.exit_code(), 0);
    assert_eq!(report, Report::default());
}

#[test]
fn push_inserts_in_sort_order_whatever_the_arrival_order() {
    let mut report = Report::new();
    for v in [
        at("b.nix", 1, 1, Rule::Xenolith),
        at("a.nix", 10, 1, Rule::Xenolith),
        at("a.nix", 2, 5, Rule::Xenolith),
        at("a.nix", 2, 1, Rule::StaleAllow),
        at("a.nix", 2, 1, Rule::DanglingLoad),
    ] {
        report.push(v);
    }
    assert_eq!(
        positions(&report),
        vec![
            ("a.nix".to_owned(), 2, 1, Rule::DanglingLoad),
            ("a.nix".to_owned(), 2, 1, Rule::StaleAllow),
            ("a.nix".to_owned(), 2, 5, Rule::Xenolith),
            ("a.nix".to_owned(), 10, 1, Rule::Xenolith),
            ("b.nix".to_owned(), 1, 1, Rule::Xenolith),
        ]
    );
}

/// Pairs of findings at ONE position, same rule, differing in one field
/// that reaches the output.
fn same_position_pairs() -> Vec<(&'static str, Violation, Violation)> {
    let base = || at("a.nix", 1, 1, Rule::Xenolith);
    let mut pairs = Vec::new();

    let (mut a, mut b) = (base(), base());
    a.why = "alpha".to_owned();
    b.why = "beta".to_owned();
    pairs.push(("why", a, b));

    let (mut a, mut b) = (base(), base());
    a.sink = "script".to_owned();
    b.sink = "preStart".to_owned();
    pairs.push(("sink", a, b));

    let (a, mut b) = (base(), base());
    b.guest = LangId::Python;
    pairs.push(("guest", a, b));

    let (a, mut b) = (base(), base());
    b.host = LangId::Pkl;
    pairs.push(("host", a, b));

    let (a, mut b) = (base(), base());
    b.site = DelimKind::NixString;
    pairs.push(("site", a, b));

    let (a, mut b) = (base(), base());
    b.directions.push(Direction {
        kind: Fix::Judgment,
        action: "allow it".to_owned(),
    });
    pairs.push(("directions", a, b));

    pairs
}

#[test]
fn push_orders_equal_positions_independent_of_arrival() {
    // `src:V95`: the scan is parallel, so which of two findings at one
    // position arrives first is a race. `src:V11`: the bytes must not
    // depend on it. Both arrival orders must render the same report.
    for (field, a, b) in same_position_pairs() {
        let mut forward = Report::new();
        forward.push(a.clone());
        forward.push(b.clone());
        let mut reverse = Report::new();
        reverse.push(b);
        reverse.push(a);
        assert_eq!(forward.to_json(), reverse.to_json(), "differing in {field}");
        let human = |r: &Report| -> Vec<String> {
            r.violations().iter().map(Violation::to_human).collect()
        };
        assert_eq!(human(&forward), human(&reverse), "differing in {field}");
    }
}

#[test]
fn any_violation_sets_the_exit_code_to_one() {
    let mut report = Report::new();
    report.push(at("a.nix", 1, 1, Rule::Xenolith));
    assert_eq!(report.exit_code(), 1);
    report.push(at("b.nix", 1, 1, Rule::Xenolith));
    assert_eq!(
        report.exit_code(),
        1,
        "a count, not a boolean, would be wrong"
    );
}

#[test]
fn warnings_never_change_the_exit_code() {
    let mut report = Report::new();
    report.warn(warning("slow-scan", None));
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn warn_sorts_by_code_then_file_with_no_file_first() {
    let mut report = Report::new();
    report.warn(warning("symlink-skipped", Some("b")));
    report.warn(warning("slow-scan", None));
    report.warn(warning("symlink-skipped", Some("a")));
    report.warn(warning("symlink-skipped", None));
    let keys: Vec<(String, Option<PathBuf>)> = warning_keys(&report)
        .into_iter()
        .map(|(code, file, _)| (code, file))
        .collect();
    assert_eq!(
        keys,
        vec![
            ("slow-scan".to_owned(), None),
            ("symlink-skipped".to_owned(), None),
            ("symlink-skipped".to_owned(), Some(PathBuf::from("a"))),
            ("symlink-skipped".to_owned(), Some(PathBuf::from("b"))),
        ]
    );
}

#[test]
fn warn_orders_equal_code_and_file_independent_of_arrival() {
    // Same race as for violations (`src:V95`, `src:V11`): two warnings
    // with one code and one file differ in their message, which is
    // rendered, so arrival order must not decide the bytes.
    for file in [None, Some("a.nix")] {
        let mut alpha = warning("slow-scan", file);
        alpha.message = "alpha".to_owned();
        let mut beta = warning("slow-scan", file);
        beta.message = "beta".to_owned();

        let mut forward = Report::new();
        forward.warn(alpha.clone());
        forward.warn(beta.clone());
        let mut reverse = Report::new();
        reverse.warn(beta);
        reverse.warn(alpha);
        assert_eq!(warning_keys(&forward), warning_keys(&reverse), "{file:?}");
        assert_eq!(forward.to_json(), reverse.to_json(), "{file:?}");
    }
}

#[test]
fn to_json_of_an_empty_report_is_the_bare_envelope() {
    let json = Report::new().to_json();
    assert!(json.ends_with("}\n"), "{json:?}");
    assert!(!json.ends_with("\n\n"), "{json:?}");
    assert_eq!(
        parse(&json),
        json!({ "schema": SCHEMA, "violations": [], "warnings": [] })
    );
}

#[test]
fn to_json_renders_violations_and_warnings_in_report_order() {
    let mut report = Report::new();
    report.push(at("b.nix", 1, 1, Rule::Xenolith));
    report.push(at("a.nix", 1, 1, Rule::Xenolith));
    report.warn(warning("slow-scan", None));
    let value = parse(&report.to_json());
    let files: Vec<&str> = value
        .get("violations")
        .and_then(Value::as_array)
        .map(|vs| vs.iter().filter_map(|v| v.get("file")?.as_str()).collect())
        .unwrap_or_default();
    assert_eq!(files, vec!["a.nix", "b.nix"]);
    assert_eq!(
        value.get("warnings"),
        Some(&json!([{ "code": "slow-scan", "message": "slow-scan happened" }]))
    );
}

#[test]
fn to_json_is_pretty_with_keys_in_byte_order() {
    let json = Report::new().to_json();
    assert_eq!(
        json,
        "{\n  \"schema\": 1,\n  \"violations\": [],\n  \"warnings\": []\n}\n"
    );
}

// ---------------------------------------------------------------------
// delim_kind_name
// ---------------------------------------------------------------------

#[test]
fn every_delimiter_kind_has_its_own_kebab_case_name() {
    let kinds = [
        (DelimKind::NixIndented, "nix-indented"),
        (DelimKind::NixString, "nix-string"),
        (
            DelimKind::Heredoc {
                tag: "EOF".to_owned(),
                quoted: true,
                strip_indent: false,
            },
            "heredoc",
        ),
        (DelimKind::PklMultiline { pounds: 1 }, "pkl-multiline"),
        (
            DelimKind::YamlBlock {
                literal: true,
                chomp: Some('-'),
            },
            "yaml-block",
        ),
        (
            DelimKind::HtmlElement {
                tag: "script".to_owned(),
            },
            "html-element",
        ),
        (DelimKind::RustRawString { pounds: 2 }, "rust-raw-string"),
        (
            DelimKind::RubyHeredoc {
                tag: "SQL".to_owned(),
                squiggly: true,
            },
            "ruby-heredoc",
        ),
        (DelimKind::ArgvString, "argv-string"),
        (DelimKind::JustRecipe, "just-recipe"),
        (DelimKind::JustShebangRecipe, "just-shebang-recipe"),
    ];
    let mut names = Vec::new();
    for (kind, want) in &kinds {
        let name = delim_kind_name(kind);
        assert_eq!(name, *want, "{kind:?}");
        assert!(
            name.bytes().all(|b| b.is_ascii_lowercase() || b == b'-'),
            "{name}"
        );
        names.push(name);
    }
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), kinds.len());
}

#[test]
fn a_warning_renders_as_one_human_line_with_or_without_its_file() {
    assert_eq!(
        warning("x-y", Some("a.nix")).to_human(),
        "a.nix: warning: x-y: x-y happened"
    );
    assert_eq!(
        warning("x-y", None).to_human(),
        "warning: x-y: x-y happened"
    );
}

#[test]
fn a_site_pushed_with_its_hash_answers_it_and_the_json_never_shows_it() {
    let mut report = Report::new();
    report.push_site(at("a.nix", 2, 3, Rule::Xenolith), "cafe".to_owned());
    report.push(at("b.nix", 1, 1, Rule::StaleAllow));
    let first = at("a.nix", 2, 3, Rule::Xenolith);
    assert_eq!(report.allow_hash(&first), Some("cafe"));
    assert_eq!(
        report.allow_hash(&at("b.nix", 1, 1, Rule::StaleAllow)),
        None
    );
    assert!(!report.to_json().contains("cafe"), "{}", report.to_json());
}

#[test]
fn a_delimiter_name_ignores_the_variant_payload() {
    // The payload is detail for the rewriter; the JSON contract names the
    // kind only, so two heredocs report the same `site`.
    for (quoted, strip_indent) in [(false, false), (true, true)] {
        let kind = DelimKind::Heredoc {
            tag: "X".to_owned(),
            quoted,
            strip_indent,
        };
        assert_eq!(delim_kind_name(&kind), "heredoc");
    }
    for pounds in [0, 3] {
        assert_eq!(
            delim_kind_name(&DelimKind::PklMultiline { pounds }),
            "pkl-multiline"
        );
    }
}
