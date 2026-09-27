//! The write side's helpers, one rule each (`languages/ci/nix:V170`,
//! `src:C139`).
//!
//! `tests/rewrite.rs` pins what `rewrite` and `inline` write and refuse,
//! and `tests/lens.rs` the laws over every fixture; this file pins the
//! pieces those rest on.

use std::path::Path;

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::{DelimKind, Error, LangId, Span};

use super::{line_indent, needs_parens, path_literal, read_back, refuse, splice};
use crate::parse;

fn root(src: &str) -> SyntaxNode {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

/// The first string node in `src`.
fn first_string(src: &str) -> SyntaxNode {
    root(src)
        .descendants()
        .find(|node| node.kind() == SyntaxKind::NODE_STRING)
        .unwrap_or_else(|| panic!("{src:?}: no string"))
}

#[test]
fn a_refusal_is_unsupported_and_names_what_was_refused() {
    assert_eq!(
        refuse("rewrite of x"),
        Error::Unsupported {
            lang: LangId::Nix,
            operation: "rewrite of x"
        }
    );
}

// --- path_literal ------------------------------------------------------

#[test]
fn a_relative_path_is_written_as_nix_reads_it() {
    let literal = |p: &str| path_literal(Path::new(p));
    assert_eq!(literal("./a/x.sh"), Some("./a/x.sh".to_owned()));
    assert_eq!(literal("../x.sh"), Some("../x.sh".to_owned()));
    assert_eq!(literal("a/x.sh"), Some("./a/x.sh".to_owned()));
    assert_eq!(literal("x-1_2+3.sh"), Some("./x-1_2+3.sh".to_owned()));
}

#[test]
fn a_path_nix_cannot_write_as_a_literal_is_none() {
    for path in [
        "/a/x.sh", "", "a b.sh", "a\"x.sh", "${a}.sh", "a//x.sh", "a/", "~/x.sh",
    ] {
        assert_eq!(path_literal(Path::new(path)), None, "{path:?}");
    }
}

// --- needs_parens ------------------------------------------------------

#[test]
fn slots_looser_than_application_need_no_parentheses() {
    for src in [
        "{ a = ''x''; }",
        "''x'' + y",
        "(''x'')",
        "let a = 1; in ''x''",
        "if c then ''x'' else y",
        "with p; ''x''",
        "assert c; ''x''",
        "a: ''x''",
        "''x''",
        "\"${''x''}\"",
    ] {
        let string = root(src)
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::NODE_STRING)
            .find(|node| node.text() == "''x''")
            .unwrap_or_else(|| panic!("{src:?}: no ''x''"));
        assert!(!needs_parens(&string), "{src:?}");
    }
}

#[test]
fn an_argument_a_list_element_or_a_select_needs_them() {
    for src in [
        "f ''x''",
        "[ ''x'' ]",
        "''x''.a",
        "''x'' ? a",
        "{ a ? ''x'' }: a",
    ] {
        assert!(needs_parens(&first_string(src)), "{src:?}");
    }
}

// --- line_indent -------------------------------------------------------

#[test]
fn the_indent_is_the_leading_spaces_of_the_line_and_stops_at_a_tab() {
    let src = "{\n    a = b;\n  \tc = d;\n}";
    let a = src.find("b;").unwrap_or_default();
    let c = src.find("d;").unwrap_or_default();
    assert_eq!(line_indent(src, a), "    ");
    assert_eq!(line_indent(src, c), "  ");
    assert_eq!(line_indent("x = 1;", 4), "");
}

// --- read_back ---------------------------------------------------------

#[test]
fn read_back_unescapes_the_string_starting_there() {
    let src = "{ a = ''\n  x ''${y}\n''; b = \"q\\\"\"; }";
    let a = src.find("''").unwrap_or_default();
    let b = src.find('"').unwrap_or_default();
    assert_eq!(
        read_back(src, a, &DelimKind::NixIndented),
        Ok("x ${y}\n".into())
    );
    assert_eq!(read_back(src, b, &DelimKind::NixString), Ok("q\"".into()));
}

#[test]
fn read_back_refuses_a_hole_a_wrong_delimiter_or_no_string() {
    let src = "{ a = \"${b}\"; c = 1; }";
    let a = src.find('"').unwrap_or_default();
    let c = src.find('1').unwrap_or_default();
    assert!(read_back(src, a, &DelimKind::NixString).is_err());
    assert!(read_back("{ a = \"x\"; }", 6, &DelimKind::NixIndented).is_err());
    assert!(read_back(src, c, &DelimKind::NixString).is_err());
}

// --- splice ------------------------------------------------------------

#[test]
fn splice_replaces_exactly_the_span() {
    assert_eq!(splice("abcdef", Span::new(2, 4), "XY"), Ok("abXYef".into()));
    assert_eq!(splice("ab", Span::new(1, 5), "x").ok(), None);
}
