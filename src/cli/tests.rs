//! Unit tests for `cli` (`src:C139`): the dispatch `xnl` runs, driven
//! with in-memory streams instead of a spawned process.
//!
//! `tests/skeleton.rs` spawns the real binary and checks the process
//! boundary; these check every branch of the dispatch and of `refuse`,
//! including which STREAM each message lands on -- a refusal on stdout
//! corrupts a caller parsing JSON (`src/cli` §I) -- and what happens
//! when a stream cannot be written.

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
    let (code, out, _) = xnl(&["-v"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(out.is_empty());
}

#[test]
fn version_looks_only_at_the_first_argument() {
    let (code, out, _) = xnl(&["--version", "check"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("xnl "));
}

#[test]
fn version_after_a_verb_is_still_a_refusal() {
    let (code, out, err) = xnl(&["check", "--version"]);
    assert_eq!(code, EXIT_USAGE);
    assert!(out.is_empty());
    assert!(err.contains("`check`"), "{err:?}");
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
// refusals
// ---------------------------------------------------------------------

#[test]
fn no_arguments_is_a_usage_refusal_on_stderr() {
    let (code, out, err) = xnl(&[]);
    assert_eq!(code, EXIT_USAGE);
    assert!(out.is_empty(), "{out:?}");
    assert!(err.starts_with("usage: xnl --version\n"), "{err:?}");
    assert!(err.contains("src/cli:T9"), "{err:?}");
    assert!(err.ends_with('\n'));
}

#[test]
fn every_planned_verb_is_refused_by_name() {
    for verb in ["check", "extract", "graph", "lint", "langs"] {
        let (code, out, err) = xnl(&[verb]);
        assert_eq!(code, EXIT_USAGE, "{verb}");
        assert!(out.is_empty(), "{verb}: {out:?}");
        assert!(err.starts_with(&format!("xnl: `{verb}` ")), "{err:?}");
        assert!(err.contains("src/cli:T9"), "{err:?}");
    }
}

#[test]
fn an_unknown_word_or_flag_is_refused_by_name() {
    for word in ["--help", "frobnicate", "-", ""] {
        let (code, out, err) = xnl(&[word]);
        assert_eq!(code, EXIT_USAGE, "{word:?}");
        assert!(out.is_empty(), "{word:?}");
        assert!(err.contains(&format!("`{word}`")), "{err:?}");
    }
}

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

#[test]
fn a_refusal_to_a_closed_stderr_leaves_stdout_empty() {
    let args = vec!["check".to_owned()];
    let mut out = Vec::new();
    assert_eq!(run(&args, &mut out, &mut Closed), EXIT_USAGE);
    assert!(out.is_empty());
}
