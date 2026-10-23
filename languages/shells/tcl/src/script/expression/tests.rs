//! Unit tests for `expression.rs` (`src:C139`): the substitutions an
//! expression runs, and the openers that never close.

use xenolith_lang_api::{Error, Span};

use super::super::{Part, WordKind, parse};
use super::expression;

fn of(span: Span, src: &str) -> &str {
    span.of(src)
        .unwrap_or_else(|| panic!("{span:?} outside {src:?}"))
}

/// The braced condition of the `if` in `src`.
fn condition(src: &str) -> Span {
    let script = parse(src).unwrap_or_else(|e| panic!("{e}"));
    match script
        .commands
        .first()
        .and_then(|c| c.words.get(1))
        .map(|w| &w.kind)
    {
        Some(WordKind::Braced { body, .. }) => *body,
        other => panic!("{src:?}: no braced condition, got {other:?}"),
    }
}

#[test]
fn an_expression_yields_its_substitutions_and_skips_braced_strings() {
    let src = "if {[file exists $f] && {[x]} ne \"[y]\"} {}\n";
    let body = condition(src);
    let parts = expression(src, body).unwrap_or_else(|e| panic!("{e}"));
    let subs: Vec<&str> = parts
        .iter()
        .map(|p| match p {
            Part::Var { span, .. } | Part::Cmd { span, .. } => of(*span, src),
            Part::Text(s) | Part::Escape(s) => of(*s, src),
        })
        .collect();
    assert_eq!(subs, ["[file exists $f]", "[y]"]);
}

#[test]
fn an_expression_reads_escapes_quoted_text_and_a_lone_dollar() {
    let src = "expr {\\[ + $ + \"a $x b\" + $ns::y}";
    let body = Span::new(6, src.len() - 1);
    let parts = expression(src, body).unwrap_or_else(|e| panic!("{e}"));
    let subs: Vec<&str> = parts
        .iter()
        .filter_map(|p| match p {
            Part::Var { span, .. } => Some(of(*span, src)),
            _ => None,
        })
        .collect();
    assert_eq!(subs, ["$x", "$ns::y"]);
}

#[test]
fn an_expression_that_never_closes_is_a_parse_error() {
    for (src, message) in [("{a", "missing close-brace"), ("\"a", "missing \"")] {
        match expression(src, Span::new(0, src.len())) {
            Err(Error::Parse { message: m, .. }) => assert!(m.contains(message), "{m}"),
            other => panic!("{src:?}: expected a parse error, got {other:?}"),
        }
    }
}
