//! The badge readers and the block: the mirror of `dev/src/badge.rs`
//! (`src:C139`). Every input is a string literal shaped like the real file
//! it stands for, because each reader here reads exactly one owner's shape.

use super::{
    Facts, Locked, Sources, ci_platforms, civil_date, default_languages, dependencies, facts,
    features, gate_steps, locked, manifest_value, members, published, ratchet, render, shield_text,
    truncate_tenth, unsafe_level,
};

/// A root manifest shaped like this repository's: `[workspace.package]`
/// keys, a commented multi-line `members`, a wrapped dependency and the
/// feature table the default languages come from.
const ROOT: &str = "\
[workspace]
# a comment above the list
members = [
  \"dev\",
  # a comment inside it
  \"languages/api\", \"languages/ci/nix\",
]

[workspace.package]
edition = \"2024\"
rust-version = \"1.95\"
license = \"MIT\"

[workspace.lints.rust]
unsafe_code = \"forbid\"

[package]
name = \"xenolith\"
edition.workspace = true

[dependencies]
# a comment that is not a dependency
xenolith-lang-api = { path = \"languages/api\", version = \"0.1.0\" }
serde_json = \"1\"
toml = { version = \"1\", default-features = false, features = [
  \"std\",
  \"parse\",
] }
xenolith-lang-nix = { path = \"languages/ci/nix\", version = \"0.1.0\", optional = true }

[features]
default = [\"lang-nix\", \"lang-shell\"]
lang-nix = [\"dep:xenolith-lang-nix\"]
lang-shell = []

[lints]
workspace = true
";

const NIX: &str = "\
[package]
name = \"xenolith-lang-nix\"

[dependencies]
xenolith-lang-api = { path = \"../../api\", version = \"0.1.0\" }
rnix = \"0.14\"
serde_json = \"1\"

[lints]
workspace = true
";

const GRAMMAR: &str = "\
[package]
name = \"xenolith-lang-pkl\"

[dependencies]
tree-sitter = \"0.27\"

[build-dependencies]
cc = \"1\"

[lints.rust]
unsafe_code = \"deny\"
";

const DEV: &str = "\
[package]
name = \"xenolith-dev\"
publish = false

[dependencies]
private-only = \"1\"

[lints.rust]
unsafe_code = \"deny\"
";

/// `nix-hk` sorts first AND its `inputs` block carries `\"nixpkgs\": [` --
/// the shape that made sherd's prefix match badge the wrong project's rev.
const LOCK: &str = r#"{
  "nodes": {
    "nix-hk": {
      "inputs": {
        "nixpkgs": [
          "nixpkgs-lock",
          "nixpkgs"
        ]
      },
      "locked": {
        "lastModified": 1786698718,
        "rev": "a687c1404575d67f425e17c6bee9ad75dfa14728",
        "type": "github"
      }
    },
    "nixpkgs": {
      "locked": {
        "lastModified": 1789749394,
        "rev": "cf9d2fb3e50fa1cd5114c47505ea9177f7ff5f49",
        "type": "github"
      },
      "original": {
        "ref": "nixos-26.05",
        "type": "github"
      }
    },
    "root": {
      "inputs": {
        "nix-hk": "nix-hk"
      }
    }
  }
}"#;

const PKL: &str = "\
env {
  [\"HK_HIDE_WHEN_DONE\"] = \"true\"
}

local fast = new Mapping<String, Step> {
  [\"fmt\"] {
    env {
      [\"RUST_LOG\"] = \"error\"
    }
  }
  [\"check\"] {
  }
}

local all = (fast) {
  [\"coverage\"] {
  }
}

hooks {
  [\"pre-commit\"] {
  }
  [\"check\"] {
  }
}
";

#[test]
fn manifest_values_come_from_the_first_key_line() {
    assert_eq!(manifest_value(ROOT, "edition").as_deref(), Some("2024"));
    assert_eq!(
        manifest_value(ROOT, "rust-version").as_deref(),
        Some("1.95")
    );
    assert_eq!(manifest_value(ROOT, "license").as_deref(), Some("MIT"));
    assert_eq!(manifest_value(ROOT, "absent"), None);
}

#[test]
fn members_skip_comments_and_span_lines() {
    assert_eq!(
        members(ROOT),
        vec!["dev", "languages/api", "languages/ci/nix"]
    );
    assert!(members("[package]\nname = \"x\"\n").is_empty());
}

#[test]
fn default_languages_are_the_default_features_without_their_prefix() {
    assert_eq!(default_languages(ROOT), vec!["nix", "shell"]);
    assert_eq!(features(ROOT), vec!["default", "lang-nix", "lang-shell"]);
}

/// A key counts where it starts: `toml`'s entry spans four lines, and the
/// comment line and the `[lints]` table count for nothing.
#[test]
fn a_wrapped_dependency_is_one_dependency_and_a_path_is_marked() {
    assert_eq!(
        dependencies(ROOT),
        vec![
            ("xenolith-lang-api".to_string(), true),
            ("serde_json".to_string(), false),
            ("toml".to_string(), false),
            ("xenolith-lang-nix".to_string(), true),
        ]
    );
    assert!(dependencies("[package]\nname = \"x\"\n").is_empty());
}

#[test]
fn publish_false_and_unsafe_levels_are_read_from_the_manifest() {
    assert!(published(ROOT));
    assert!(!published(DEV));
    assert_eq!(unsafe_level(ROOT).as_deref(), Some("forbid"));
    assert_eq!(unsafe_level(GRAMMAR).as_deref(), Some("deny"));
    assert_eq!(unsafe_level(NIX), None, "inherits the workspace table");
}

#[test]
fn publish_forms_match_cargo_metadata_shipped_rule() {
    assert!(!published("[package]\npublish = []\n"));
    assert!(!published("[package]\npublish = [ ]\n"));
    assert!(!published("[package]\npublish = false  # reason\n"));
    assert!(!published("[package]\npublish=false\n"));
    assert!(published("[package]\npublish = true\n"));
    assert!(published("[package]\nname = \"inherited\"\n"));
}

#[test]
fn ratchets_read_their_own_row() {
    let cov = "# comment\nlines 97.96\n# trailing note\n";
    assert_eq!(ratchet(cov, "lines").as_deref(), Some("97.96"));
    assert_eq!(
        ratchet("density 0.0\nshape 0.0\n", "density").as_deref(),
        Some("0.0")
    );
    assert_eq!(ratchet(cov, "density"), None);
}

/// Truncation, not rounding: 98.06 and 98.04 land on one tenth
/// (`dev:V344`).
#[test]
fn a_percentage_truncates_rather_than_rounds() {
    assert_eq!(truncate_tenth("97.96").as_deref(), Some("97.9"));
    assert_eq!(truncate_tenth("98.06").as_deref(), Some("98.0"));
    assert_eq!(truncate_tenth("98.04").as_deref(), Some("98.0"));
    assert_eq!(truncate_tenth("98").as_deref(), Some("98.0"));
    assert_eq!(truncate_tenth("not a number"), None);
    assert_eq!(truncate_tenth("9x.5"), None);
    assert_eq!(truncate_tenth("97."), None);
}

/// The regression sherd's own dev node records: a prefix match read the
/// `"nixpkgs": [` line of `nix-hk`'s inputs as the node and returned
/// nix-hk's rev.
#[test]
fn a_node_header_is_read_and_an_inputs_entry_of_the_same_name_is_not() {
    let np = locked(LOCK, "nixpkgs").unwrap_or_else(|| panic!("no nixpkgs node"));
    assert_eq!(
        np,
        Locked {
            rev: "cf9d2fb".to_string(),
            date: "2026-09-18".to_string(),
            release: Some("26.05".to_string()),
        }
    );
    let hk = locked(LOCK, "nix-hk").unwrap_or_else(|| panic!("no nix-hk node"));
    assert_eq!(hk.rev, "a687c14");
    assert_eq!(
        hk.release, None,
        "an input with no branch claims no release"
    );
    assert!(locked(LOCK, "absent").is_none());
    assert!(
        locked(LOCK, "root").is_none(),
        "a node with no rev is no pin"
    );
}

#[test]
fn a_timestamp_becomes_a_utc_date() {
    assert_eq!(civil_date(1_789_749_394), "2026-09-18");
    assert_eq!(civil_date(0), "1970-01-01");
    // A leap day, and a January: where a hand-rolled calendar breaks.
    assert_eq!(civil_date(1_709_164_800), "2024-02-29");
    assert_eq!(civil_date(1_704_067_200), "2024-01-01");
}

/// `check` is both a step and a hook, and an `env` block sits inside a
/// step: only the two-space `["name"]` lines of `fast` and `all` count.
#[test]
fn gate_steps_counts_steps_and_not_hooks_or_env() {
    assert_eq!(gate_steps(PKL), 3);
    assert_eq!(gate_steps(""), 0);
}

#[test]
fn platforms_come_from_the_matrix_and_ubuntu_means_two_vendors() {
    let yml = "        os: [ubuntu-latest, ubuntu-24.04-arm, macos-13, macos-latest]\n";
    assert_eq!(
        ci_platforms(yml),
        Ok(vec![
            ("amd".to_string(), "linux".to_string()),
            ("arm".to_string(), "linux".to_string()),
            ("arm".to_string(), "macos".to_string()),
            ("intel".to_string(), "linux".to_string()),
            ("intel".to_string(), "macos".to_string()),
        ])
    );
    assert!(
        ci_platforms("jobs:\n  gate:\n")
            .unwrap_or_default()
            .is_empty()
    );
}

#[test]
fn unknown_runner_and_disagreeing_matrices_are_errors() {
    assert!(matches!(
        ci_platforms("os: [windows-latest]\n"),
        Err(error) if error.contains("windows-latest")
    ));
    assert!(ci_platforms("os: [ubuntu-latest]\nos: [macos-13]\n").is_err());
}

#[test]
fn a_shield_field_doubles_what_shields_reads_as_syntax() {
    assert_eq!(shield_text("2026-09-18"), "2026--09--18");
    assert_eq!(shield_text("lint_debt"), "lint__debt");
    assert_eq!(shield_text("MIT OR Apache-2.0"), "MIT_OR_Apache--2.0");
}

fn sources() -> Sources {
    Sources {
        manifest: ROOT.to_string(),
        members: vec![
            ("dev".to_string(), DEV.to_string()),
            ("languages/ci/nix".to_string(), NIX.to_string()),
            ("languages/ci/pkl".to_string(), GRAMMAR.to_string()),
        ],
        coverage: "lines 97.96\n".to_string(),
        debt: "density 0.0\nshape 0.0\n".to_string(),
        pkl: PKL.to_string(),
        lock: LOCK.to_string(),
        workflow: "    os: [macos-latest]\n".to_string(),
    }
}

/// Every number from its owner (`dev:V340`), dependencies over the SHIPPED
/// manifests only and each counted once (`dev:V346`): `serde_json` is named
/// twice, `private-only` belongs to the unpublished crate.
#[test]
fn facts_are_gathered_from_every_owner() {
    let ids = [
        "nix", "shell", "missing", "missing", "missing", "missing", "missing", "missing",
        "missing", "missing", "missing", "missing", "missing", "missing", "missing", "missing",
        "missing", "missing",
    ];
    let f = facts(&sources(), 46, &ids).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        f,
        Facts {
            license: "MIT".to_string(),
            edition: "2024".to_string(),
            msrv: "1.95".to_string(),
            deps: 4,
            unsafe_deny: 1,
            languages: 2,
            planned: 16,
            gate_steps: 3,
            coverage_floor: "97.9".to_string(),
            lint_debt: "0.0".to_string(),
            nodes: 46,
            nixpkgs: Locked {
                rev: "cf9d2fb".to_string(),
                date: "2026-09-18".to_string(),
                release: Some("26.05".to_string()),
            },
            platforms: vec![("arm".to_string(), "macos".to_string())],
        }
    );
}

/// A value with no owner is an ERROR naming the owner, never a default.
#[test]
fn a_missing_value_names_the_file_that_owns_it() {
    type Break = fn(&mut Sources);
    let cases: [(&str, Break); 8] = [
        (".coverage", |s| s.coverage = "# empty\n".to_string()),
        (".coverage's `lines` row", |s| {
            s.coverage = "lines many\n".to_string();
        }),
        (".lint-debt", |s| s.debt = String::new()),
        ("flake.lock", |s| s.lock = "{}".to_string()),
        ("`unsafe_code`", |s| {
            s.manifest = s.manifest.replace("unsafe_code", "x");
        }),
        ("`license`", |s| {
            s.manifest = s.manifest.replace("license", "x");
        }),
        ("`edition`", |s| {
            s.manifest = s.manifest.replace("edition =", "x =");
        }),
        ("`rust-version`", |s| {
            s.manifest = s.manifest.replace("rust-version", "x");
        }),
    ];
    for (owner, break_it) in cases {
        let mut s = sources();
        break_it(&mut s);
        let err = facts(&s, 1, &["missing"]).err().unwrap_or_default();
        assert!(err.contains(owner), "{owner}: {err}");
        assert!(err.contains("dev:V340"), "{err}");
    }
}

#[test]
fn a_root_that_does_not_forbid_unsafe_is_refused() {
    let mut s = sources();
    s.manifest = s.manifest.replace("\"forbid\"", "\"warn\"");
    let err = facts(&s, 1, &["missing"]).err().unwrap_or_default();
    assert!(err.contains("unsafe_code = \"warn\""), "{err}");
    assert!(err.contains("dev:V346"), "{err}");
}

fn rendered(unsafe_deny: usize) -> String {
    let ids = [
        "nix", "shell", "missing", "missing", "missing", "missing", "missing", "missing",
        "missing", "missing", "missing", "missing", "missing", "missing", "missing", "missing",
        "missing", "missing",
    ];
    let mut f = facts(&sources(), 46, &ids).unwrap_or_else(|e| panic!("{e}"));
    f.unsafe_deny = unsafe_deny;
    render(&f)
}

#[test]
fn every_fact_reaches_the_rendered_block() {
    let out = rendered(3);
    for expected in [
        "license-MIT-blue",
        "edition-2024",
        "MSRV-1.95",
        "direct_dependencies-4",
        "unsafe-forbidden,_deny_in_3_FFI_crates-yellowgreen",
        "[![unsafe forbidden, deny in 3 FFI crates]",
        "languages-2_built,_16_planned-6E4AFF",
        "gate_steps-3",
        "coverage_floor-%E2%89%A597.9%25",
        "lint_debt-%E2%89%A40.0%2FKLoC",
        "federated_nodes-46",
        "nixpkgs-26.05_(2026--09--18_--_cf9d2fb)",
        "[![nixpkgs 26.05 (2026-09-18 - cf9d2fb)]",
        "logo=arm",
        "built_with-Opus_5.5",
    ] {
        assert!(out.contains(expected), "missing {expected} in:\n{out}");
    }
    assert!(
        out.lines().all(|l| l.is_empty() || l.ends_with(')')),
        "every badge closes on one line:\n{out}"
    );
}

/// All forbid: the plain badge, green -- the exception wording appears only
/// when there is an exception to name (`dev:V346`).
#[test]
fn an_all_forbid_tree_renders_the_plain_unsafe_badge() {
    let out = rendered(0);
    assert!(out.contains(
        "[![unsafe forbidden](https://img.shields.io/badge/unsafe-forbidden-brightgreen)]\
         (Cargo.toml)"
    ));
    assert!(!out.contains("FFI"));
}

/// A release-less pin says so rather than claiming a release.
#[test]
fn an_unpinned_nixpkgs_says_unpinned() {
    let mut f = facts(&sources(), 1, &["missing"]).unwrap_or_else(|e| panic!("{e}"));
    f.nixpkgs.release = None;
    assert!(render(&f).contains("nixpkgs-unpinned_("));
}

/// No badge may claim something that does not exist yet (`dev:V341`).
#[test]
fn nothing_claims_a_registry_or_a_run() {
    let out = rendered(0);
    assert!(!out.contains("crates.io"));
    assert!(!out.contains("docs.rs"));
    assert!(!out.contains("badge.svg)](https://github.com"));
}
