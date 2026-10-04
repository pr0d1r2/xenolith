//! `[[exclude]]` and the per-verb exclude lists: the mirror of
//! `src/config/exclude.rs` (`src:C139`, `src/config:T80`).
//!
//! Three concerns, tested apart because they fail apart:
//!
//! * PARSING (`src/config:V79`): every entry carries a glob and a
//!   non-empty reason, in `[[exclude]]` and in `[check]`, `[extract]`,
//!   `[lint]` and `[graph]` `exclude`; anything else is refused.
//! * MATCHING: what a glob covers, root-relative, a directory covering
//!   everything beneath it.
//! * STALENESS: a glob matching no tracked file is `stale-exclude`.

use super::super::{Config, ConfigError, parse};
use super::{Exclude, Verb};

fn ok(text: &str) -> Config {
    match parse(text) {
        Ok(config) => config,
        Err(err) => panic!("expected a config, got: {err}"),
    }
}

fn err(text: &str) -> ConfigError {
    match parse(text) {
        Ok(config) => panic!("expected a refusal, got: {config:?}"),
        Err(err) => err,
    }
}

fn glob(pattern: &str) -> Exclude {
    Exclude {
        glob: pattern.to_owned(),
        reason: "test".to_owned(),
    }
}

/// The fixture `src/config:T80` names: a vendored directory, excluded
/// for every verb, next to a host file that is not.
const VENDORED: &str = r#"
version = 1

[[exclude]]
glob = "vendor/**"
reason = "third-party code, linted upstream"
"#;

const TRACKED: &[&str] = &[
    "hosts/web/default.nix",
    "vendor/nixpkgs-overlay/default.nix",
    "vendor/tools/install.sh",
];

// ---------------------------------------------------------------------
// parsing (`src/config:V79`)
// ---------------------------------------------------------------------

#[test]
fn a_top_level_exclude_parses_with_its_reason() {
    let config = ok(VENDORED);
    assert_eq!(
        config.exclude.all,
        vec![Exclude {
            glob: "vendor/**".to_owned(),
            reason: "third-party code, linted upstream".to_owned(),
        }]
    );
    for verb in Verb::ALL {
        assert!(config.exclude.verb(*verb).is_empty(), "{verb:?}");
    }
}

#[test]
fn no_file_means_nothing_excluded() {
    // Default = scan every tracked file (`src/config` §I).
    let config = Config::default();
    assert!(config.exclude.all.is_empty());
    for verb in Verb::ALL {
        assert_eq!(config.excluded(*verb, "vendor/tools/install.sh"), None);
    }
}

#[test]
fn every_per_verb_list_parses_into_its_own_verb() {
    let config = ok(r#"
version = 1

[check]
exclude = [{ glob = "a/**", reason = "check only" }]

[extract]
exclude = [{ glob = "b/**", reason = "extract only" }]

[graph]
exclude = [{ glob = "c/**", reason = "graph only" }]

[lint]
exclude = [{ glob = "d/**", reason = "lint only" }]
"#);
    for (verb, want) in [
        (Verb::Check, "a/**"),
        (Verb::Extract, "b/**"),
        (Verb::Graph, "c/**"),
        (Verb::Lint, "d/**"),
    ] {
        let globs: Vec<&str> = config
            .exclude
            .verb(verb)
            .iter()
            .map(|e| e.glob.as_str())
            .collect();
        assert_eq!(globs, vec![want], "{verb:?}");
    }
    assert!(config.exclude.all.is_empty());
}

#[test]
fn an_exclude_without_a_reason_is_refused_everywhere() {
    for (text, key) in [
        ("[[exclude]]\nglob = \"vendor/**\"\n", "exclude[0].reason"),
        (
            "[check]\nexclude = [{ glob = \"vendor/**\" }]\n",
            "check.exclude[0].reason",
        ),
        (
            "[extract]\nexclude = [{ glob = \"vendor/**\" }]\n",
            "extract.exclude[0].reason",
        ),
        (
            "[graph]\nexclude = [{ glob = \"vendor/**\" }]\n",
            "graph.exclude[0].reason",
        ),
        (
            "[lint]\nexclude = [{ glob = \"vendor/**\" }]\n",
            "lint.exclude[0].reason",
        ),
    ] {
        let e = err(&format!("version = 1\n{text}"));
        assert_eq!(e.key, key);
        assert!(e.message.contains("V79"), "{e}");
    }
}

#[test]
fn a_blank_reason_is_refused() {
    let e = err("version = 1\n[[exclude]]\nglob = \"vendor/**\"\nreason = \"  \"\n");
    assert_eq!(e.key, "exclude[0].reason");
}

#[test]
fn a_missing_or_empty_glob_is_refused() {
    let e = err("version = 1\n[[exclude]]\nreason = \"why\"\n");
    assert_eq!(e.key, "exclude[0].glob");
    let e = err("version = 1\n[[exclude]]\nglob = \"\"\nreason = \"why\"\n");
    assert_eq!(e.key, "exclude[0].glob");
}

#[test]
fn a_malformed_glob_is_refused() {
    // An unclosed class would otherwise match nothing, silently, and be
    // reported stale for a reason the user cannot see.
    let e = err("version = 1\n[[exclude]]\nglob = \"vendor/[ab\"\nreason = \"why\"\n");
    assert_eq!(e.key, "exclude[0].glob");
}

#[test]
fn an_unknown_key_in_an_exclude_entry_is_refused() {
    let e = err("version = 1\n[[exclude]]\nglob = \"v/**\"\nreason = \"why\"\npath = \"v\"\n");
    assert_eq!(e.key, "exclude[0].path");
}

#[test]
fn a_wrong_shape_is_refused_naming_the_key() {
    let e = err("version = 1\n[check]\nexclude = \"vendor/**\"\n");
    assert_eq!(e.key, "check.exclude");
    let e = err("version = 1\nexclude = [\"vendor/**\"]\n");
    assert_eq!(e.key, "exclude[0]");
}

#[test]
fn an_unknown_key_in_a_verb_table_is_refused() {
    // `[check]` and `[graph]` hold `exclude` only (`src/config` §I).
    let e = err("version = 1\n[check]\nskip = []\n");
    assert_eq!(e.key, "check.skip");
    let e = err("version = 1\n[graph]\nstrict = true\n");
    assert_eq!(e.key, "graph.strict");
}

// ---------------------------------------------------------------------
// matching
// ---------------------------------------------------------------------

#[test]
fn the_vendored_dir_is_excluded_for_every_verb() {
    // Fixture: vendored dir excluded. The host file beside it is not.
    let config = ok(VENDORED);
    for verb in Verb::ALL {
        for path in [
            "vendor/nixpkgs-overlay/default.nix",
            "vendor/tools/install.sh",
        ] {
            assert_eq!(
                config.excluded(*verb, path).map(|e| e.glob.as_str()),
                Some("vendor/**"),
                "{verb:?} {path}"
            );
        }
        assert_eq!(config.excluded(*verb, "hosts/web/default.nix"), None);
    }
}

#[test]
fn a_per_verb_exclude_applies_to_its_verb_only() {
    // Applied ON TOP of `[[exclude]]` (`src/config` §I): lint skips the
    // generated file, check still reads it.
    let config = ok(r#"
version = 1

[[exclude]]
glob = "vendor/**"
reason = "third party"

[lint]
exclude = [{ glob = "generated/**", reason = "formatter output" }]
"#);
    assert!(config.excluded(Verb::Lint, "generated/a.sh").is_some());
    assert!(config.excluded(Verb::Lint, "vendor/a.sh").is_some());
    for verb in [Verb::Check, Verb::Extract, Verb::Graph] {
        assert_eq!(config.excluded(verb, "generated/a.sh"), None, "{verb:?}");
        assert!(config.excluded(verb, "vendor/a.sh").is_some(), "{verb:?}");
    }
}

#[test]
fn a_star_stays_inside_one_segment() {
    let e = glob("*.png");
    assert!(e.matches("logo.png"));
    assert!(!e.matches("img/logo.png"));
    let e = glob("docs/*.md");
    assert!(e.matches("docs/a.md"));
    assert!(!e.matches("docs/x/a.md"));
    assert!(!e.matches("docs/a.mdx"));
}

#[test]
fn a_double_star_spans_any_number_of_segments() {
    let e = glob("**/*.png");
    assert!(e.matches("logo.png"));
    assert!(e.matches("a/b/c/logo.png"));
    let e = glob("a/**/b");
    for path in ["a/b", "a/x/b", "a/x/y/b"] {
        assert!(e.matches(path), "{path}");
    }
    assert!(!e.matches("a/x/c"));
    assert!(glob("**").matches("any/path/at/all"));
}

#[test]
fn a_directory_covers_everything_beneath_it() {
    for pattern in ["vendor", "vendor/", "vendor/**", "./vendor"] {
        let e = glob(pattern);
        assert!(e.matches("vendor/a.nix"), "{pattern}");
        assert!(e.matches("vendor/x/y.sh"), "{pattern}");
        assert!(!e.matches("vendors/a.nix"), "{pattern}");
        assert!(!e.matches("src/vendor/a.nix"), "{pattern}");
    }
    assert!(glob("vendor").matches("vendor"));
}

#[test]
fn a_glob_is_anchored_at_the_root() {
    // Relative to the declaring file (`src/config` §I), never floating:
    // `vendor/**` does not reach a nested `vendor` by accident.
    assert!(!glob("vendor/**").matches("third/vendor/a.nix"));
}

#[test]
fn question_marks_and_classes_match_one_character() {
    let e = glob("hosts/?.nix");
    assert!(e.matches("hosts/a.nix"));
    assert!(!e.matches("hosts/ab.nix"));
    let e = glob("hosts/[ab].nix");
    assert!(e.matches("hosts/a.nix"));
    assert!(e.matches("hosts/b.nix"));
    assert!(!e.matches("hosts/c.nix"));
    let e = glob("hosts/[!a].nix");
    assert!(!e.matches("hosts/a.nix"));
    assert!(e.matches("hosts/c.nix"));
    let e = glob("hosts/[a-c].nix");
    assert!(e.matches("hosts/b.nix"));
    assert!(!e.matches("hosts/d.nix"));
    // Neither crosses a separator.
    assert!(!glob("a?b").matches("a/b"));
    assert!(!glob("a[/]b").matches("a/b"));
}

#[test]
fn a_literal_glob_matches_only_itself() {
    let e = glob("flake.nix");
    assert!(e.matches("flake.nix"));
    assert!(!e.matches("flake.nixx"));
    assert!(!e.matches("sub/flake.nix"));
}

// ---------------------------------------------------------------------
// staleness (`src/config:V79`)
// ---------------------------------------------------------------------

#[test]
fn a_glob_matching_a_tracked_file_is_not_stale() {
    let config = ok(VENDORED);
    assert!(config.stale_excludes(TRACKED.iter().copied()).is_empty());
}

#[test]
fn a_glob_matching_no_tracked_file_is_stale_naming_its_entry() {
    let config = ok(r#"
version = 1

[[exclude]]
glob = "vendor/**"
reason = "third party"

[[exclude]]
glob = "third_party/**"
reason = "removed last year"

[lint]
exclude = [
  { glob = "hosts/**", reason = "fine" },
  { glob = "generated/**", reason = "no longer generated" },
]
"#);
    let stale: Vec<(String, &str)> = config
        .stale_excludes(TRACKED.iter().copied())
        .into_iter()
        .map(|(key, e)| (key, e.glob.as_str()))
        .collect();
    assert_eq!(
        stale,
        vec![
            ("exclude[1]".to_owned(), "third_party/**"),
            ("lint.exclude[1]".to_owned(), "generated/**"),
        ]
    );
}

#[test]
fn stale_entries_come_in_a_fixed_order() {
    // `[[exclude]]` first, then the verbs by name, each in file order,
    // whatever order the tracked files arrive in (`src:V11`).
    let config = ok(r#"
version = 1

[[exclude]]
glob = "x/**"
reason = "r"

[lint]
exclude = [{ glob = "l/**", reason = "r" }]

[check]
exclude = [{ glob = "c/**", reason = "r" }]

[graph]
exclude = [{ glob = "g/**", reason = "r" }]

[extract]
exclude = [{ glob = "e/**", reason = "r" }]
"#);
    let keys: Vec<String> = config
        .stale_excludes(TRACKED.iter().rev().copied())
        .into_iter()
        .map(|(key, _)| key)
        .collect();
    assert_eq!(
        keys,
        vec![
            "exclude[0]",
            "check.exclude[0]",
            "extract.exclude[0]",
            "graph.exclude[0]",
            "lint.exclude[0]",
        ]
    );
}

#[test]
fn with_no_tracked_files_every_exclude_is_stale() {
    let config = ok(VENDORED);
    assert_eq!(config.stale_excludes(std::iter::empty()).len(), 1);
}

#[test]
fn verbs_have_stable_names() {
    let names: Vec<&str> = Verb::ALL.iter().map(|v| v.as_str()).collect();
    assert_eq!(names, vec!["check", "extract", "graph", "lint"]);
}
