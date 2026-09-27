//! Unit tests for `grammar.rs` (`src:C139`): the vendored grammar loads
//! and parses, and the shapes the host walks are the ones it names.

use super::{language, parse};

/// The S-expression of `src`'s tree.
fn tree(src: &str) -> String {
    parse(src)
        .unwrap_or_else(|e| panic!("{e}"))
        .root_node()
        .to_sexp()
}

#[test]
fn the_grammar_is_tcl_at_a_runtime_supported_abi() {
    let lang = language();
    assert_eq!(lang.name(), Some("tcl"));
    assert!(lang.abi_version() >= tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION);
    assert!(lang.abi_version() <= tree_sitter::LANGUAGE_VERSION);
}

#[test]
fn a_command_is_a_name_and_a_word_list() {
    assert_eq!(
        tree("exec sh -c {a | b}\n"),
        "(source_file (command name: (simple_word) arguments: (word_list \
         (simple_word) (simple_word) (braced_word (command name: (simple_word) \
         arguments: (word_list (simple_word) (simple_word)))))))"
    );
}

#[test]
fn a_braced_word_is_parsed_as_a_script_even_where_tcl_reads_data() {
    // What makes the host treat some braces as opaque: shell's `$(…)`
    // is not tcl, and the grammar says so inside the braces.
    let parsed = parse("exec sh -c {ls $(pwd)}\n").unwrap_or_else(|e| panic!("{e}"));
    assert!(parsed.root_node().has_error());
}

#[test]
fn an_empty_file_parses_without_error() {
    let parsed = parse("").unwrap_or_else(|e| panic!("{e}"));
    assert!(!parsed.root_node().has_error());
}

/// Whether `src` parses with no `ERROR` or `MISSING` node.
fn clean(src: &str) -> bool {
    !parse(src)
        .unwrap_or_else(|e| panic!("{e}"))
        .root_node()
        .has_error()
}

#[test]
fn a_dollar_that_starts_no_variable_is_text_in_quotes() {
    // `languages/shells/tcl:B1`: expect's prompt pattern.
    assert!(clean("expect \"$ \"\n"));
    assert!(clean("puts \"cost: $5, $\"\n"));
    assert!(clean("puts \"$a $ ${b} $::c $d(x)\"\n"));
}

#[test]
fn a_semicolon_ends_a_command_mid_line() {
    assert!(clean("puts hi; set x 1\n"));
    assert!(clean("puts \"hi\"; puts {hi}; puts [pwd]; a b;c d\n"));
}

#[test]
fn an_expression_needs_no_spaces_around_its_operators() {
    assert!(clean("set x [expr {1+2}]\n"));
    assert!(clean("expr {1-2*3}\n"));
    assert!(clean("expr {sin(1)+abs(-2)}\n"));
}

#[test]
fn a_leading_sign_is_a_unary_operator() {
    // The patch took the sign out of `number` (`languages/shells/tcl:B1`);
    // `-1` still reads, as `unary_expr` over the number.
    let sexp = tree("expr {-1+2}\n");
    assert!(
        sexp.contains("(binop_expr (unary_expr (number)) (number))"),
        "{sexp}"
    );
    assert!(clean("expr {-1 + -2}\n"));
    assert!(clean("expr {+1}\n"));
    assert!(clean("expr {1e+5 - 2.5e-3}\n"));
}

#[test]
fn a_boolean_word_in_an_expression_is_a_boolean_not_a_function_name() {
    // The function name became an identifier token; it sits below the
    // booleans, so `true` still reads as one (`languages/shells/tcl:B1`).
    assert!(clean("expr {true && false}\n"));
    assert!(clean("expr {max(1,2) + rand() + int($x)}\n"));
}

#[test]
fn an_array_index_attaches_to_its_name() {
    // `(` never concatenates a word, so `$y(z)` is one substitution
    // wherever it stands (`languages/shells/tcl:B1`).
    assert!(clean("set x $y(z)\n"));
    assert!(clean("set a(x) $b($k)\n"));
    assert!(clean("puts foo$a(b)\n"));
}

#[test]
fn a_close_bracket_in_quotes_is_text() {
    assert!(clean("puts \"a]b\"\n"));
    assert!(clean("puts [format \"%d]\" 1]\n"));
}

#[test]
fn the_control_words_tcl_allows_read() {
    assert!(clean("if {1} then {puts a} elseif {0} then {puts b}\n"));
    assert!(clean("catch {a} result options\n"));
    assert!(clean(
        "try {a} on error {m o} {b} trap {POSIX ENOENT} {m} {c} finally {d}\n"
    ));
    assert!(clean("try {a} on ok {r} {b}\n"));
}

#[test]
fn some_valid_tcl_is_still_rejected() {
    // Open in `languages/shells/tcl:B1`: a bare word with an index as
    // `set`'s value. The host fails such a file whole (`languages:V78`).
    assert!(!clean("set x a(b)\n"));
}
