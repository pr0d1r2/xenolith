//! Nested `xenolith.toml` discovery and merge: the mirror of
//! `src/config/tree.rs` (`src:C139`, `.:T91`).
//!
//! Merge semantics first, on trees built in memory with [`Tree::with`],
//! then discovery from disk with [`Tree::load`] in a [`Sandbox`]
//! (`tests:V150`). The every-key case walks the defaults table rather
//! than listing keys, so a row added later is covered the day it lands.

use xenolith_lang_api::LangId;

use super::{Tree, TreeError, file_in};
use crate::config::defaults::TABLE;
use crate::config::{Config, Effective, Source, Verb, parse};
use crate::discover::{Sandbox, write};

fn ok(text: &str) -> Config {
    parse(text).unwrap_or_else(|e| panic!("fixture config parses: {e}"))
}

fn tree(layers: &[(&str, &str)]) -> Tree {
    layers
        .iter()
        .fold(Tree::new(Config::default()), |tree, (dir, text)| {
            tree.with(dir, ok(text))
                .unwrap_or_else(|e| panic!("layer {dir} merges: {e}"))
        })
}

fn int(config: &Config, key: &str) -> u64 {
    match config.effective(key) {
        Some(Effective::Int(n)) => n,
        other => panic!("{key}: expected an int, got {other:?}"),
    }
}

// ---------------------------------------------------------------------
// the chain: nearest last, subtree only
// ---------------------------------------------------------------------

#[test]
fn no_file_anywhere_is_the_defaults() {
    let tree = Tree::new(Config::default());
    assert_eq!(tree.config_for("a/b/c.nix"), &Config::default());
    assert_eq!(tree.config_for("top.nix"), &Config::default());
}

#[test]
fn a_scalar_is_overridden_by_the_nearest_file() {
    let tree = tree(&[
        ("", "version = 1\n[extract]\ndepth = 3\n"),
        ("a", "version = 1\n[extract]\ndepth = 4\n"),
        ("a/b/c", "version = 1\n[extract]\ndepth = 6\n"),
    ]);
    assert_eq!(int(tree.config_for("x.nix"), "extract.depth"), 3);
    assert_eq!(int(tree.config_for("a/x.nix"), "extract.depth"), 4);
    // `a/b` has no file of its own: `a` governs it.
    assert_eq!(int(tree.config_for("a/b/x.nix"), "extract.depth"), 4);
    assert_eq!(int(tree.config_for("a/b/c/d/x.nix"), "extract.depth"), 6);
}

#[test]
fn a_nested_file_governs_its_subtree_only() {
    // `ab` shares a prefix with `a` and is not under it.
    let tree = tree(&[("a", "version = 1\n[langs]\nunclaimed = \"warn\"\n")]);
    assert_eq!(
        tree.config_for("ab/x.png").langs,
        Config::default().langs,
        "a sibling sharing a prefix is not in the subtree"
    );
    assert_eq!(tree.config_for("x.png").langs, Config::default().langs);
    assert_eq!(
        tree.config_for("a/x.png").langs.unclaimed,
        crate::config::Policy::Warn
    );
}

#[test]
fn a_key_the_nested_file_leaves_unset_is_inherited() {
    let tree = tree(&[
        ("", "version = 1\n[langs]\nunclaimed = \"warn\"\n"),
        ("a", "version = 1\n[parse]\nhost_errors = \"error\"\n"),
    ]);
    let config = tree.config_for("a/x.nix");
    assert_eq!(config.langs.unclaimed, crate::config::Policy::Warn);
    assert_eq!(config.parse.host_errors, crate::config::Policy::Error);
}

#[test]
fn every_table_key_is_overridable_from_a_nested_file() {
    // One non-default value per row of the defaults table; `<guest>` is
    // python. A key the merge forgot would come out at the root's value.
    let sample = |key: &str| -> Option<&'static str> {
        Some(match key {
            "extract.depth" => "[extract]\ndepth = 9",
            "extract.inactive_rules" => "[extract]\ninactive_rules = \"error\"",
            "extract.layout" => "[extract]\nlayout = \"central\"",
            "extract.root" => "[extract]\nroot = \"elsewhere\"",
            "extract.shell.strict" => "[extract.shell]\nstrict = \"enforce\"",
            "langs.missing_guest" => "[langs]\nmissing_guest = \"ignore\"",
            "langs.unclaimed" => "[langs]\nunclaimed = \"error\"",
            "lint.<guest>.extend" => "[lint.python]\nextend = false",
            "lint.hosts" => "[lint]\nhosts = false",
            "lint.timeout" => "[lint]\ntimeout = 7",
            "parse.host_errors" => "[parse]\nhost_errors = \"ignore\"",
            "threshold.<guest>.max_bytes" => "[threshold.python]\nmax_bytes = 3",
            "threshold.<guest>.max_lines" => "[threshold.python]\nmax_lines = 3",
            "threshold.exec.max_args" => "[threshold.exec]\nmax_args = 2",
            "threshold.exec.max_len" => "[threshold.exec]\nmax_len = 2",
            "threshold.load.max_params" => "[threshold.load]\nmax_params = 2",
            "threshold.load.param_prefix" => "[threshold.load]\nparam_prefix = \"P_\"",
            "threshold.shell.allow" => "[threshold.shell]\nallow = [\"case\"]",
            // Resolved per site, not a value a file sets.
            "extract.rule.base" => return None,
            other => panic!("no sample for table row {other}: add one"),
        })
    };
    for entry in TABLE {
        let Some(body) = sample(entry.key) else {
            continue;
        };
        let text = format!("version = 1\n{body}\n");
        let key = entry.key.replace("<guest>", "python");
        let tree = tree(&[("a/b", &text)]);
        let nested = tree.config_for("a/b/x.nix");
        assert_eq!(
            nested.effective(&key),
            ok(&text).effective(&key),
            "{key} from a nested file"
        );
        assert_ne!(
            nested.effective(&key),
            Config::default().effective(&key),
            "{key}: the sample must differ from the default"
        );
        assert_eq!(nested.source(&key), Some(Source::File), "{key}");
        assert_eq!(
            tree.config_for("x.nix").effective(&key),
            Config::default().effective(&key),
            "{key} leaks above its file"
        );
    }
}

// ---------------------------------------------------------------------
// tables deep-merge, entry lists append
// ---------------------------------------------------------------------

#[test]
fn lint_guest_tables_merge_field_by_field_and_their_lists_append() {
    let tree = tree(&[
        (
            "",
            "version = 1\n[lint]\nall = [\"r\"]\n\
             [lint.shell]\nchecks = [\"a\"]\nfixers = [\"f\"]\n",
        ),
        (
            "a",
            "version = 1\n[lint]\nall = [\"s\"]\n\
             [lint.shell]\nchecks = [\"b\"]\nextend = false\n",
        ),
    ]);
    let lint = &tree.config_for("a/x.sh").lint;
    assert_eq!(lint.all, vec!["r".to_owned(), "s".to_owned()]);
    let shell = lint
        .guests
        .get(&LangId::Shell)
        .unwrap_or_else(|| panic!("[lint.shell] survives the merge"));
    assert_eq!(shell.checks, vec!["a".to_owned(), "b".to_owned()]);
    assert_eq!(shell.fixers, vec!["f".to_owned()]);
    assert_eq!(shell.extend, Some(false));
}

#[test]
fn guest_thresholds_merge_field_by_field() {
    let tree = tree(&[
        ("", "version = 1\n[threshold.python]\nmax_lines = 3\n"),
        ("a", "version = 1\n[threshold.python]\nmax_bytes = 9\n"),
    ]);
    assert_eq!(
        tree.config_for("a/x.nix").size_ceiling(LangId::Python),
        Some((3, 9))
    );
}

#[test]
fn threshold_shell_allow_overrides_rather_than_appends() {
    let tree = tree(&[
        ("", "version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n"),
        (
            "a",
            "version = 1\n[threshold.shell]\nallow = [\"pipeline\"]\n",
        ),
    ]);
    assert_eq!(
        tree.config_for("a/x.nix").construct_allow(LangId::Shell),
        ["pipeline".to_owned()]
    );
    assert_eq!(
        tree.config_for("x.nix").construct_allow(LangId::Shell),
        ["and-or".to_owned()]
    );
}

fn allow(path: &str) -> String {
    format!(
        "version = 1\n[[allow]]\npath = \"{path}\"\nsink = \"s\"\nhash = \"h\"\nreason = \"r\"\n"
    )
}

#[test]
fn allows_append_and_a_nested_path_is_relative_to_its_file() {
    let tree = tree(&[("", &allow("top.nix")), ("a/b", &allow("x.nix"))]);
    let paths: Vec<&str> = tree
        .config_for("a/b/x.nix")
        .allow
        .iter()
        .map(|a| a.path.as_str())
        .collect();
    assert_eq!(paths, vec!["top.nix", "a/b/x.nix"]);
    // Above the nested file, only the root's entry.
    assert_eq!(tree.config_for("top.nix").allow.len(), 1);
}

#[test]
fn excludes_append_and_a_nested_glob_is_relative_to_its_file() {
    let tree = tree(&[
        (
            "",
            "version = 1\n[[exclude]]\nglob = \"vendor\"\nreason = \"third party\"\n",
        ),
        (
            "a",
            "version = 1\n[[exclude]]\nglob = \"*.png\"\nreason = \"images\"\n\
             [check]\nexclude = [{ glob = \"./gen/\", reason = \"generated\" }]\n",
        ),
    ]);
    let nested = tree.config_for("a/x.png");
    assert!(nested.excluded(Verb::Check, "vendor/x.nix").is_some());
    assert!(nested.excluded(Verb::Check, "a/x.png").is_some());
    // Anchored to the declaring directory: neither the root nor deeper.
    assert!(nested.excluded(Verb::Check, "x.png").is_none());
    assert!(nested.excluded(Verb::Check, "a/b/x.png").is_none());
    assert!(nested.excluded(Verb::Check, "a/gen/y.nix").is_some());
    assert!(nested.excluded(Verb::Lint, "a/gen/y.nix").is_none());
    // Rebased, the entry is stale only for want of a file under `a`.
    let stale = nested.stale_excludes(["x.png", "a/gen/y.nix", "vendor/v.nix"]);
    let stale: Vec<&str> = stale.iter().map(|(_, e)| e.glob.as_str()).collect();
    assert_eq!(stale, vec!["a/*.png"]);
}

#[test]
fn extract_rules_append() {
    let rule = |sink: &str| format!("version = 1\n[[extract.rule]]\nsink = \"{sink}\"\n");
    let tree = tree(&[("", &rule("root.*")), ("a", &rule("nested.*"))]);
    let sinks: Vec<Option<&str>> = tree
        .config_for("a/x.nix")
        .extract
        .rules
        .iter()
        .map(|r| r.sink.as_deref())
        .collect();
    assert_eq!(sinks, vec![Some("root.*"), Some("nested.*")]);
}

// ---------------------------------------------------------------------
// the layers themselves
// ---------------------------------------------------------------------

#[test]
fn layers_are_the_root_first_then_each_file_by_directory_rebased() {
    let tree = tree(&[("b", &allow("x.nix")), ("a", &allow("y.nix"))]);
    let layers: Vec<(&str, Vec<&str>)> = tree
        .layers()
        .map(|(dir, own)| (dir, own.allow.iter().map(|a| a.path.as_str()).collect()))
        .collect();
    assert_eq!(
        layers,
        vec![("", vec![]), ("a", vec!["a/y.nix"]), ("b", vec!["b/x.nix"])]
    );
}

#[test]
fn nearest_names_the_governing_directory() {
    let tree = tree(&[("a", "version = 1\n"), ("a/b/c", "version = 1\n")]);
    assert_eq!(tree.nearest("x.nix"), "");
    assert_eq!(tree.nearest("a/x.nix"), "a");
    assert_eq!(tree.nearest("a/b/x.nix"), "a");
    assert_eq!(tree.nearest("a/b/c/x.nix"), "a/b/c");
    assert_eq!(tree.nearest("ab/x.nix"), "");
}

#[test]
fn file_in_names_the_config_of_a_directory() {
    assert_eq!(file_in(""), "xenolith.toml");
    assert_eq!(file_in("a/b"), "a/b/xenolith.toml");
}

#[test]
fn a_version_differing_from_the_chain_is_refused_naming_both_files() {
    // `src/config:V70`. Only version 1 parses today, so the differing
    // file is built by hand: the rule must hold the day a second lands.
    let future = Config {
        version: 2,
        ..Config::default()
    };
    let e = tree(&[("a", "version = 1\n")])
        .with("a/b", future)
        .err()
        .unwrap_or_else(|| panic!("a version mismatch is refused"));
    assert_eq!(e.file, "a/b/xenolith.toml");
    assert!(e.message.contains("a/xenolith.toml"), "{e}");
    assert!(e.message.contains("src/config:V70"), "{e}");
}

// ---------------------------------------------------------------------
// discovery from disk
// ---------------------------------------------------------------------

#[test]
fn load_reads_every_file_on_the_way_down_and_skips_directories_without_one() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    write(
        &root,
        "a/xenolith.toml",
        "version = 1\n[extract]\ndepth = 4\n",
    );
    write(&root, "a/b/c/x.nix", "");
    write(&root, "d/y.nix", "");
    let tree = Tree::load(
        &root,
        Config::default(),
        ["a/b/c/x.nix", "d/y.nix", "top.nix"],
    )
    .unwrap_or_else(|e| panic!("{e}"));
    let dirs: Vec<&str> = tree.layers().map(|(dir, _)| dir).collect();
    assert_eq!(dirs, vec!["", "a"]);
    assert_eq!(int(tree.config_for("a/b/c/x.nix"), "extract.depth"), 4);
    assert_eq!(int(tree.config_for("d/y.nix"), "extract.depth"), 5);
}

#[test]
fn load_keeps_the_root_config_it_is_given() {
    // The root file is the caller's: it is never read again here, so a
    // library user's in-memory config stands.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    write(&root, "xenolith.toml", "this is not toml");
    let given = ok("version = 1\n[extract]\ndepth = 2\n");
    let tree = Tree::load(&root, given.clone(), ["x.nix"]).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(tree.config_for("x.nix"), &given);
}

#[test]
fn load_never_reads_outside_the_root() {
    let sandbox = Sandbox::new();
    let outer = sandbox.plain("outer");
    write(&outer, "xenolith.toml", "broken");
    let root = outer.join("inner");
    write(&root, "x.nix", "");
    let tree = Tree::load(&root, Config::default(), ["../x.nix", "x.nix"])
        .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(tree.layers().count(), 1);
}

#[test]
fn a_nested_file_that_does_not_parse_is_refused_naming_it_and_the_key() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    write(
        &root,
        "a/xenolith.toml",
        "version = 1\n[langs]\nbogus = 1\n",
    );
    let e: TreeError = Tree::load(&root, Config::default(), ["a/x.nix"])
        .err()
        .unwrap_or_else(|| panic!("a broken nested file is refused"));
    assert_eq!(e.file, "a/xenolith.toml");
    assert!(e.message.contains("langs.bogus"), "{e}");
    assert!(e.to_string().starts_with("a/xenolith.toml: "), "{e}");
}

#[test]
fn a_nested_file_without_version_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    write(&root, "a/xenolith.toml", "[langs]\nunclaimed = \"warn\"\n");
    let e = Tree::load(&root, Config::default(), ["a/x.nix"])
        .err()
        .unwrap_or_else(|| panic!("version is required in every file"));
    assert_eq!(e.file, "a/xenolith.toml");
    assert!(e.message.contains("version"), "{e}");
}
