//! The input map: the mirror of `dev/src/select.rs` (`src:C139`).

use super::{OUTPUTS, matches, selected};

fn paths(p: &[&str]) -> Vec<String> {
    p.iter().map(|s| (*s).to_string()).collect()
}

#[test]
fn a_literal_pattern_matches_only_itself() {
    assert!(matches("Cargo.toml", "Cargo.toml"));
    assert!(!matches("Cargo.toml", "dev/Cargo.toml"));
    assert!(!matches("Cargo.toml", "Cargo.lock"));
}

#[test]
fn a_prefix_pattern_matches_the_dir_and_below() {
    assert!(matches("dev/src/**", "dev/src/badge.rs"));
    assert!(matches("dev/src/**", "dev/src/badge/tests.rs"));
    assert!(matches("dev/src/**", "dev/src"));
    assert!(!matches("dev/src/**", "dev/tests/cli.rs"));
    assert!(!matches("dev/src/**", "dev/srcx/a.rs"));
}

#[test]
fn a_suffix_pattern_matches_the_name_at_any_depth() {
    assert!(matches("**/SPEC.md", "SPEC.md"));
    assert!(matches("**/SPEC.md", "src/cli/SPEC.md"));
    assert!(!matches("**/SPEC.md", "src/cli/NOTSPEC.md"));
    assert!(matches(
        "**/UPSTREAM",
        "languages/ci/pkl/vendor/tree-sitter-pkl/UPSTREAM"
    ));
}

/// A spec change moves the node count and the spec links; a ratchet moves
/// only the badges; the lock moves only the notices.
#[test]
fn each_input_selects_the_outputs_rendered_from_it() {
    assert_eq!(
        selected(&paths(&["src/cli/SPEC.md"])),
        vec!["badges", "langs"]
    );
    assert_eq!(selected(&paths(&[".coverage"])), vec!["badges"]);
    assert_eq!(selected(&paths(&["Cargo.lock"])), vec!["notices"]);
    assert_eq!(selected(&paths(&["nix/tools.nix"])), vec!["notices"]);
    assert_eq!(
        selected(&paths(&["languages/ci/nix/Cargo.toml"])),
        vec!["badges", "notices"]
    );
    assert_eq!(
        selected(&paths(&["Cargo.toml"])),
        vec!["badges", "langs", "notices"]
    );
}

/// A hand edit inside a generated block is compared at commit, not only at
/// push.
#[test]
fn an_output_file_selects_itself() {
    assert_eq!(selected(&paths(&["README.md"])), vec!["badges", "langs"]);
    assert_eq!(
        selected(&paths(&["docs/THIRD-PARTY-NOTICES.md"])),
        vec!["notices"]
    );
}

#[test]
fn a_change_touching_no_input_selects_nothing() {
    assert!(selected(&paths(&["src/cli/mod.rs"])).is_empty());
    assert!(selected(&paths(&["docs/SECURITY.md"])).is_empty());
}

/// No scope is the wide run: every output is compared.
#[test]
fn no_scope_selects_every_output() {
    assert_eq!(selected(&[]).len(), OUTPUTS.len());
}

/// The hk step's glob is written by hand; this ties it to the outputs'
/// inputs so a new input cannot be forgotten there (`dev:V348`).
#[test]
fn the_hk_glob_is_the_union_of_every_outputs_inputs() {
    let hk = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../hk.pkl"))
        .expect("hk.pkl");
    let step = hk
        .split("[\"dev-generated\"] {")
        .nth(1)
        .expect("dev-generated step");
    let list = step
        .split("glob = List(")
        .nth(1)
        .and_then(|s| s.split(')').next())
        .expect("glob list");
    let mut glob: Vec<&str> = list
        .split(',')
        .map(|s| s.trim().trim_matches('"'))
        .filter(|s| !s.is_empty())
        .collect();
    let mut union: Vec<&str> = OUTPUTS.iter().flat_map(|o| o.inputs.iter().copied()).collect();
    glob.sort_unstable();
    union.sort_unstable();
    union.dedup();
    assert_eq!(glob, union);
}

#[test]
fn a_name_prefix_pattern_matches_any_name_starting_so() {
    assert!(matches("**/LICENSE*", "LICENSE"));
    assert!(matches("**/LICENSE*", "a/vendor/g/LICENSE-MIT"));
    assert!(matches("**/LICENSE*", "a/vendor/g/LICENSE.md"));
    assert!(!matches("**/LICENSE*", "a/vendor/g/LICENSING/x.rs"));
    assert!(!matches("**/LICENSE*", "a/vendor/g/README"));
}

#[test]
fn any_licence_or_notice_file_in_a_vendor_dir_selects_notices() {
    for f in ["LICENSE-MIT", "LICENSE.md", "LICENSE", "NOTICE", "NOTICE.txt"] {
        let p = paths(&[&format!("languages/x/vendor/g/{f}")]);
        assert_eq!(selected(&p), vec!["notices"], "{f}");
    }
}

#[test]
fn the_notice_name_rule_is_the_one_vendored_reads() {
    for n in ["LICENSE", "LICENSE-MIT", "LICENSE.md", "NOTICE", "NOTICE.txt"] {
        assert!(super::is_notice_name(n), "{n}");
    }
    for n in ["README", "UPSTREAM", "COPYING", "license"] {
        assert!(!super::is_notice_name(n), "{n}");
    }
}
