//! Bodies back into nix strings, step by step
//! (`languages/api/src/lens:V39`, `languages/ci/nix:V170`, `src:C139`).
//!
//! `tests/lens.rs` holds the law over fixtures and vectors; this file
//! pins the exact text each rule writes, so a regression names the rule.

use xenolith_lang_api::{DelimKind, Error, LangId};

use super::{double_quoted, escape, indented, indented_line, lead_line};
use crate::unescape;

// --- escape ------------------------------------------------------------

#[test]
fn escape_writes_each_nix_delimiter_and_refuses_the_rest() {
    assert_eq!(
        escape(&DelimKind::NixIndented, "a\n"),
        Ok("\na\n".to_owned())
    );
    assert_eq!(
        escape(&DelimKind::NixString, "a\"b"),
        Ok("a\\\"b".to_owned())
    );
    assert_eq!(
        escape(&DelimKind::PklMultiline { pounds: 0 }, "a"),
        Err(Error::unsupported(LangId::Nix, "escape"))
    );
}

// --- indented ----------------------------------------------------------

#[test]
fn every_line_goes_behind_the_indent_and_empty_lines_stay_empty() {
    assert_eq!(
        indented("a\n\n  b\n", "    ", "  "),
        "\n    a\n\n      b\n  "
    );
}

#[test]
fn a_body_without_a_final_line_break_runs_into_the_closing_quotes() {
    assert_eq!(indented("a\nb", "  ", "X"), "\n  a\n  b");
}

#[test]
fn an_empty_body_is_a_closing_line_alone() {
    assert_eq!(indented("", "  ", "  "), "\n  ");
    assert_eq!(unescape::indented(&indented("", "  ", "  ")), "");
}

#[test]
fn a_body_indented_throughout_keeps_its_indent_by_one_escaped_space() {
    let raw = indented("  a\n    b\n", "  ", "");
    assert_eq!(raw, "\n  ''\\  a\n      b\n");
    assert_eq!(unescape::indented(&raw), "  a\n    b\n");
}

#[test]
fn a_last_line_of_spaces_keeps_them_by_one_escaped_space() {
    let raw = indented("a\n  ", "", "");
    assert_eq!(raw, "\na\n ''\\ ");
    assert_eq!(unescape::indented(&raw), "a\n  ");
}

// --- lead_line ---------------------------------------------------------

#[test]
fn the_lead_line_is_the_first_non_empty_one_when_all_start_with_a_space() {
    assert_eq!(lead_line(&["", " a", "  b"]), Some(1));
    assert_eq!(lead_line(&[" a", "b"]), None);
    assert_eq!(lead_line(&["\ta", " b"]), None);
    assert_eq!(lead_line(&["", ""]), None);
}

// --- indented_line -----------------------------------------------------

#[test]
fn quote_pairs_dollar_braces_and_carriage_returns_are_escaped() {
    assert_eq!(indented_line("''", false, false, false), "'''");
    assert_eq!(indented_line("${x}", false, false, false), "''${x}");
    assert_eq!(indented_line("$$ $x", false, false, false), "$$ $x");
    assert_eq!(indented_line("a\rb", false, false, false), "a''\\rb");
    assert_eq!(indented_line("\ttab", false, false, false), "\ttab");
}

#[test]
fn a_lone_quote_before_quotes_is_escaped_and_elsewhere_is_not() {
    assert_eq!(indented_line("it's", false, false, false), "it's");
    assert_eq!(indented_line("'${", false, false, false), "''\\'''${");
    assert_eq!(indented_line("'''", false, false, false), "''''");
    assert_eq!(indented_line("'''", false, false, true), "'''''\\'");
    assert_eq!(indented_line("a'", false, false, true), "a''\\'");
    assert_eq!(indented_line("a'", false, false, false), "a'");
}

#[test]
fn lead_and_trailing_spaces_are_escaped_where_asked() {
    assert_eq!(indented_line(" a", true, false, false), "''\\ a");
    assert_eq!(indented_line("  ", false, true, true), " ''\\ ");
    assert_eq!(indented_line(" ", true, true, true), "''\\ ");
}

// --- double_quoted -----------------------------------------------------

#[test]
fn double_quoted_escapes_what_a_nix_string_would_read() {
    assert_eq!(double_quoted(r"a\b"), r"a\\b");
    assert_eq!(double_quoted("say \"hi\""), "say \\\"hi\\\"");
    assert_eq!(double_quoted("${x} $y $${z}"), "\\${x} $y $\\${z}");
    assert_eq!(double_quoted("a\nb\rc\td"), "a\\nb\\rc\td");
}
