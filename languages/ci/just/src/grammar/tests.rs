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

/// just 1.51 syntax the grammar at 0.2.0 rejected, one construct a file
/// (`languages/ci/just:B3`). Each is valid to `just 1.51.0`.
const JUST_1_51: &[&str] = &[
    "a := f\"hello {{b}}\"\n",
    "a := f'{{{{b}} is {{b + \"!\"}}'\n",
    "a := f\"\"\"\n  x {{b}}\n\"\"\"\n",
    "a := f'''\n  x {{b}}\n'''\n",
    "a := x\"~/bin\"\n",
    "a := x'$HOME'\n",
    "a := x\"\"\"${HOME}\"\"\"\n",
    "a := x'''~/x'''\n",
    "set unstable\na := \"b\" && \"c\"\n",
    "set unstable\na := \"\" || \"c\"\n",
    "a := assert(\"b\" == \"b\", \"no\")\n",
    "eager a := \"b\"\n",
    "[private]\na := \"b\"\n",
    "[private]\nexport a := \"b\"\n",
    "unexport A\n",
    "[arg(\"n\", pattern='\\d+')]\nb n:\n    echo {{n}}\n",
    "b:\n    echo {{ f\"hi {{c}}\" }}\n",
    "set shell := [x\"bash\", \"-cu\"]\n",
];

#[test]
fn just_1_51_syntax_parses_without_error() {
    for src in JUST_1_51 {
        let tree = parse(src);
        let root = tree.root_node();
        assert!(!root.has_error(), "{src:?}\n{}", root.to_sexp());
    }
}

#[test]
fn a_format_string_holds_its_interpolations_in_one_string() {
    let tree = parse("a := f\"x {{b}} y {{c}}\"\n");
    let sexp = tree.root_node().to_sexp();
    assert_eq!(sexp.matches("(string").count(), 1, "{sexp}");
    assert_eq!(sexp.matches("(interpolation").count(), 2, "{sexp}");
}

#[test]
fn names_like_the_new_keywords_stay_variables() {
    // `eager` and `unexport` read as identifiers, so a variable of that
    // name -- valid just -- still parses (`languages/ci/just:B3`).
    for src in ["eager := \"a\"\n", "unexport := \"a\"\nb := unexport\n"] {
        let tree = parse(src);
        assert!(!tree.root_node().has_error(), "{src:?}");
    }
}

#[test]
fn broken_input_is_an_error_tree_not_a_panic() {
    let tree = parse("set shell := bash\n");
    assert!(tree.root_node().has_error());
}
