//! Unit tests for the grammar shim (`src:C139`, `languages:V121`).
//!
//! The vendored C is ABI 15 while the runtime is tree-sitter 0.27; these
//! pin that the runtime accepts it and that it reads a justfile as one.

use tree_sitter::{Parser, Tree};

use super::language;

fn parse(src: &str) -> Tree {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .unwrap_or_else(|e| panic!("the runtime refused the vendored grammar: {e}"));
    parser
        .parse(src, None)
        .unwrap_or_else(|| panic!("no tree for {src:?}"))
}

#[test]
fn the_runtime_loads_the_vendored_abi() {
    let abi = language().abi_version();
    assert_eq!(abi, 15, "UPSTREAM records ABI 15");
    assert!(abi <= tree_sitter::LANGUAGE_VERSION);
    assert!(abi >= tree_sitter::MIN_COMPATIBLE_LANGUAGE_VERSION);
}

#[test]
fn a_justfile_parses_to_recipes_without_error() {
    let tree = parse("build:\n    cargo build\n");
    let root = tree.root_node();
    assert_eq!(root.kind(), "source_file");
    assert!(!root.has_error(), "{}", root.to_sexp());
    let recipe = root.named_child(0).map(|n| n.kind());
    assert_eq!(recipe, Some("recipe"));
}

#[test]
fn broken_input_is_an_error_tree_not_a_panic() {
    let tree = parse("set shell := bash\n");
    assert!(tree.root_node().has_error());
}
