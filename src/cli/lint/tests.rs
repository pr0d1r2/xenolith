//! `xnl lint` in the CLI: the mirror of `src/cli/lint.rs` (`src:C139`).
//!
//! Pinned: which stream each part of a lint report lands on, the human
//! line shape and summary (`src/lint` §I), the exit code (`src/lint:V92`),
//! and the refusals.

use std::path::PathBuf;

use xenolith_lang_api::LangId;

use super::{Flags, render, run};
use crate::cli::EXIT_USAGE;
use crate::cli::args::{OutputFormat, Scan};
use crate::discover::Sandbox;
use crate::lint::{Finding, Kind, LintReport, Outcome, Source, Status};
use crate::model::Warning;

fn outcome(check: &str, status: Status, exit: Option<i32>, tail: Option<&str>) -> Outcome {
    Outcome {
        file: PathBuf::from("a.sh"),
        kind: Kind::Extract,
        guest: Some(LangId::Shell),
        dialect: None,
        check: check.to_owned(),
        argv: vec![check.to_owned(), "a.sh".to_owned()],
        source: Source::Default,
        status,
        exit,
        raw_tail: tail.map(str::to_owned),
        fixer: false,
        findings: Vec::new(),
    }
}

fn rendered(report: &LintReport, format: OutputFormat, verbose: bool) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = render(report, format, verbose, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn scan(paths: &[&str]) -> Scan {
    Scan {
        format: OutputFormat::Human,
        paths: paths.iter().map(PathBuf::from).collect(),
    }
}

fn ran(flags: Flags, paths: &[&str]) -> (u8, String, String) {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&root, &scan(paths), flags, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn human_lists_what_did_not_pass_with_its_tail_then_the_summary() {
    let mut report = LintReport::new();
    report.push(outcome("alpha", Status::Pass, Some(0), None));
    report.push(outcome("beta", Status::Fail, Some(1), Some("l1\nl2")));
    report.push(outcome(
        "gamma",
        Status::Error,
        None,
        Some("`gamma` is not on PATH"),
    ));
    report.push(outcome("own", Status::Skipped, None, None));
    report.warn(Warning {
        code: "untrusted-command".to_owned(),
        file: None,
        message: "`own` was not run".to_owned(),
    });
    let (code, out, err) = rendered(&report, OutputFormat::Human, false);
    assert_eq!(code, 2);
    assert_eq!(
        out,
        "a.sh: beta: failed (exit 1)\n    l1\n    l2\n\
         a.sh: gamma: `gamma` is not on PATH\n\
         3 checks, 2 failed\n"
    );
    assert_eq!(err, "warning: untrusted-command: `own` was not run\n");
}

#[test]
fn a_fixer_that_did_not_pass_says_so() {
    let mut report = LintReport::new();
    report.push(Outcome {
        fixer: true,
        ..outcome("fixit", Status::Fail, Some(4), Some("no"))
    });
    let (code, out, _) = rendered(&report, OutputFormat::Human, false);
    assert_eq!(code, 1);
    assert!(
        out.starts_with("a.sh: fixer fixit: failed (exit 4)\n"),
        "{out:?}"
    );
}

#[test]
fn human_puts_one_line_per_finding_in_place_of_the_tail() {
    let mut report = LintReport::new();
    report.push(Outcome {
        findings: vec![
            Finding {
                line: 3,
                col: 7,
                code: "SC2086".to_owned(),
                severity: "info".to_owned(),
                message: "Double quote.".to_owned(),
            },
            Finding {
                line: 9,
                col: 1,
                code: "SC2034".to_owned(),
                severity: "warning".to_owned(),
                message: "unused".to_owned(),
            },
        ],
        ..outcome("shellcheck", Status::Fail, Some(1), None)
    });
    let (code, out, _) = rendered(&report, OutputFormat::Human, false);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "a.sh:3:7 shellcheck: SC2086 (info) Double quote.\n\
         a.sh:9:1 shellcheck: SC2034 (warning) unused\n\
         1 checks, 1 failed\n"
    );
}

#[test]
fn a_clean_run_is_silent_unless_verbose() {
    let mut report = LintReport::new();
    report.push(outcome("alpha", Status::Pass, Some(0), None));
    let (code, out, err) = rendered(&report, OutputFormat::Human, false);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let (_, out, _) = rendered(&report, OutputFormat::Human, true);
    assert_eq!(out, "1 checks, 0 failed\n");
}

#[test]
fn json_is_the_envelope_on_stdout() {
    let mut report = LintReport::new();
    report.push(outcome("beta", Status::Fail, Some(1), Some("x")));
    let (code, out, err) = rendered(&report, OutputFormat::Json, false);
    assert_eq!(code, 1);
    assert!(err.is_empty(), "{err:?}");
    assert!(out.contains("\"results\""), "{out}");
    assert!(out.ends_with("}\n"), "{out}");
}

#[test]
fn a_named_path_that_does_not_exist_is_refused_by_discovery() {
    let (code, out, err) = ran(Flags::default(), &["nope.sh"]);
    assert_eq!(code, EXIT_USAGE, "{err}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("nope.sh"), "{err:?}");
}

#[test]
fn a_config_that_does_not_parse_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    crate::discover::write(&root, "xenolith.toml", "version = 1\n[lint]\nbogus = 1\n");
    crate::discover::write(&root, "a.txt", "x\n");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(
        &root,
        &scan(&["a.txt"]),
        Flags::default(),
        &mut out,
        &mut err,
    );
    let err = String::from_utf8_lossy(&err);
    assert_eq!(code, EXIT_USAGE, "{err}");
    assert!(err.contains("lint.bogus"), "{err:?}");
}

#[test]
fn fix_is_routed_to_the_engine() {
    // `src/lint:T87`: `--fix` runs; a missing path is discovery's refusal.
    let flags = Flags {
        fix: true,
        ..Flags::default()
    };
    let (code, _, err) = ran(flags, &["nope.sh"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(!err.contains("not implemented"), "{err:?}");
    assert!(err.contains("nope.sh"), "{err:?}");
}

#[test]
fn trust_config_is_routed_to_the_engine() {
    // `src/lint:T92`: the flag runs; a missing path is discovery's refusal.
    let flags = Flags {
        trust_config: true,
        ..Flags::default()
    };
    let (code, _, err) = ran(flags, &["nope.sh"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(!err.contains("not implemented"), "{err:?}");
    assert!(err.contains("nope.sh"), "{err:?}");
}
