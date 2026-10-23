//! Unit tests for `body.rs` (`src:C139`): a recipe's body as just lexes
//! it -- its indent, its lines, their `@`/`-` prefixes and `{{...}}`
//! interpolations, and a `#!` first line.

use super::super::parse;
use crate::recipe::{Justfile, Recipe};

fn parsed(src: &str) -> Justfile {
    parse(src).unwrap_or_else(|e| panic!("{src:?}: {e}"))
}

fn only(src: &str) -> Recipe {
    match parsed(src).recipes.as_slice() {
        [one] => one.clone(),
        other => panic!("{src:?}: expected one recipe, got {other:#?}"),
    }
}

/// Each line's interpolations, as text.
fn holes(src: &str) -> Vec<Vec<String>> {
    only(src)
        .lines
        .iter()
        .map(|l| {
            l.holes
                .iter()
                .map(|h| h.of(src).unwrap_or_default().to_owned())
                .collect()
        })
        .collect()
}

#[test]
fn a_line_spans_its_text_without_indent_or_carriage_return() {
    let src = "a:\n    echo one\r\n\n      echo two\n";
    let lines: Vec<&str> = only(src)
        .lines
        .iter()
        .map(|l| l.span.of(src).unwrap_or_default())
        .collect();
    assert_eq!(lines, ["echo one", "  echo two"]);
}

#[test]
fn a_body_span_starts_at_its_first_line_s_indent_and_ends_after_its_last() {
    let src = "a:\n    echo one\n\n    echo two\n\nb:\n  echo b\n";
    let bodies: Vec<&str> = parsed(src)
        .recipes
        .iter()
        .filter_map(|r| r.body(src).and_then(|s| s.of(src)))
        .collect();
    assert_eq!(bodies, ["    echo one\n\n    echo two", "  echo b"]);
}

#[test]
fn a_shebang_first_line_is_kept_apart_and_starts_the_body() {
    let src = "s:\n  #!/bin/sh\n  echo x\n";
    let recipe = only(src);
    assert_eq!(
        recipe.shebang.map(|l| l.span.of(src).unwrap_or_default().to_owned()),
        Some("#!/bin/sh".to_owned())
    );
    assert_eq!(recipe.lines.len(), 1);
    assert_eq!(
        recipe.body(src).and_then(|s| s.of(src)),
        Some("  #!/bin/sh\n  echo x")
    );
}

#[test]
fn prefixes_are_read_off_the_start_of_the_text() {
    let src = "a x:\n    @-echo {{x}}\n    -@true\n    @quiet\n    -loud\n    plain @not\n";
    let prefixes: Vec<&str> = only(src)
        .lines
        .iter()
        .map(|l| l.prefix.of(src).unwrap_or_default())
        .collect();
    assert_eq!(prefixes, ["@-", "-@", "@", "-", ""]);
}

#[test]
fn interpolations_are_whole_expressions() {
    assert_eq!(
        holes(concat!(
            "a x:\n",
            "    echo {{x}} and {{ x + \"}}\" }} ",
            "{{ if x == \"a\" { \"b\" } else { \"c\" } }}\n",
        )),
        [vec![
            "{{x}}".to_owned(),
            "{{ x + \"}}\" }}".to_owned(),
            "{{ if x == \"a\" { \"b\" } else { \"c\" } }}".to_owned(),
        ]]
    );
}

#[test]
fn four_braces_are_a_literal_and_open_no_hole() {
    // `languages/ci/just:B2`: just prints `x{{y}}` for `x{{{{y}}`.
    assert_eq!(
        holes("a:\n    echo x{{{{y}} {{ z }}\n    echo {{{{ w }}\n    echo {{{{{{v}}\n"),
        [
            vec!["{{ z }}".to_owned()],
            vec![],
            vec!["{{v}}".to_owned()],
        ]
    );
}

#[test]
fn a_format_string_inside_an_interpolation_is_one_hole() {
    assert_eq!(
        holes("hello:\n    echo {{ f\"hi {{name}}\" }}\n"),
        [vec!["{{ f\"hi {{name}}\" }}".to_owned()]]
    );
}

#[test]
fn a_comment_line_in_a_body_is_a_line() {
    let src = "a:\n    # passed to the shell\n    echo\n";
    assert_eq!(only(src).lines.len(), 2);
}

#[test]
fn the_body_ends_at_the_first_unindented_line() {
    let got = parsed("a:\n    one\n\n# top comment\nb:\n    two\n");
    let counts: Vec<usize> = got.recipes.iter().map(|r| r.lines.len()).collect();
    assert_eq!(counts, [1, 1]);
}

#[test]
fn a_broken_body_is_a_parse_error() {
    for src in [
        "a:\n    one\n  two\n",
        "a:\n    echo {{ x\n",
        "a:\n    echo {{ x }\n",
        "a:\n    echo {{ }}\n",
    ] {
        assert!(parse(src).is_err(), "{src:?}");
    }
}
