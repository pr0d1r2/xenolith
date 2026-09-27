//! Lint results: the mirror of `src/lint/report.rs` (`src:C139`).
//!
//! Pinned: the exit code a set of results makes (`src/lint:V92`), the
//! JSON shape (`src/lint` §I) and warning order.

use std::path::PathBuf;

use serde_json::Value;
use xenolith_lang_api::LangId;

use super::{Kind, LintReport, Outcome, Source, Status};
use crate::lint::Finding;
use crate::model::Warning;

fn outcome(status: Status) -> Outcome {
    Outcome {
        file: PathBuf::from("scripts/a.sh"),
        kind: Kind::Extract,
        guest: Some(LangId::Shell),
        dialect: Some("bash".to_owned()),
        check: "shellcheck".to_owned(),
        argv: vec!["shellcheck".to_owned(), "scripts/a.sh".to_owned()],
        source: Source::Default,
        status,
        exit: Some(1),
        raw_tail: Some("line 3: bad".to_owned()),
        fixer: false,
        findings: Vec::new(),
    }
}

fn with(statuses: &[Status]) -> LintReport {
    let mut report = LintReport::new();
    for status in statuses {
        report.push(outcome(*status));
    }
    report
}

#[test]
fn the_exit_code_is_the_worst_status() {
    assert_eq!(with(&[]).exit_code(), 0);
    assert_eq!(with(&[Status::Pass, Status::Skipped]).exit_code(), 0);
    assert_eq!(with(&[Status::Pass, Status::Fail]).exit_code(), 1);
    assert_eq!(with(&[Status::Error, Status::Fail]).exit_code(), 2);
}

#[test]
fn outcomes_keep_the_order_they_ran_in() {
    let report = with(&[Status::Fail, Status::Pass]);
    let statuses: Vec<Status> = report.outcomes().iter().map(|o| o.status).collect();
    assert_eq!(statuses, [Status::Fail, Status::Pass]);
}

/// The report as parsed JSON.
fn parsed(report: &LintReport) -> Value {
    serde_json::from_str(&report.to_json()).unwrap_or_else(|e| panic!("not JSON: {e}"))
}

/// The value at a JSON pointer, `Null` when absent.
fn at(json: &Value, pointer: &str) -> Value {
    json.pointer(pointer).cloned().unwrap_or(Value::Null)
}

#[test]
fn json_carries_the_envelope_and_every_result_field() {
    let json = parsed(&with(&[Status::Fail]));
    let empty = Value::Array(Vec::new());
    assert_eq!(at(&json, "/schema"), 1);
    assert_eq!(at(&json, "/violations"), empty);
    for (key, want) in [
        ("file", Value::from("scripts/a.sh")),
        ("kind", Value::from("extract")),
        ("guest", Value::from("shell")),
        ("dialect", Value::from("bash")),
        ("check", Value::from("shellcheck")),
        ("argv/1", Value::from("scripts/a.sh")),
        ("source", Value::from("default")),
        ("status", Value::from("fail")),
        ("exit", Value::from(1)),
        ("findings", empty.clone()),
        ("raw_tail", Value::from("line 3: bad")),
    ] {
        assert_eq!(at(&json, &format!("/results/0/{key}")), want, "{key}");
    }
    assert!(json.pointer("/results/0/fixer").is_none(), "{json}");
}

#[test]
fn a_host_result_has_no_guest_and_no_dialect() {
    let mut report = LintReport::new();
    report.push(Outcome {
        kind: Kind::Host,
        guest: None,
        dialect: None,
        raw_tail: None,
        ..outcome(Status::Pass)
    });
    let json = parsed(&report);
    assert_eq!(at(&json, "/results/0/kind"), "host");
    for key in ["guest", "dialect", "raw_tail"] {
        let pointer = format!("/results/0/{key}");
        assert_eq!(json.pointer(&pointer), Some(&Value::Null), "{key}");
    }
}

#[test]
fn warnings_are_sorted_and_kept_once() {
    let mut report = LintReport::new();
    let warning = |code: &str| Warning {
        code: code.to_owned(),
        file: None,
        message: "m".to_owned(),
    };
    report.warn(warning("b"));
    report.warn(warning("a"));
    report.warn(warning("b"));
    let codes: Vec<&str> = report.warnings().iter().map(|w| w.code.as_str()).collect();
    assert_eq!(codes, ["a", "b"]);
    let json = parsed(&report);
    assert_eq!(at(&json, "/warnings/0/code"), "a");
    assert!(json.pointer("/warnings/0/file").is_none());
}

#[test]
fn findings_render_in_the_documented_shape() {
    let mut report = LintReport::new();
    report.push(Outcome {
        findings: vec![Finding {
            line: 3,
            col: 7,
            code: "SC2086".to_owned(),
            severity: "warning".to_owned(),
            message: "quote it".to_owned(),
        }],
        raw_tail: None,
        ..outcome(Status::Fail)
    });
    let json = parsed(&report);
    for (key, want) in [
        ("line", Value::from(3)),
        ("col", Value::from(7)),
        ("code", Value::from("SC2086")),
        ("severity", Value::from("warning")),
        ("message", Value::from("quote it")),
    ] {
        let pointer = format!("/results/0/findings/0/{key}");
        assert_eq!(at(&json, &pointer), want, "{key}");
    }
    assert_eq!(at(&json, "/results/0/raw_tail"), Value::Null);
}
