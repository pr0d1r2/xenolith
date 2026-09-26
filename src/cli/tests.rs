//! Unit tests for `cli` (`src:C139`): the dispatch `xnl` runs, driven
//! with in-memory streams instead of a spawned process.
//!
//! `tests/skeleton.rs` spawns the real binary and checks the process
//! boundary; these check every branch of the dispatch and of `refuse`,
//! including which STREAM each message lands on -- a refusal on stdout
//! corrupts a caller parsing JSON (`src/cli` §I) -- and what happens
//! when a stream cannot be written. The parser has its own mirror
//! (`src/cli/args/tests.rs`), and so does `langs`
//! (`src/cli/langs/tests.rs`); what is pinned here is that the dispatch
//! routes each verb where it belongs and refuses, with exit 2, every verb
//! whose engine has not landed (`src/cli:V24`).

use std::io::{self, Write};

use super::{EXIT_USAGE, refuse, run};

/// A stream that refuses every write, like a closed pipe.
struct Closed;

impl Write for Closed {
    fn write(&mut self, _: &[u8]) -> io::Result<usize> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }

    fn flush(&mut self) -> io::Result<()> {
        Err(io::Error::from(io::ErrorKind::BrokenPipe))
    }
}

/// Run with `args`, returning the exit code, stdout and stderr.
fn xnl(args: &[&str]) -> (u8, String, String) {
    let args: Vec<String> = args.iter().map(|a| (*a).to_owned()).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(&args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

/// Run with `args`, expect a refusal, and return stderr.
fn refused(args: &[&str]) -> String {
    let (code, out, err) = xnl(args);
    assert_eq!(code, EXIT_USAGE, "{args:?}: {err}");
    assert!(out.is_empty(), "{args:?}: {out:?}");
    assert!(err.ends_with('\n'), "{args:?}: {err:?}");
    err
}

#[test]
fn the_usage_exit_code_is_two() {
    // `src/cli:V24`: 0 ok, 1 violation, 2 usage. 1 is for findings, so a
    // refusal must never be it.
    assert_eq!(EXIT_USAGE, 2);
}

// ---------------------------------------------------------------------
// --version
// ---------------------------------------------------------------------

#[test]
fn version_prints_the_library_version_on_stdout() {
    let (code, out, err) = xnl(&["--version"]);
    assert_eq!(code, 0);
    assert_eq!(out, format!("xnl {}\n", crate::VERSION));
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn dash_capital_v_is_the_short_form() {
    assert_eq!(xnl(&["-V"]), xnl(&["--version"]));
}

#[test]
fn dash_lowercase_v_is_not_version() {
    // `-v` is `--verbose` in most tools; answering it as version would
    // make a typo succeed.
    let err = refused(&["-v"]);
    assert!(err.contains("`-v`"), "{err:?}");
}

#[test]
fn version_looks_only_at_the_first_argument() {
    let (code, out, _) = xnl(&["--version", "check"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("xnl "));
}

#[test]
fn version_after_a_verb_is_an_unknown_flag_of_that_verb() {
    let err = refused(&["check", "--version"]);
    assert!(err.contains("`--version`"), "{err:?}");
    assert!(err.contains("check"), "{err:?}");
}

#[test]
fn version_to_a_closed_stdout_does_not_panic() {
    // Nothing can report a failed stdout write, and panicking would put
    // a panic message where no caller asked for one.
    let args = vec!["--version".to_owned()];
    let mut err = Vec::new();
    assert_eq!(run(&args, &mut Closed, &mut err), 0);
    assert!(err.is_empty());
}

// ---------------------------------------------------------------------
// usage
// ---------------------------------------------------------------------

#[test]
fn no_arguments_prints_the_usage_on_stderr() {
    let err = refused(&[]);
    assert!(err.starts_with("usage: xnl "), "{err:?}");
    for word in [
        "check",
        "extract",
        "graph",
        "lint",
        "langs",
        "migrate",
        "--write",
        "--format",
        "--verbose",
        "--strict-hosts",
        "--version",
    ] {
        assert!(err.contains(word), "usage lacks {word}: {err}");
    }
}

#[test]
fn an_unknown_word_or_flag_is_refused_by_name() {
    for word in ["--help", "frobnicate", "-", ""] {
        let err = refused(&[word]);
        assert!(err.starts_with("xnl: "), "{err:?}");
        assert!(err.contains(&format!("`{word}`")), "{err:?}");
    }
}

#[test]
fn a_usage_error_is_followed_by_the_usage() {
    // The line that says what was wrong comes FIRST, so a terminal
    // showing only the top of the message shows the mistake.
    let err = refused(&["check", "--frobnicate"]);
    let first = err.lines().next().unwrap_or_default();
    assert!(first.contains("`--frobnicate`"), "{err:?}");
    assert!(err.contains("usage: xnl "), "{err:?}");
}

#[test]
fn flags_are_parsed_before_a_verb_is_refused() {
    // A bad flag on a verb without an engine is still a bad flag: the
    // user learns about the typo today, not the day the engine lands.
    let err = refused(&["check", "--format", "xml"]);
    assert!(err.contains("`xml`"), "{err:?}");
    assert!(!err.contains("not implemented"), "{err:?}");
}

// ---------------------------------------------------------------------
// verbs whose engines have not landed
// ---------------------------------------------------------------------

#[test]
fn every_scanning_verb_is_refused_naming_the_task_that_brings_it() {
    // Refusing matters more than it looks: `xnl check` exiting 0 having
    // scanned nothing is, in a gate, a clean tree.
    for (args, task) in [
        (&["extract", "a.nix:3"][..], "src/extract:T22"),
        (&["extract", "--write", "a.nix"][..], "src/extract:T22"),
        (&["graph", "--verbose"][..], "src/graph:T21"),
    ] {
        let verb = args.first().copied().unwrap_or_default();
        let err = refused(args);
        assert!(
            err.starts_with(&format!("xnl: `{verb}` is not implemented yet")),
            "{args:?}: {err:?}"
        );
        assert!(err.contains(task), "{args:?} should name {task}: {err:?}");
    }
}

#[test]
fn check_is_routed_to_its_engine_not_refused() {
    // `src:T153`: the arm runs the engine. A named path that does not
    // exist is the engine's refusal (`src:V57`), not the not-yet one.
    let sandbox = crate::discover::Sandbox::new();
    let root = sandbox.plain("r");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = super::run_in(&root, &["check", "nope.nix"], &mut out, &mut err);
    let err = String::from_utf8_lossy(&err);
    assert_eq!(code, EXIT_USAGE, "{err}");
    assert!(!err.contains("not implemented"), "{err:?}");
    assert!(err.contains("nope.nix"), "{err:?}");
}

#[test]
fn lint_is_routed_to_its_engine_not_refused() {
    // `src/lint:T24`: the arm runs the lint engine. A named path that
    // does not exist is discovery's refusal (`src:V57`), not the not-yet
    // one.
    let sandbox = crate::discover::Sandbox::new();
    let root = sandbox.plain("r");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = super::run_in(&root, &["lint", "nope.sh"], &mut out, &mut err);
    let err = String::from_utf8_lossy(&err);
    assert_eq!(code, EXIT_USAGE, "{err}");
    assert!(!err.contains("not implemented"), "{err:?}");
    assert!(err.contains("nope.sh"), "{err:?}");
}

#[test]
fn migrate_is_routed_to_its_engine_not_refused() {
    // `src/cli:T97`: with no legacy list at the root there is nothing to
    // migrate, which is success and silence (`src/cli` §I).
    let sandbox = crate::discover::Sandbox::new();
    let root = sandbox.plain("r");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = super::run_in(&root, &["migrate"], &mut out, &mut err);
    let err = String::from_utf8_lossy(&err);
    assert_eq!(code, 0, "{err}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn sarif_is_refused_naming_its_own_task() {
    for verb in ["check", "graph", "lint"] {
        let err = refused(&[verb, "--format", "sarif"]);
        assert!(err.contains("sarif"), "{verb}: {err:?}");
        assert!(err.contains("src/cli:T103"), "{verb}: {err:?}");
    }
}

#[test]
fn a_refusal_to_a_closed_stderr_leaves_stdout_empty() {
    let args = vec!["graph".to_owned()];
    let mut out = Vec::new();
    assert_eq!(run(&args, &mut out, &mut Closed), EXIT_USAGE);
    assert!(out.is_empty());
}

// ---------------------------------------------------------------------
// langs
// ---------------------------------------------------------------------

#[test]
fn langs_lists_every_language_on_stdout_and_exits_zero() {
    let (code, out, err) = xnl(&["langs"]);
    assert_eq!(code, 0, "{err}");
    assert!(err.is_empty(), "{err:?}");
    for id in xenolith_lang_api::LangId::ALL {
        assert!(
            out.lines().any(|line| line.starts_with(id.as_str())),
            "{id} missing from {out}"
        );
    }
}

#[test]
fn langs_json_is_the_envelope_with_langs() {
    let (code, out, err) = xnl(&["langs", "--format", "json", "--verbose"]);
    assert_eq!(code, 0, "{err}");
    let value: serde_json::Value =
        serde_json::from_str(&out).unwrap_or_else(|e| panic!("{e}: {out}"));
    assert_eq!(
        value.get("schema").and_then(serde_json::Value::as_u64),
        Some(u64::from(crate::model::SCHEMA))
    );
    assert!(
        value.get("langs").is_some_and(serde_json::Value::is_array),
        "{value}"
    );
}

#[test]
fn langs_to_a_closed_stdout_does_not_panic() {
    let args = vec!["langs".to_owned()];
    let mut err = Vec::new();
    assert_eq!(run(&args, &mut Closed, &mut err), 0);
    assert!(err.is_empty());
}

// ---------------------------------------------------------------------
// refuse
// ---------------------------------------------------------------------

#[test]
fn refuse_writes_the_message_and_a_newline() {
    let mut err = Vec::new();
    assert_eq!(refuse(&mut err, "no."), EXIT_USAGE);
    assert_eq!(err, b"no.\n");
}

#[test]
fn refuse_to_a_closed_stderr_still_exits_two() {
    // A clean exit 2 beats a panic on the stream that just failed.
    assert_eq!(refuse(&mut Closed, "no."), EXIT_USAGE);
}
