//! Unit tests for `syntax.rs` (`src:C139`): the check agrees with Tcl's
//! own parser. Every verdict below was read off `tclsh` 8.5 first --
//! `info complete` for the unclosed openers, an evaluation with no
//! commands defined for the extra characters (`languages/shells/tcl:R208`).

use std::ffi::OsString;

use xenolith_lang_api::{FileArg, Format, LintCmd};

use super::{SyntaxError, check, lint_cmd, run};

/// The error `check` finds in `src`, as `line:col: message`.
fn error(src: &str) -> Option<String> {
    check(src)
        .err()
        .map(|SyntaxError { line, col, message }| format!("{line}:{col}: {message}"))
}

#[test]
fn a_clean_script_has_no_error() {
    for src in [
        "",
        "puts hi\n",
        "puts hi; set x 1\n",
        "set x [expr {1+2}]\n",
        "expect \"$ \"\n",
        "puts {a {b} c}\n",
        "puts [list a {b c}]\n",
        "puts $a(x) ${b} $::env(HOME) $a::b $\n",
        "puts {*}{a b}\n",
        "proc f {} {\n  return 1\n}\n",
        "puts \"a]\" a]\n",
        "puts \\\n  more\n",
        "puts \\\\\n",
    ] {
        assert_eq!(error(src), None, "{src:?}");
    }
}

#[test]
fn an_unclosed_opener_is_reported_where_it_opens() {
    assert_eq!(
        error("set x 1\nputs {hi\n").as_deref(),
        Some("2:6: missing close-brace")
    );
    assert_eq!(error("puts \"hi\n").as_deref(), Some("1:6: missing \""));
    assert_eq!(
        error("puts [pwd\n").as_deref(),
        Some("1:6: missing close-bracket")
    );
    assert_eq!(
        error("puts ${a\n").as_deref(),
        Some("1:6: missing close-brace for variable name")
    );
    assert_eq!(error("puts $a(x\n").as_deref(), Some("1:8: missing )"));
}

#[test]
fn a_backslash_newline_ending_the_text_is_incomplete() {
    assert_eq!(
        error("puts a \\\n").as_deref(),
        Some("1:8: missing line after backslash-newline")
    );
    // An escaped backslash before the newline is a literal backslash.
    assert_eq!(error("puts a \\\\\n"), None);
    // Anything after the newline completes the command.
    assert_eq!(error("puts a \\\n "), None);
}

#[test]
fn extra_characters_after_a_close_are_errors() {
    assert_eq!(
        error("puts {a}b\n").as_deref(),
        Some("1:9: extra characters after close-brace")
    );
    assert_eq!(
        error("puts \"a\"b\n").as_deref(),
        Some("1:9: extra characters after close-quote")
    );
    // Also in a substitution Tcl parses when it reaches the word.
    assert_eq!(
        error("puts [list {a}b]\n").as_deref(),
        Some("1:15: extra characters after close-brace")
    );
    // `]` ends a word only inside a substitution.
    assert_eq!(error("puts [list {a}]\n"), None);
    assert_eq!(
        error("puts {a}]\n").as_deref(),
        Some("1:9: extra characters after close-brace")
    );
}

#[test]
fn a_braced_word_is_matched_never_entered() {
    // A `proc` body is text until the proc runs: Tcl reports nothing in
    // it now, and neither does the check.
    assert_eq!(error("proc f {} {\n  puts {a}b\n}\n"), None);
    assert_eq!(error("set x {a \"b}\n"), None);
    assert_eq!(error("set x {a [b}\n"), None);
    // An escaped brace does not count.
    assert_eq!(error("set x {a \\{ b}\n"), None);
    assert_eq!(
        error("set x {a \\} b\n").as_deref(),
        Some("1:7: missing close-brace")
    );
}

#[test]
fn braces_count_inside_a_comment_as_tcl_counts_them() {
    // Tcl's own sharp edge: the brace in the comment opens the body's
    // count, so the body never closes.
    assert_eq!(
        error("proc f {} {\n  # {\n}\n").as_deref(),
        Some("1:11: missing close-brace")
    );
    // A top-level comment is a comment: its brace is text.
    assert_eq!(error("# comment {\nputs hi\n"), None);
    // A backslash-newline continues a comment.
    assert_eq!(error("# comment \\\n {\n"), None);
}

#[test]
fn the_scan_stops_at_the_first_error_as_tcl_does() {
    // The extra character stops the parse; the unclosed brace after it is
    // never reached.
    assert_eq!(
        error("set x {a}b {\n").as_deref(),
        Some("1:10: extra characters after close-brace")
    );
}

#[test]
fn a_dollar_opens_an_index_even_with_no_name() {
    // `$(x)` is element `x` of the array named "".
    assert_eq!(error("puts $(x)\n"), None);
    assert_eq!(error("puts $(x\n").as_deref(), Some("1:7: missing )"));
}

#[test]
fn an_expansion_prefix_is_not_a_braced_word() {
    assert_eq!(error("puts {*}[list a b]\n"), None);
    assert_eq!(
        error("puts {*}[list a\n").as_deref(),
        Some("1:9: missing close-bracket")
    );
    // Alone, `{*}` is a braced word holding `*`.
    assert_eq!(error("puts {*}\n"), None);
}

#[test]
fn a_column_counts_bytes() {
    assert_eq!(
        error("puts \u{e9} {x\n").as_deref(),
        Some("1:9: missing close-brace")
    );
}

/// `run` over `args`, returning its exit code and what it printed.
fn ran(args: &[&str]) -> (u8, String) {
    let mut out = Vec::new();
    let code = run(args.iter().map(OsString::from), &mut out);
    (code, String::from_utf8_lossy(&out).into_owned())
}

#[test]
fn run_reports_each_bad_file_and_exits_by_the_worst() {
    let dir = std::env::temp_dir().join(format!("xenolith-tcl-syntax-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
    let good = dir.join("good.tcl");
    let bad = dir.join("bad.tcl");
    std::fs::write(&good, "puts hi\n").unwrap_or_else(|e| panic!("{e}"));
    std::fs::write(&bad, "puts {hi\n").unwrap_or_else(|e| panic!("{e}"));
    let (good, bad) = (good.display().to_string(), bad.display().to_string());

    assert_eq!(ran(&[&good]), (0, String::new()));
    assert_eq!(
        ran(&[&good, &bad]),
        (1, format!("{bad}:1:6: missing close-brace\n"))
    );
    let missing = dir.join("absent.tcl").display().to_string();
    let (code, out) = ran(&[&bad, &missing]);
    assert_eq!(code, 2);
    assert!(
        out.starts_with(&format!("{bad}:1:6: missing close-brace\n")),
        "{out}"
    );
    assert!(out.contains(&format!("{missing}: cannot read: ")), "{out}");

    std::fs::remove_dir_all(&dir).unwrap_or_else(|e| panic!("{e}"));
}

#[test]
fn the_lint_command_is_this_binary_with_the_file_appended() {
    // The lint engine runs checks as commands (`src/lint:V8`); the output
    // is `file:line:col: message` text, not a format it parses.
    assert_eq!(
        lint_cmd(),
        LintCmd {
            argv: vec!["xenolith-tcl-syntax".to_owned()],
            file_arg: FileArg::Append,
            format: Format::Raw,
        }
    );
}

#[test]
fn run_with_no_file_is_a_usage_error() {
    assert_eq!(
        ran(&[]),
        (2, "usage: xenolith-tcl-syntax FILE...\n".to_owned())
    );
}
