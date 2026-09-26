//! The load finder's helpers one by one (`languages/nix:V53`,
//! `src:C139`): which callee names are load calls, which arguments are
//! extract paths, and the sorted walk over a tree.

use std::path::PathBuf;

use rnix::{Root, SyntaxKind, SyntaxNode};
use xenolith_lang_api::LangId;

use super::{dotted, is_load_call, load, loads, relative_path};

fn tree(src: &str) -> SyntaxNode {
    let parsed = Root::parse(src);
    assert!(parsed.errors().is_empty(), "{src:?}: {:?}", parsed.errors());
    parsed.syntax()
}

/// The first node of `kind` in `src`.
fn first(src: &str, kind: SyntaxKind) -> SyntaxNode {
    tree(src)
        .descendants()
        .find(|n| n.kind() == kind)
        .unwrap_or_else(|| panic!("{src:?}: no {kind:?}"))
}

fn name(parts: &[&str]) -> Vec<String> {
    parts.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn read_file_is_matched_whole() {
    assert!(is_load_call(&name(&["builtins", "readFile"])));
    assert!(!is_load_call(&name(&["readFile"])));
    assert!(!is_load_call(&name(&["my", "builtins", "readFile"])));
    assert!(!is_load_call(&name(&["builtins", "readDir"])));
}

#[test]
fn read_without_strict_is_matched_by_its_tail() {
    assert!(is_load_call(&name(&[
        "nix-shebang",
        "lib",
        "readWithoutStrict"
    ])));
    assert!(is_load_call(&name(&[
        "inputs",
        "nix-shebang",
        "lib",
        "readWithoutStrict"
    ])));
    assert!(!is_load_call(&name(&["lib", "readWithoutStrict"])));
    assert!(!is_load_call(&name(&["pkgs", "lib", "readWithoutStrict"])));
    assert!(!is_load_call(&[]));
}

#[test]
fn dotted_reads_plain_names_and_selects_only() {
    let callee = |src: &str| {
        let apply = first(src, SyntaxKind::NODE_APPLY);
        apply.first_child().and_then(|f| dotted(&f))
    };
    assert_eq!(callee("f ./x"), Some(name(&["f"])));
    assert_eq!(
        callee("builtins.readFile ./x"),
        Some(name(&["builtins", "readFile"]))
    );
    assert_eq!(
        callee("inputs.nix-shebang.lib.readWithoutStrict ./x"),
        Some(name(&["inputs", "nix-shebang", "lib", "readWithoutStrict"]))
    );
    assert_eq!(callee("(f x).y ./x"), None);
    assert_eq!(callee("a.${b} ./x"), None);
    assert_eq!(callee("a.\"b\" ./x"), None);
}

#[test]
fn only_a_relative_literal_path_is_an_extract_path() {
    let path = |src: &str| {
        let apply = first(src, SyntaxKind::NODE_APPLY);
        apply.children().nth(1).and_then(|a| relative_path(&a))
    };
    assert_eq!(path("f ./a/x.sh"), Some(PathBuf::from("./a/x.sh")));
    assert_eq!(path("f ../x.sh"), Some(PathBuf::from("../x.sh")));
    assert_eq!(path("f /etc/x.sh"), None);
    assert_eq!(path("f ~/x.sh"), None);
    assert_eq!(path("f <nixpkgs/x.sh>"), None);
    assert_eq!(path("f ./a/${b}.sh"), None);
    assert_eq!(path("f \"./x.sh\""), None);
}

#[test]
fn a_load_needs_a_shell_extension_and_spans_the_call() {
    let one = |src: &str| load(&first(src, SyntaxKind::NODE_APPLY));
    let found = one("builtins.readFile ./x.zsh").unwrap_or_else(|| panic!("no load"));
    assert_eq!(found.path, PathBuf::from("./x.zsh"));
    assert_eq!(found.guest, LangId::Shell);
    assert_eq!((found.span.start, found.span.end), (0, 25));
    assert!(one("builtins.readFile ./x.bash").is_some());
    assert!(one("builtins.readFile ./x.conf").is_none());
    assert!(one("builtins.readFile ./Makefile").is_none());
    assert!(one("builtins.readDir ./x.sh").is_none());
}

#[test]
fn loads_come_back_sorted_and_nested_ones_count() {
    let src = "{ b = builtins.readFile ./b.sh; a = f (builtins.readFile ./a.sh); }";
    let found = loads(&tree(src));
    let paths: Vec<PathBuf> = found.iter().map(|l| l.path.clone()).collect();
    assert_eq!(paths, [PathBuf::from("./b.sh"), PathBuf::from("./a.sh")]);
    let starts: Vec<usize> = found.iter().map(|l| l.span.start).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted);
    assert!(loads(&tree("{ }")).is_empty());
}
