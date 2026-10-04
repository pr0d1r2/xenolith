//! The Languages block: the mirror of `dev/src/langs.rs` (`src:C139`).

use super::render;

fn owned(v: &[&str]) -> Vec<String> {
    v.iter().map(|s| (*s).to_string()).collect()
}

/// The three states a language can be in, each from its owner: in the
/// default set, a feature that is off by default, or no feature at all.
#[test]
fn each_language_is_default_opt_in_or_planned() {
    let out = render(
        &["awk", "just", "nix", "yaml"],
        &owned(&["default", "lang-just", "lang-nix"]),
        &owned(&["nix"]),
        &owned(&[
            "SPEC.md",
            "languages/SPEC.md",
            "languages/ci/just/SPEC.md",
            "languages/ci/nix/SPEC.md",
            "languages/data/awk/SPEC.md",
            "src/nix/SPEC.md",
        ]),
    );
    assert_eq!(
        out,
        "| language | in `xnl` | cargo feature | spec |\n\
         |---|---|---|---|\n\
         | awk | planned | — | [`languages/data/awk`](languages/data/awk/SPEC.md) |\n\
         | just | opt-in feature | `lang-just` | [`languages/ci/just`](languages/ci/just/\
         SPEC.md) |\n\
         | nix | default build | `lang-nix` | [`languages/ci/nix`](languages/ci/nix/SPEC.md) |\n\
         | yaml | planned | — | — |\n"
    );
}

/// A language directly under `languages/` (no family hub) still resolves,
/// and a node elsewhere with the same last segment does not.
#[test]
fn a_family_less_language_resolves_and_a_namesake_elsewhere_does_not() {
    let out = render(
        &["rust"],
        &[],
        &[],
        &owned(&["tests/rust/SPEC.md", "languages/rust/SPEC.md"]),
    );
    assert!(out.contains("| rust | planned | — | [`languages/rust`](languages/rust/SPEC.md) |"));
    assert!(!out.contains("tests/rust"));
}
