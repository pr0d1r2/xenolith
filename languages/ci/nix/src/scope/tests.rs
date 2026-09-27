//! Which name the write side may call nix-shebang by, rule by rule
//! (`languages/ci/nix:V170`, `src:C139`).
//!
//! `tests/rewrite.rs` pins the load each answer turns into; this file
//! pins the answers: every binding form that proves the name, every
//! form that looks close and proves nothing, and shadowing.

use rnix::{Root, SyntaxKind, SyntaxNode};

use super::{binder, binds, nix_shebang, prefix_uses, with_supplies};

const NAME: &str = "nix-shebang";

fn tree(src: &str) -> SyntaxNode {
    let parsed = Root::parse(src);
    assert!(parsed.errors().is_empty(), "{src:?}: {:?}", parsed.errors());
    parsed.syntax()
}

/// The string `''x''` in `src`: the site every case asks about.
fn site(src: &str) -> SyntaxNode {
    tree(src)
        .descendants()
        .find(|node| node.kind() == SyntaxKind::NODE_STRING && node.text() == "''x''")
        .unwrap_or_else(|| panic!("{src:?}: no ''x''"))
}

fn answer(src: &str) -> Option<String> {
    nix_shebang(&site(src))
}

// --- nix_shebang -------------------------------------------------------

#[test]
fn a_binding_of_the_name_itself_proves_it() {
    for src in [
        "{ nix-shebang, ... }: { script = ''x''; }",
        "{ pkgs, nix-shebang }: pkgs.writeShellScript \"n\" ''x''",
        "{ nix-shebang ? null }: ''x''",
        "nix-shebang: { script = ''x''; }",
        "{ ... }@nix-shebang: ''x''",
        "{ inputs }: let nix-shebang = inputs.nix-shebang; in ''x''",
        "{ inputs }: let inherit (inputs) nix-shebang; in ''x''",
        "{ inputs }: rec { nix-shebang = inputs.a; script = ''x''; }",
        "{ a }: let nix-shebang.lib = a; in ''x''",
        "{ inputs }: with { inherit (inputs) nix-shebang; }; ''x''",
        "{ a }: with { nix-shebang = a; }; with { b = 1; }; ''x''",
    ] {
        assert_eq!(answer(src), Some(NAME.to_owned()), "{src:?}");
    }
}

#[test]
fn a_file_without_the_name_in_scope_proves_nothing() {
    for src in [
        "{ script = ''x''; }",
        "{ pkgs, ... }: { script = ''x''; }",
        "{ pkgs }: { nix-shebang = pkgs.a; script = ''x''; }",
        "{ pkgs }: with pkgs; ''x''",
        "{ pkgs, a }: with { nix-shebang = a; }; with pkgs; ''x''",
        "{ a }: with { nix-shebang = a; }.x; ''x''",
        "{ a }: let \"nix-shebang\" = a; in ''x''",
        "{ f }: [ (nix-shebang: nix-shebang) ''x'' ]",
    ] {
        assert_eq!(answer(src), None, "{src:?}");
    }
}

#[test]
fn a_with_namespace_is_no_site_of_its_own_with() {
    // `with E; b` puts E's attributes in scope for `b` only.
    let src = "{ a }: with { nix-shebang = a; s = ''x''; }; 1";
    assert_eq!(answer(src), None);
}

#[test]
fn a_path_the_file_uses_under_the_same_binder_is_the_prefix() {
    let src = "{ inputs, ... }: {\n  \
               a = inputs.nix-shebang.lib.toShellScript;\n  \
               script = ''x'';\n}";
    assert_eq!(answer(src), Some("inputs.nix-shebang".to_owned()));
    let deep = "{ self }: [ self.inputs.nix-shebang.lib.x ''x'' ]";
    assert_eq!(answer(deep), Some("self.inputs.nix-shebang".to_owned()));
}

#[test]
fn a_bound_name_wins_over_a_path_the_file_uses() {
    let src = "{ inputs, nix-shebang }: [ inputs.nix-shebang.lib.x ''x'' ]";
    assert_eq!(answer(src), Some(NAME.to_owned()));
}

#[test]
fn a_shadowed_or_unbound_prefix_proves_nothing() {
    for src in [
        // The inner `inputs` is another value than the one the path used.
        "{ inputs }: { a = inputs.nix-shebang.lib.x; b = inputs: { s = ''x''; }; }",
        "{ inputs }: { a = inputs.nix-shebang.lib.x; b = let inputs = 1; in ''x''; }",
        // Bound where it is used, not where the site is.
        "{ s = ''x''; a = inputs: inputs.nix-shebang.lib.x; }",
        // `inputs` comes from a `with`: nothing proves what it is.
        "{ pkgs }: with pkgs; [ inputs.nix-shebang.lib.x ''x'' ]",
        // No `lib` after the name, or nothing after `lib`.
        "{ inputs }: [ inputs.nix-shebang.x ''x'' ]",
        "{ inputs }: [ inputs.nix-shebang.lib ''x'' ]",
        // A computed segment names no path this module can know.
        "{ inputs, k }: [ inputs.${k}.nix-shebang.lib.x ''x'' ]",
    ] {
        assert_eq!(answer(src), None, "{src:?}");
    }
}

#[test]
fn an_inner_binding_that_shadows_the_name_still_binds_it() {
    // Scope proves a name, never a value: an inner `nix-shebang` is the
    // one nix calls, and the self-check reads the load back.
    let src = "{ nix-shebang }: let nix-shebang = 1; in ''x''";
    assert_eq!(answer(src), Some(NAME.to_owned()));
}

// --- binder, binds, with_supplies, prefix_uses -------------------------

#[test]
fn binder_is_the_nearest_scope_that_binds_the_name() {
    let src = "a: let a = 1; in b: ''x''";
    let at = site(src);
    let found = binder(&at, "a").map(|node| node.kind());
    assert_eq!(found, Some(SyntaxKind::NODE_LET_IN));
    assert_eq!(
        binder(&at, "b").map(|n| n.kind()),
        Some(SyntaxKind::NODE_LAMBDA)
    );
    assert_eq!(binder(&at, "c"), None);
}

#[test]
fn binds_reads_lambdas_lets_and_rec_sets_only() {
    let scope = |src: &str, kind: SyntaxKind| {
        tree(src)
            .descendants()
            .find(|node| node.kind() == kind)
            .unwrap_or_else(|| panic!("{src:?}: no {kind:?}"))
    };
    let lambda = scope("{ a, b ? 1 }@c: 1", SyntaxKind::NODE_LAMBDA);
    assert!(["a", "b", "c"].iter().all(|n| binds(&lambda, n)));
    assert!(!binds(&lambda, "d"));
    let set = scope("{ a = 1; inherit b; }", SyntaxKind::NODE_ATTR_SET);
    assert!(!binds(&set, "a"));
    let rec = scope("rec { a = 1; inherit (x) b; }", SyntaxKind::NODE_ATTR_SET);
    assert!(binds(&rec, "a") && binds(&rec, "b") && !binds(&rec, "x"));
    let with = scope("with { a = 1; }; 1", SyntaxKind::NODE_WITH);
    assert!(!binds(&with, "a"));
}

#[test]
fn with_supplies_stops_at_the_first_namespace_it_cannot_read() {
    let supplied = |src: &str| with_supplies(&site(src), "a");
    assert!(supplied("with { a = 1; }; ''x''"));
    assert!(supplied("with { a = 1; }; with rec { b = 1; }; ''x''"));
    assert!(!supplied("with { a = 1; }; with p; ''x''"));
    assert!(!supplied("with p; ''x''"));
    assert!(!supplied("''x''"));
}

#[test]
fn prefix_uses_lists_every_path_through_nix_shebang_lib() {
    let src = "{ i, j }: [ i.nix-shebang.lib.a j.k.nix-shebang.lib.b.c i.x.y ]";
    let found: Vec<(String, String)> = prefix_uses(&tree(src))
        .into_iter()
        .map(|(base, prefix)| (base.text().to_string(), prefix))
        .collect();
    assert_eq!(
        found,
        vec![
            ("i".to_owned(), "i.nix-shebang".to_owned()),
            ("j".to_owned(), "j.k.nix-shebang".to_owned()),
        ]
    );
}
