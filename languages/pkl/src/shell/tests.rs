//! Unit tests for reading hk's shell per step (`src:C139`,
//! `languages/pkl:V172`).
//!
//! `tests/host.rs` checks the env every fixture site carries. These go
//! one branch at a time: the command-line reader, the `shell` lookup in
//! a body, and the walk from a step up to the group that owns it.

use tree_sitter::{Node, Tree};
use xenolith_lang_api::GuestEnv;

use super::{HK_DEFAULT, bundle, env, hk_default, option_name, owner, read, shell_value};
use crate::host::{parse, text};

fn with(dialect: &str, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.iter().map(|&option| option.to_owned()).collect(),
    }
}

fn tree(src: &str) -> Tree {
    parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"))
}

/// Every node of `kind` under `node`, depth first, in source order.
fn all<'t>(node: Node<'t>, kind: &str) -> Vec<Node<'t>> {
    let mut out = Vec::new();
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        out.extend(all(child, kind));
    }
    out
}

/// The entry `["key"] { … }` and its body.
fn entry<'t>(tree: &'t Tree, src: &str, key: &str) -> (Node<'t>, Node<'t>) {
    let quoted = format!("\"{key}\"");
    all(tree.root_node(), "objectEntry")
        .into_iter()
        .find_map(|entry| {
            let name = entry.named_child(0)?;
            let body = all(entry, "objectBody").into_iter().next()?;
            (text(name, src) == quoted).then_some((entry, body))
        })
        .unwrap_or_else(|| panic!("no entry {key} in {src:?}"))
}

/// The env of step `key` in `src`.
fn env_of(src: &str, key: &str) -> GuestEnv {
    let parsed = tree(src);
    let (entry, body) = entry(&parsed, src, key);
    env(entry, body, src)
}

/// A hk config whose pre-commit hook's steps are `steps`.
fn hook(steps: &str) -> String {
    format!(
        concat!(
            "amends \"pkl/Config.pkl\"\n",
            "hooks {{\n",
            "  [\"pre-commit\"] {{\n",
            "    steps {{\n",
            "{}",
            "    }}\n",
            "  }}\n",
            "}}\n",
        ),
        steps
    )
}

// --- read -----------------------------------------------------------------

#[test]
fn hks_default_reads_as_the_env_it_is() {
    assert_eq!(read(HK_DEFAULT), Some(hk_default()));
    assert_eq!(hk_default(), with("sh", &["errexit"]));
}

#[test]
fn a_shell_is_read_by_its_basename() {
    assert_eq!(read("bash -c"), Some(with("bash", &[])));
    assert_eq!(read("/bin/zsh -c"), Some(with("zsh", &[])));
    assert_eq!(read("/usr/bin/sh -c"), Some(with("sh", &[])));
}

#[test]
fn letters_and_long_options_are_the_set_state_in_order_once() {
    assert_eq!(
        read("bash -eu -o pipefail -c"),
        Some(with("bash", &["errexit", "nounset", "pipefail"]))
    );
    assert_eq!(
        read("bash -o pipefail -e -c"),
        Some(with("bash", &["pipefail", "errexit"]))
    );
    assert_eq!(read("sh -e -o errexit -c"), Some(with("sh", &["errexit"])));
}

#[test]
fn a_bundle_may_close_on_the_c() {
    assert_eq!(read("bash -ec"), Some(with("bash", &["errexit"])));
    assert_eq!(read("sh -xc"), Some(with("sh", &["xtrace"])));
}

#[test]
fn zsh_folds_its_option_spelling_and_keeps_its_own_letters() {
    // zsh's `NO_UNSET` is `set -u`, its own name for nounset.
    assert_eq!(
        read("zsh -o ERR_EXIT -o no_unset -c"),
        Some(with("zsh", &["errexit", "nounset"]))
    );
    assert_eq!(
        read("zsh -o ERR_EXIT -o pipe_fail -c"),
        Some(with("zsh", &["errexit", "pipefail"]))
    );
    assert_eq!(read("zsh -e -c"), Some(with("zsh", &["errexit"])));
    assert_eq!(read("zsh -f -c"), None, "-f is NO_RCS in zsh");
    assert_eq!(read("bash -f -c"), Some(with("bash", &["noglob"])));
}

#[test]
fn anything_else_is_not_read() {
    for command in [
        "",
        "bash",
        "fish -c",
        "env bash -c",
        "bash -e",
        "bash -c extra",
        "bash --posix -c",
        "bash +e -c",
        "bash -o -c",
        "bash -o nosuch -c",
        "bash -q -c",
        "bash x -c",
    ] {
        assert_eq!(read(command), None, "{command:?}");
    }
}

#[test]
fn bundle_and_option_name_answer_per_letter_and_per_name() {
    assert_eq!(bundle("", super::SET_LETTERS), Some(Vec::new()));
    assert_eq!(
        bundle("aC", super::SET_LETTERS),
        Some(vec!["allexport", "noclobber"])
    );
    assert_eq!(bundle("a", super::ZSH_LETTERS), None);
    assert_eq!(option_name("xtrace", "bash"), Some("xtrace"));
    assert_eq!(option_name("XTRACE", "bash"), None);
    assert_eq!(option_name("X_TRACE", "zsh"), Some("xtrace"));
}

// --- the tree -------------------------------------------------------------

#[test]
fn shell_value_is_the_bodys_own_shell_not_a_local_one() {
    let src = hook(concat!(
        "      [\"a\"] {\n",
        "        local shell = \"bash -c\"\n",
        "        check = \"x\"\n",
        "      }\n",
    ));
    let parsed = tree(&src);
    let (_, body) = entry(&parsed, &src, "a");
    assert!(shell_value(body, &src).is_none());

    let src = hook("      [\"a\"] {\n        shell = \"bash -c\"\n      }\n");
    let parsed = tree(&src);
    let (_, body) = entry(&parsed, &src, "a");
    let value = shell_value(body, &src).unwrap_or_else(|| panic!("no shell in {src:?}"));
    assert_eq!(text(value, &src), "\"bash -c\"");
}

#[test]
fn a_step_in_a_group_is_owned_by_the_group() {
    let src = hook(concat!(
        "      [\"g\"] = new Group {\n",
        "        shell = \"zsh -c\"\n",
        "        steps {\n",
        "          [\"s\"] {\n",
        "            check = \"x\"\n",
        "          }\n",
        "        }\n",
        "      }\n",
    ));
    let parsed = tree(&src);
    let (step, _) = entry(&parsed, &src, "s");
    let group = owner(step, &src).unwrap_or_else(|| panic!("no owner in {src:?}"));
    assert!(shell_value(group, &src).is_some());
    assert_eq!(env_of(&src, "s"), with("zsh", &[]));
}

#[test]
fn a_step_outside_steps_has_no_owner() {
    let src = concat!(
        "amends \"pkl/Config.pkl\"\n",
        "local m = new Mapping {\n",
        "  [\"s\"] {\n",
        "    check = \"x\"\n",
        "  }\n",
        "}\n",
    );
    let parsed = tree(src);
    let (step, _) = entry(&parsed, src, "s");
    assert!(owner(step, src).is_none());
    assert_eq!(env_of(src, "s"), hk_default());
}

#[test]
fn a_steps_mapping_under_new_is_still_its_owners() {
    let mapping = concat!(
        "    steps = new Mapping<String, Step> {\n",
        "      [\"s\"] {\n",
        "        check = \"x\"\n",
        "      }\n",
        "    }\n",
    );
    let src = hook("").replace("    steps {\n    }\n", mapping);
    let parsed = tree(&src);
    let (step, _) = entry(&parsed, &src, "s");
    assert!(owner(step, &src).is_some(), "{src}");
}

#[test]
fn the_steps_own_shell_wins_and_an_unreadable_one_is_hks_default() {
    let group = |step_shell: &str| {
        hook(&format!(
            concat!(
                "      [\"g\"] = new Group {{\n",
                "        shell = \"zsh -c\"\n",
                "        steps {{\n",
                "          [\"s\"] {{\n",
                "{}",
                "            check = \"x\"\n",
                "          }}\n",
                "        }}\n",
                "      }}\n",
            ),
            step_shell
        ))
    };
    assert_eq!(
        env_of(&group("            shell = \"bash -e -c\"\n"), "s"),
        with("bash", &["errexit"])
    );
    assert_eq!(
        env_of(&group("            shell = \"fish -c\"\n"), "s"),
        hk_default()
    );
    let bare = hook("      [\"s\"] {\n        check = \"x\"\n      }\n");
    assert_eq!(env_of(&bare, "s"), hk_default());
}
