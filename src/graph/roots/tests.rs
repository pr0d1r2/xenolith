//! Extract roots: the mirror of `src/graph/roots.rs` (`src:C139`).
//!
//! Pure string work over templates and config, so no tree is built: what
//! is pinned is which prefixes a config and a host's placements yield
//! (`src/graph:V50`), and that none of them is ever the whole repository.

use super::{Roots, dir_prefix, render_dir, rule_prefix};
use crate::config::{self, Config};

fn config(toml: &str) -> Config {
    config::parse(toml).unwrap_or_else(|e| panic!("fixture config parses: {e}"))
}

fn prefixes(roots: &Roots) -> Vec<&str> {
    roots.prefixes().collect()
}

// ---------------------------------------------------------------------
// a rule `path`: the text before its first `{`
// ---------------------------------------------------------------------

#[test]
fn a_rule_path_counts_up_to_its_first_brace() {
    assert_eq!(
        rule_prefix("scripts/{guest}/{name}.{ext}").as_deref(),
        Some("scripts/")
    );
    // The literal prefix, not its directory: the narrower reading.
    assert_eq!(
        rule_prefix("scripts/hk-{name}.sh").as_deref(),
        Some("scripts/hk-")
    );
}

#[test]
fn a_rule_path_without_a_brace_is_its_own_prefix() {
    assert_eq!(
        rule_prefix("scripts/one.sh").as_deref(),
        Some("scripts/one.sh")
    );
}

#[test]
fn a_rule_path_starting_with_a_brace_is_no_root() {
    // An empty prefix would be the whole repository, which V50 rules out.
    assert_eq!(rule_prefix("{host_dir}/{name}.sh"), None);
    assert_eq!(rule_prefix(""), None);
}

#[test]
fn a_rule_path_that_is_absolute_or_climbs_out_is_no_root() {
    for template in [
        "/abs/{name}.sh",
        "../out/{name}.sh",
        "a/../../{name}.sh",
        "..{name}",
    ] {
        assert_eq!(rule_prefix(template), None, "{template}");
    }
}

#[test]
fn a_rule_path_is_normalised() {
    assert_eq!(
        rule_prefix("./scripts//x/{name}").as_deref(),
        Some("scripts/x/")
    );
    assert_eq!(rule_prefix("a/../b/{name}").as_deref(), Some("b/"));
}

// ---------------------------------------------------------------------
// a directory: the layout root, a placement dir
// ---------------------------------------------------------------------

#[test]
fn a_directory_prefix_ends_in_a_slash() {
    // `scripts` must not cover `scriptsx/a.sh`.
    assert_eq!(dir_prefix("scripts").as_deref(), Some("scripts/"));
    assert_eq!(dir_prefix("scripts/hk/").as_deref(), Some("scripts/hk/"));
}

#[test]
fn a_directory_with_a_brace_left_is_cut_there() {
    assert_eq!(dir_prefix("scripts/{guest}").as_deref(), Some("scripts/"));
    assert_eq!(dir_prefix("{guest}"), None);
}

#[test]
fn an_empty_or_escaping_directory_is_no_root() {
    for dir in ["", ".", "./", "/etc", "..", "../x", "x/../.."] {
        assert_eq!(dir_prefix(dir), None, "{dir:?}");
    }
}

#[test]
fn a_placement_dir_renders_the_host_variables() {
    assert_eq!(
        render_dir("{host_dir}/{host_stem}", "nixos/foo.nix"),
        "nixos/foo"
    );
    // A host at the root has an empty dir: the result is still relative.
    let top = render_dir("{host_dir}/{host_stem}", "flake.nix");
    assert_eq!(dir_prefix(&top).as_deref(), Some("flake/"));
    // Variables a placement dir cannot know stay for the cut.
    assert_eq!(
        render_dir("scripts/{guest}", "a/b.pkl"),
        "scripts/{guest}".to_owned()
    );
}

// ---------------------------------------------------------------------
// a config's roots
// ---------------------------------------------------------------------

#[test]
fn the_default_config_has_no_root() {
    // `layout = "host"` places nothing under `root`, and no rule exists.
    let mut roots = Roots::default();
    roots.config(&Config::default());
    assert!(prefixes(&roots).is_empty(), "{:?}", prefixes(&roots));
}

#[test]
fn the_layout_root_counts_under_mirror_and_central_only() {
    for (layout, expected) in [
        ("host", vec![]),
        ("sibling", vec![]),
        ("mirror", vec!["tools/"]),
        ("central", vec!["tools/"]),
    ] {
        let mut roots = Roots::default();
        roots.config(&config(&format!(
            "version = 1\n[extract]\nlayout = \"{layout}\"\nroot = \"tools\"\n"
        )));
        assert_eq!(prefixes(&roots), expected, "{layout}");
    }
}

#[test]
fn every_rule_path_counts_whatever_the_layout() {
    let mut roots = Roots::default();
    roots.config(&config(
        "version = 1\n\
         [[extract.rule]]\nguest = \"shell\"\npath = \"scripts/{guest}/{name}.{ext}\"\n\
         [[extract.rule]]\nhost = \"pkl\"\npath = \"{host_dir}/{name}.sh\"\n\
         [[extract.rule]]\nhost = \"nix\"\nbase = \"root\"\n",
    ));
    assert_eq!(prefixes(&roots), vec!["scripts/"]);
}

#[test]
fn placements_add_roots_and_prefixes_come_out_sorted_once() {
    let mut roots = Roots::default();
    roots.placement("scripts/hk", "hk.pkl");
    roots.placement("{host_dir}/{host_stem}", "nixos/foo.nix");
    roots.placement("scripts/hk", "other/hk.pkl");
    roots.placement("{guest}", "a.nix");
    assert_eq!(prefixes(&roots), vec!["nixos/foo/", "scripts/hk/"]);
}

#[test]
fn a_root_covers_the_files_under_it_and_nothing_else() {
    let mut roots = Roots::default();
    roots.placement("scripts", "hk.pkl");
    assert_eq!(roots.covering("scripts/a.sh"), Some("scripts/"));
    assert_eq!(roots.covering("scripts/deep/a.sh"), Some("scripts/"));
    assert_eq!(roots.covering("scriptsx/a.sh"), None);
    assert_eq!(roots.covering("scripts"), None);
    assert_eq!(roots.covering("src/a.sh"), None);
}
