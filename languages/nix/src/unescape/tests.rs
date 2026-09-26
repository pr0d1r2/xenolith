//! Nix string bodies as the guest reads them, step by step
//! (`languages/api/src/lens:V39`, `languages/nix:T158`, `src:C139`).
//!
//! `tests/host.rs` pins what `NixHost::unescape` answers; this file pins
//! each step behind it -- the opening line, the lexer, the indent and
//! the strip -- so a regression names the step that broke.

use xenolith_lang_api::{DelimKind, Error, LangId};

use super::{
    Part, double_quoted, drop_opening_line, indented, lex_indented, min_indent, strip_indent,
    unescape,
};

fn text(s: &str) -> Part {
    Part::Text(s.to_owned())
}

fn escaped(s: &str) -> Part {
    Part::Escaped(s.to_owned())
}

// --- drop_opening_line -------------------------------------------------

#[test]
fn an_opening_line_of_spaces_is_not_content() {
    assert_eq!(drop_opening_line("\n  a"), "  a");
    assert_eq!(drop_opening_line("   \n  a"), "  a");
}

#[test]
fn an_opening_line_with_anything_else_stays() {
    assert_eq!(drop_opening_line("  a\n"), "  a\n");
    assert_eq!(drop_opening_line("\t\na"), "\t\na");
    assert_eq!(drop_opening_line(""), "");
    assert_eq!(drop_opening_line("  "), "  ");
}

// --- lex_indented ------------------------------------------------------

#[test]
fn lex_splits_escapes_from_text() {
    assert_eq!(
        lex_indented("a''$b'''c''\\nd"),
        vec![
            text("a"),
            escaped("$"),
            text("b"),
            escaped("''"),
            text("c"),
            escaped("\n"),
            text("d"),
        ]
    );
}

#[test]
fn lex_decodes_each_backslash_escape() {
    assert_eq!(lex_indented("''\\t"), vec![escaped("\t")]);
    assert_eq!(lex_indented("''\\r"), vec![escaped("\r")]);
    assert_eq!(lex_indented("''\\x"), vec![escaped("x")]);
    assert_eq!(lex_indented("''\\é"), vec![escaped("é")]);
    assert_eq!(lex_indented("''\\\n"), vec![escaped("\n")]);
}

#[test]
fn lex_keeps_what_is_not_an_escape_as_text() {
    // A lone quote, `$${`, a dangling `''\` and `''` before anything else
    // (which only a hole's own nix could hold) are plain text.
    assert_eq!(lex_indented("it's $${x}"), vec![text("it's $${x}")]);
    assert_eq!(lex_indented("a''\\"), vec![text("a''\\")]);
    assert_eq!(lex_indented("a''b"), vec![text("a''b")]);
    assert!(lex_indented("").is_empty());
}

// --- min_indent --------------------------------------------------------

#[test]
fn min_indent_is_the_least_leading_spaces_of_a_content_line() {
    assert_eq!(min_indent(&[text("    a\n  b\n      c")]), Some(2));
    assert_eq!(min_indent(&[text("a\n    b")]), Some(0));
}

#[test]
fn min_indent_skips_lines_of_spaces_only() {
    assert_eq!(min_indent(&[text("    a\n\n \n    b\n  ")]), Some(4));
}

#[test]
fn min_indent_counts_an_escape_or_a_tab_as_content() {
    assert_eq!(
        min_indent(&[text("  "), escaped("$"), text("\n    b")]),
        Some(2)
    );
    assert_eq!(min_indent(&[text("    a\n\tb")]), Some(0));
}

#[test]
fn min_indent_of_nothing_but_spaces_is_none() {
    assert_eq!(min_indent(&[text("  \n   ")]), None);
    assert_eq!(min_indent(&[]), None);
}

// --- strip_indent ------------------------------------------------------

#[test]
fn strip_takes_the_indent_off_every_line() {
    assert_eq!(
        strip_indent(&[text("    a\n      b\n    c")], Some(4)),
        "a\n  b\nc"
    );
}

#[test]
fn strip_empties_a_short_line_of_spaces() {
    assert_eq!(
        strip_indent(&[text("    a\n  \n      \n    b")], Some(4)),
        "a\n\n  \nb"
    );
}

#[test]
fn strip_without_a_content_line_drops_every_leading_space() {
    assert_eq!(strip_indent(&[text("   \n  ")], None), "\n");
}

#[test]
fn strip_drops_a_last_line_of_spaces() {
    assert_eq!(strip_indent(&[text("  a\n     ")], Some(2)), "a\n");
    // Only when there is a line break before it.
    assert_eq!(strip_indent(&[text("a   ")], Some(0)), "a   ");
}

#[test]
fn strip_reads_an_escape_like_text() {
    // Nix strips escaped characters in the same pass as text: `''$` at
    // a line's start ends its indent, and is emitted.
    assert_eq!(
        strip_indent(&[text("  "), escaped("$a"), text("\n  b")], Some(2)),
        "$a\nb"
    );
    // The last-line rule looks inside the LAST part only.
    assert_eq!(
        strip_indent(&[text("a\n  "), escaped("x")], Some(0)),
        "a\n  x"
    );
    assert_eq!(strip_indent(&[text("a\n"), escaped(" ")], Some(0)), "a\n ");
}

// --- indented / double_quoted / unescape -------------------------------

#[test]
fn indented_is_the_steps_in_order() {
    assert_eq!(
        indented("\n    cat <<EOF\n    ''${x}\n    EOF\n  "),
        "cat <<EOF\n${x}\nEOF\n"
    );
}

#[test]
fn double_quoted_decodes_backslash_escapes_only() {
    assert_eq!(double_quoted(r"a\nb\tc\rd"), "a\nb\tc\rd");
    assert_eq!(double_quoted(r#"\"\\\$\{"#), "\"\\${");
    assert_eq!(double_quoted(r"\q"), "q");
    // A dangling backslash is kept.
    assert_eq!(double_quoted("a\\"), "a\\");
    assert_eq!(double_quoted("  a\n  b"), "  a\n  b");
}

#[test]
fn unescape_picks_the_rules_by_delimiter() {
    assert_eq!(
        unescape(&DelimKind::NixIndented, "\n  a\n"),
        Ok("a\n".to_owned())
    );
    assert_eq!(
        unescape(&DelimKind::NixString, r"a\n"),
        Ok("a\n".to_owned())
    );
    assert_eq!(
        unescape(&DelimKind::JustRecipe, "a"),
        Err(Error::unsupported(LangId::Nix, "unescape"))
    );
}
