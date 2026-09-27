//! The core model: what a violation IS, and the bytes it becomes.
//!
//! Every verb reports through this (`src:V1`), and a consumer parses the
//! JSON rather than the prose, so the shape is a contract with a version
//! number on it (`src/cli:V24`).

use xenolith::model::{Direction, Fix, Report, Rule, Violation, Warning};
use xenolith_lang_api::{DelimKind, LangId};

fn violation() -> Violation {
    Violation {
        rule: Rule::Xenolith,
        file: "flake.nix".into(),
        line: 42,
        col: 7,
        host: LangId::Nix,
        guest: LangId::Shell,
        sink: "packages.default.postBuild".into(),
        site: DelimKind::NixIndented,
        why: "a 12-line bash script here is unlinted and untested".into(),
        directions: vec![
            Direction {
                kind: Fix::Mechanical,
                action: "extract to scripts/post-build.sh and load it".into(),
            },
            Direction {
                kind: Fix::Judgment,
                action: "allow it in xenolith.toml with a reason".into(),
            },
        ],
    }
}

#[test]
fn a_violation_carries_everything_a_reader_needs() {
    // `src:V1`: rule, position, both languages, the sink, the delimiter
    // kind, a why, and at least one direction. Never a bare "bad".
    let v = violation();
    assert_eq!(v.rule.as_str(), "xenolith");
    assert_eq!(v.host, LangId::Nix);
    assert_eq!(v.guest, LangId::Shell);
    assert!(!v.why.is_empty());
    assert!(!v.directions.is_empty());
}

#[test]
fn every_rule_id_is_kebab_case_and_distinct() {
    let mut seen: Vec<&str> = Rule::ALL.iter().map(|r| r.as_str()).collect();
    let count = seen.len();
    seen.sort_unstable();
    seen.dedup();
    assert_eq!(seen.len(), count, "two rules share an id");

    for rule in Rule::ALL {
        let id = rule.as_str();
        assert!(
            id.chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'),
            "{id} is not kebab-case"
        );
        assert_eq!(Rule::from_id(id), Some(*rule), "{id} does not round trip");
    }
}

#[test]
fn the_rule_set_is_the_one_the_cli_documents() {
    let ids: Vec<&str> = Rule::ALL.iter().map(|r| r.as_str()).collect();
    assert_eq!(
        ids,
        vec![
            "dangling-load",
            "host-parse-error",
            "orphan-extract",
            "stale-allow",
            "stale-exclude",
            "stale-rule",
            "xenolith",
        ]
    );
}

#[test]
fn violations_sort_by_file_then_line_then_col() {
    // `src:V11`. The engines scan in parallel (`src/check:V95`), so merged
    // output has to be put in order by the model rather than by the
    // order results happened to arrive.
    let mut report = Report::new();
    for (file, line, col) in [
        ("b.nix", 1, 1),
        ("a.nix", 9, 1),
        ("a.nix", 2, 5),
        ("a.nix", 2, 1),
    ] {
        let mut v = violation();
        v.file = file.into();
        v.line = line;
        v.col = col;
        report.push(v);
    }
    let order: Vec<(String, usize, usize)> = report
        .violations()
        .iter()
        .map(|v| (v.file.display().to_string(), v.line, v.col))
        .collect();
    assert_eq!(
        order,
        vec![
            ("a.nix".to_owned(), 2, 1),
            ("a.nix".to_owned(), 2, 5),
            ("a.nix".to_owned(), 9, 1),
            ("b.nix".to_owned(), 1, 1),
        ]
    );
}

#[test]
fn the_json_envelope_is_schema_one() {
    let json = Report::new().to_json();
    assert!(json.contains("\"schema\": 1"), "got {json}");
    assert!(json.contains("\"violations\""), "got {json}");
    assert!(json.contains("\"warnings\""), "got {json}");
    assert!(json.ends_with('\n'), "output must end with a newline");
}

#[test]
fn json_keys_are_sorted_so_the_bytes_are_stable() {
    let mut report = Report::new();
    report.push(violation());
    let json = report.to_json();

    // Inside a violation object, the keys appear in sorted order.
    let keys = [
        "\"col\"",
        "\"directions\"",
        "\"file\"",
        "\"guest\"",
        "\"host\"",
        "\"line\"",
        "\"rule\"",
        "\"sink\"",
        "\"site\"",
        "\"why\"",
    ];
    let mut at = 0;
    for key in keys {
        let found = json[at..]
            .find(key)
            .unwrap_or_else(|| panic!("{key} missing or out of order in {json}"));
        at += found + key.len();
    }
}

#[test]
fn the_same_report_renders_the_same_bytes_every_time() {
    let mut report = Report::new();
    report.push(violation());
    report.warn(Warning {
        code: "symlink-skipped".into(),
        file: Some("link.nix".into()),
        message: "a tracked symlink is not scanned".into(),
    });
    assert_eq!(report.to_json(), report.to_json());
}

#[test]
fn languages_and_delimiters_render_as_their_names() {
    let mut report = Report::new();
    report.push(violation());
    let json = report.to_json();
    assert!(json.contains("\"host\": \"nix\""), "got {json}");
    assert!(json.contains("\"guest\": \"shell\""), "got {json}");
    assert!(json.contains("\"site\": \"nix-indented\""), "got {json}");
}

#[test]
fn a_direction_states_whether_a_machine_may_apply_it() {
    let json = {
        let mut report = Report::new();
        report.push(violation());
        report.to_json()
    };
    assert!(json.contains("\"kind\": \"mechanical\""), "got {json}");
    assert!(json.contains("\"kind\": \"judgment\""), "got {json}");
}

#[test]
fn a_warning_does_not_become_a_violation() {
    let mut report = Report::new();
    report.warn(Warning {
        code: "unclaimed".into(),
        file: None,
        message: "no host claims README.md".into(),
    });
    assert!(report.violations().is_empty());
    assert_eq!(report.warnings().len(), 1);
    // `src/cli` §I: warnings never change the exit code.
    assert_eq!(report.exit_code(), 0);
}

#[test]
fn a_violation_sets_the_exit_code_to_one() {
    let mut report = Report::new();
    report.push(violation());
    assert_eq!(report.exit_code(), 1);
}

#[test]
fn human_output_names_the_guest_the_host_and_the_sink() {
    let line = violation().to_human();
    assert_eq!(
        line,
        "flake.nix:42:7 xenolith: shell in nix packages.default.postBuild \
         (a 12-line bash script here is unlinted and untested)"
    );
}
