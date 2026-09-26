//! Unit tests for pkl multi-line strings, both directions (`src:C139`).
//!
//! `tests/host.rs` checks `unescape` and `multiline` on real hk sites.
//! These take each piece alone: the delimiter-layout ranges
//! (`indentation`), escape decoding (`decode`), the pound search
//! (`pounds_for`), the tree helpers, and the round trip the module
//! promises -- `unescape(multiline(body)) == body`.

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Result, Span};

use super::{decode, find_kind, holes_in, indentation, multiline, pounds_for, unescape};
use crate::grammar;

fn delim(kind: DelimKind) -> Delim {
    Delim {
        kind,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    }
}

fn pkl(pounds: usize) -> Delim {
    delim(DelimKind::PklMultiline { pounds })
}

fn ok(pounds: usize, raw: &str) -> String {
    unescape(&pkl(pounds), raw).unwrap_or_else(|e| panic!("unescape {raw:?} failed: {e}"))
}

fn is_parse_error<T: std::fmt::Debug>(result: &Result<T>) -> bool {
    matches!(
        result,
        Err(Error::Parse {
            lang: LangId::Pkl,
            ..
        })
    )
}

fn tree(src: &str) -> Tree {
    let mut parser = Parser::new();
    if let Err(e) = parser.set_language(&grammar::language()) {
        panic!("language: {e}");
    }
    parser
        .parse(src, None)
        .unwrap_or_else(|| panic!("no tree for {src:?}"))
}

fn literal(tree: &Tree) -> Node<'_> {
    find_kind(tree.root_node(), "mlStringLiteralExpr")
        .unwrap_or_else(|| panic!("no multi-line string in {}", tree.root_node().to_sexp()))
}

/// The raw text between the delimiters of a literal `multiline` wrote.
fn inside(literal: &str) -> (usize, &str) {
    let pounds = literal.chars().take_while(|&ch| ch == '#').count();
    let guard = "#".repeat(pounds);
    let raw = literal
        .strip_prefix(&format!("{guard}\"\"\""))
        .and_then(|rest| rest.strip_suffix(&format!("\"\"\"{guard}")))
        .unwrap_or_else(|| panic!("not a delimited literal: {literal:?}"));
    (pounds, raw)
}

// --- unescape ------------------------------------------------------------------

#[test]
fn unescape_refuses_a_delimiter_that_is_not_pkl() {
    assert_eq!(
        unescape(&delim(DelimKind::NixIndented), "\n  x\n  "),
        Err(Error::unsupported(LangId::Pkl, "unescape"))
    );
    assert_eq!(
        unescape(&delim(DelimKind::RustRawString { pounds: 1 }), "\n  x\n  "),
        Err(Error::unsupported(LangId::Pkl, "unescape"))
    );
}

#[test]
fn unescape_strips_the_layout_and_the_closing_indent() {
    assert_eq!(ok(0, "\n  a\n    b\n  "), "a\n  b");
    assert_eq!(ok(0, "\na\nb\n"), "a\nb");
    assert_eq!(ok(0, "\n\t\tx\n\t\t"), "x");
}

#[test]
fn an_empty_body_evaluates_to_the_empty_string() {
    assert_eq!(ok(0, "\n"), "");
    assert_eq!(ok(0, "\n    "), "");
}

#[test]
fn blank_lines_survive_and_short_whitespace_lines_become_empty() {
    assert_eq!(ok(0, "\n  a\n\n  b\n  "), "a\n\nb");
    assert_eq!(ok(0, "\n    a\n  \n    b\n    "), "a\n\nb");
    // Longer than the indent: only the indent goes.
    assert_eq!(ok(0, "\n  a\n     \n  "), "a\n   ");
}

#[test]
fn a_trailing_empty_line_is_a_trailing_newline() {
    assert_eq!(ok(0, "\n  a\n\n  "), "a\n");
}

#[test]
fn escapes_are_decoded_in_a_plain_string() {
    assert_eq!(
        ok(0, "\n  a\\tb\\nc\\rd\\\"e\\\\f\\u{41}\n  "),
        "a\tb\nc\rd\"e\\fA"
    );
}

#[test]
fn a_guarded_string_keeps_bare_backslashes_and_decodes_guarded_ones() {
    assert_eq!(ok(1, "\n  sed 's/\\t/ /'\n  "), "sed 's/\\t/ /'");
    assert_eq!(ok(1, "\n  a\\#tb\n  "), "a\tb");
    assert_eq!(ok(2, "\n  a\\#tb\\##nc\n  "), "a\\#tb\nc");
}

#[test]
fn interpolations_are_kept_verbatim() {
    assert_eq!(ok(0, "\n  echo \\(x) \\(y.z)\n  "), "echo \\(x) \\(y.z)");
    assert_eq!(ok(1, "\n  \\(lit) \\#(real)\n  "), "\\(lit) \\#(real)");
}

#[test]
fn a_newline_inside_an_interpolation_is_not_a_string_line() {
    // The interpolation's second line sits left of the closing indent;
    // as a string line that would be an error, as pkl expression text it
    // is fine and is kept as written.
    assert_eq!(ok(0, "\n    a \\(f(\n1)) b\n    "), "a \\(f(\n1)) b");
}

#[test]
fn a_crlf_body_evaluates_as_its_lf_twin() {
    // Pkl reads `\r\n`, `\r` and `\n` alike as one newline in a string
    // and each evaluates to `\n` (`languages/pkl:B1`).
    for lf in [
        "\n  a\n    b\n  ",
        "\na\nb\n",
        "\n",
        "\n    ",
        "\n  a\n\n  b\n  ",
        "\n    a\n  \n    b\n    ",
        "\n  a\n\n  ",
    ] {
        let want = ok(0, lf);
        assert_eq!(ok(0, &lf.replace('\n', "\r\n")), want, "{lf:?} as CRLF");
        assert_eq!(ok(0, &lf.replace('\n', "\r")), want, "{lf:?} as CR");
    }
    // `\n\r` is two line breaks, not one.
    assert_eq!(ok(1, "\r\n  a\\#tb\r\n  c\n\r  "), "a\tb\nc\n");
    // Inside a hole the line break is pkl expression text, kept as is.
    assert_eq!(
        ok(0, "\r\n    a \\(f(\r\n1)) b\r\n    "),
        "a \\(f(\r\n1)) b"
    );
}

#[test]
fn an_escaped_cr_stays_a_cr() {
    assert_eq!(ok(0, "\r\n  a\\r\r\n  "), "a\r");
}

#[test]
fn a_crlf_layout_is_still_checked() {
    assert!(is_parse_error(&unescape(&pkl(0), "a\r\n  ")));
    assert!(is_parse_error(&unescape(&pkl(0), "\r\n  a\r\n  b")));
    assert!(is_parse_error(&unescape(
        &pkl(0),
        "\r\n    a\r\n  b\r\n    "
    )));
}

#[test]
fn unescape_refuses_what_pkl_would_not_parse() {
    assert!(is_parse_error(&unescape(&pkl(0), "\n  a \"\"\" b\n  ")));
    assert!(is_parse_error(&unescape(&pkl(0), "\n  \\(\n  ")));
    assert!(is_parse_error(&unescape(&pkl(0), "\n  \\q\n  ")));
}

#[test]
fn unescape_refuses_content_on_the_delimiter_lines() {
    assert!(is_parse_error(&unescape(&pkl(0), "a\n  ")));
    assert!(is_parse_error(&unescape(&pkl(0), "\n  a\n  b")));
    assert!(is_parse_error(&unescape(&pkl(0), "")));
}

#[test]
fn unescape_refuses_a_line_left_of_the_closing_indent() {
    assert!(is_parse_error(&unescape(&pkl(0), "\n    a\n  b\n    ")));
}

// --- multiline ------------------------------------------------------------------

#[test]
fn multiline_indents_content_and_leaves_empty_lines_empty() {
    assert_eq!(multiline("a\n\nb", "  "), "\"\"\"\n  a\n\n  b\n  \"\"\"");
}

#[test]
fn multiline_turns_a_trailing_newline_into_an_empty_last_line() {
    assert_eq!(multiline("a\n", "  "), "\"\"\"\n  a\n\n  \"\"\"");
}

#[test]
fn multiline_of_nothing_is_an_empty_literal() {
    assert_eq!(multiline("", "  "), "\"\"\"\n\n  \"\"\"");
    assert_eq!(multiline("x", ""), "\"\"\"\nx\n\"\"\"");
}

#[test]
fn multiline_guards_a_body_that_needs_it() {
    assert_eq!(multiline("a\\tb", " "), "#\"\"\"\n a\\tb\n \"\"\"#");
    assert_eq!(multiline("q\"\"\"q", " "), "#\"\"\"\n q\"\"\"q\n \"\"\"#");
    assert_eq!(multiline("\\#t", " "), "##\"\"\"\n \\#t\n \"\"\"##");
}

#[test]
fn multiline_then_unescape_is_the_identity() {
    for body in [
        "",
        "\n",
        "\n\n",
        "echo hi",
        "echo hi\n",
        "a\n\nb\n",
        "  leading spaces\n\tand a tab",
        "sed 's/\\t/ /g' \"$f\"",
        "printf '%s\\n' \"$x\"",
        "a \\# b \\## c",
        "say \"\"\" loudly",
        "say \"\"\"# loudly \\#",
        "\\(not a hole)",
        "unicode: zażółć ✓",
    ] {
        let written = multiline(body, "      ");
        let (pounds, raw) = inside(&written);
        assert_eq!(
            unescape(&pkl(pounds), raw),
            Ok(body.to_owned()),
            "{body:?} written as {written:?}"
        );
    }
}

// --- pounds_for ------------------------------------------------------------------

#[test]
fn pounds_for_picks_the_fewest_that_let_the_body_in() {
    for (body, pounds) in [
        ("", 0),
        ("plain", 0),
        ("\"\" two quotes", 0),
        ("a\\b", 1),
        ("\"\"\"", 1),
        ("\\#", 2),
        ("\"\"\"#", 2),
        ("\\## and \"\"\"#", 3),
        ("\\\\", 1),
    ] {
        assert_eq!(pounds_for(body), pounds, "{body:?}");
    }
}

#[test]
fn pounds_for_always_finds_an_answer() {
    // Every `#` count up to the body's length is taken; the search range
    // still has one more.
    assert_eq!(pounds_for("\\"), 1);
    assert_eq!(pounds_for("\\#"), 2);
    assert_eq!(pounds_for("\\##"), 3);
}

// --- find_kind, holes_in ---------------------------------------------------------

#[test]
fn find_kind_returns_the_node_itself_the_first_descendant_or_none() {
    let tree = tree("a = \"\"\"\n  x\n  \"\"\"\nb = \"\"\"\n  y\n  \"\"\"\n");
    let root = tree.root_node();
    assert_eq!(find_kind(root, "module").map(|n| n.id()), Some(root.id()));
    let first = find_kind(root, "mlStringLiteralExpr")
        .unwrap_or_else(|| panic!("no literal in {}", root.to_sexp()));
    assert_eq!(first.start_byte(), 4);
    assert!(find_kind(root, "intLiteralExpr").is_none());
}

#[test]
fn holes_in_is_relative_to_the_shift() {
    let src = "x = \"\"\"\n  \\(a) b \\(c)\n  \"\"\"\n";
    let tree = tree(src);
    let prefix = "x = \"\"\"".len();
    let holes = holes_in(literal(&tree), prefix);
    let raw = src.get(prefix..).unwrap_or_default();
    let texts: Vec<Option<&str>> = holes.iter().map(|&(from, to)| raw.get(from..to)).collect();
    assert_eq!(texts, [Some("\\(a)"), Some("\\(c)")]);
}

#[test]
fn holes_in_a_literal_without_interpolation_is_empty() {
    let tree = tree("x = \"\"\"\n  plain\n  \"\"\"\n");
    assert!(holes_in(literal(&tree), 0).is_empty());
}

// --- indentation ------------------------------------------------------------------

#[test]
fn indentation_names_every_layout_range() {
    // Opening newline, closing line, and the indent of each content line.
    assert_eq!(
        indentation("\n  a\n  b\n  ", &[]),
        Ok(vec![(0, 1), (8, 11), (1, 3), (5, 7)])
    );
}

#[test]
fn indentation_deletes_a_short_whitespace_line_whole() {
    assert_eq!(
        indentation("\n    a\n  \n    ", &[]),
        Ok(vec![(0, 1), (9, 14), (1, 5), (7, 9)])
    );
    assert_eq!(
        indentation("\n  a\n\n  ", &[]),
        Ok(vec![(0, 1), (5, 8), (1, 3), (5, 5)])
    );
}

#[test]
fn indentation_skips_newlines_inside_holes() {
    let raw = "\n  \\(f(\n1))\n  ";
    // The hole covers `\(f(\n1))`, bytes 3..11.
    assert_eq!(
        indentation(raw, &[(3, 11)]),
        Ok(vec![(0, 1), (11, 14), (1, 3)])
    );
    assert!(is_parse_error(&indentation(raw, &[])));
}

#[test]
fn indentation_ranges_cover_a_crlf_whole() {
    // Host bytes: the opening and closing ranges take both bytes of a
    // `\r\n`, an indent starts after its `\n`.
    assert_eq!(
        indentation("\r\n  a\r\n  ", &[]),
        Ok(vec![(0, 2), (5, 9), (2, 4)])
    );
}

#[test]
fn indentation_refuses_a_malformed_layout() {
    assert!(is_parse_error(&indentation("", &[])));
    assert!(is_parse_error(&indentation("x\n", &[])));
    assert!(is_parse_error(&indentation("\n  a\n  b", &[])));
    assert!(is_parse_error(&indentation("\n a\n  ", &[])));
}

// --- decode --------------------------------------------------------------------------

#[test]
fn decode_knows_every_simple_escape() {
    for (text, ch) in [
        ("\\n", '\n'),
        ("\\t", '\t'),
        ("\\r", '\r'),
        ("\\\"", '"'),
        ("\\\\", '\\'),
    ] {
        assert_eq!(decode(text, 0), Ok(ch), "{text:?}");
    }
}

#[test]
fn decode_reads_unicode_escapes() {
    assert_eq!(decode("\\u{41}", 0), Ok('A'));
    assert_eq!(decode("\\u{0}", 0), Ok('\0'));
    assert_eq!(decode("\\u{1F600}", 0), Ok('\u{1F600}'));
    assert_eq!(decode("\\##u{7a}", 2), Ok('z'));
}

#[test]
fn decode_skips_the_guard() {
    assert_eq!(decode("\\#n", 1), Ok('\n'));
    assert_eq!(decode("\\###t", 3), Ok('\t'));
}

#[test]
fn decode_refuses_what_is_not_an_escape() {
    for (text, pounds) in [
        ("\\q", 0),
        ("n", 0),
        ("", 0),
        ("\\", 0),
        ("\\", 1),
        ("\\u{}", 0),
        ("\\u{zz}", 0),
        ("\\u{110000}", 0),
        ("\\u{D800}", 0),
        ("\\u{41", 0),
        ("\\é", 1),
    ] {
        assert!(is_parse_error(&decode(text, pounds)), "{text:?} / {pounds}");
    }
}
