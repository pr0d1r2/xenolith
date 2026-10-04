//! Unit tests for `site` (`src:C139`): the one function the site types
//! carry, [`DelimKind::runs_line_by_line`] (`languages/api/src/site` §I).

use super::DelimKind;

/// Every delimiter kind, one of each; the `match` stops compiling when a
/// kind is added, which is when its answer has to be decided.
fn every_kind() -> Vec<DelimKind> {
    let kinds = vec![
        DelimKind::NixIndented,
        DelimKind::NixString,
        DelimKind::Heredoc {
            tag: "EOF".into(),
            quoted: false,
            strip_indent: true,
        },
        DelimKind::PklMultiline { pounds: 0 },
        DelimKind::YamlBlock {
            literal: true,
            chomp: None,
        },
        DelimKind::HtmlElement {
            tag: "script".into(),
        },
        DelimKind::RustRawString { pounds: 1 },
        DelimKind::RubyHeredoc {
            tag: "SQL".into(),
            squiggly: true,
        },
        DelimKind::ArgvString,
        DelimKind::JustRecipe,
        DelimKind::JustShebangRecipe,
    ];
    for kind in &kinds {
        match kind {
            DelimKind::NixIndented
            | DelimKind::NixString
            | DelimKind::Heredoc { .. }
            | DelimKind::PklMultiline { .. }
            | DelimKind::YamlBlock { .. }
            | DelimKind::HtmlElement { .. }
            | DelimKind::RustRawString { .. }
            | DelimKind::RubyHeredoc { .. }
            | DelimKind::ArgvString
            | DelimKind::JustRecipe
            | DelimKind::JustShebangRecipe => {}
        }
    }
    kinds
}

#[test]
fn a_just_recipe_body_runs_line_by_line() {
    // just hands each line of a recipe without a shebang to a fresh
    // shell (`languages/ci/just:V180`).
    assert!(DelimKind::JustRecipe.runs_line_by_line());
}

#[test]
fn a_shebang_recipe_is_one_script() {
    // The `#!` makes the whole body one file for one interpreter.
    assert!(!DelimKind::JustShebangRecipe.runs_line_by_line());
}

#[test]
fn only_the_just_recipe_body_runs_line_by_line() {
    let line_by_line: Vec<DelimKind> = every_kind()
        .into_iter()
        .filter(DelimKind::runs_line_by_line)
        .collect();
    assert_eq!(line_by_line, vec![DelimKind::JustRecipe]);
}
