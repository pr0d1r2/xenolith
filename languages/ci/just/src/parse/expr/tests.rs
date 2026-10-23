//! Unit tests for `expr.rs` (`src:C139`): just's expression grammar,
//! through the assignments that hold expressions.

use super::super::parse;

fn clean(expression: &str) -> bool {
    parse(&format!("x := {expression}\n")).is_ok()
}

#[test]
fn every_expression_form_reads() {
    for expression in [
        "\"a\"",
        "'a'",
        "x'~/bin'",
        "f\"hi {{name}}\"",
        "`date`",
        "name",
        "a + b + \"c\"",
        "a ++ b",
        "a / b",
        "/ \"root\"",
        "a && b || c",
        "a == b",
        "a != b",
        "a =~ 'x.*'",
        "a !~ 'x'",
        "!a",
        "(a + b)",
        "[a, b, ]",
        "[]",
        "env('HOME', 'x')",
        "uuid()",
        "assert(name == \"world\", \"unexpected\")",
        "if a == b { \"x\" } else { \"y\" }",
        "if a { \"x\" } else if b { \"y\" } else { \"z\" }",
        "if a { \"x\" }",
        "(\n  a +\n  b\n)",
    ] {
        assert!(clean(expression), "{expression:?}");
    }
}

#[test]
fn what_just_rejects_reads_as_an_error() {
    for expression in [
        "a +",
        "(a",
        "[a b]",
        "f(",
        "if a { \"x\" } else",
        "if a \"x\"",
        "a ==",
        ")",
        "assert(a b)",
    ] {
        assert!(!clean(expression), "{expression:?}");
    }
}
