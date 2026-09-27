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
