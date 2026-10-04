//! `[[allow]]` matching and staleness: the mirror of
//! `src/config/allow.rs` (`src:C139`, `src/config:T25`).
//!
//! The engine hands over the sites it saw; this module answers two
//! questions about them. Which allow covers a site (`src/config:V10`:
//! path, sink and body hash, nothing positional), and which allows cover
//! no site at all -- the stale ones `src/config:V9` makes a violation.

use super::super::{Config, parse};
use super::SiteKey;

fn config(text: &str) -> Config {
    match parse(text) {
        Ok(config) => config,
        Err(err) => panic!("expected a config, got: {err}"),
    }
}

/// Two allows over one host: `foo` and `bar`, each with its own body.
const TWO: &str = r#"
version = 1

[[allow]]
path = "hosts/web/default.nix"
sink = "systemd.services.foo.script"
hash = "3f2a9c"
reason = "upstream module expects an inline script"

[[allow]]
path = "hosts/web/default.nix"
sink = "systemd.services.bar.script"
hash = "b71e04"
reason = "one-off migration, removed with #31"
"#;

const FOO: SiteKey<'static> = SiteKey {
    path: "hosts/web/default.nix",
    sink: "systemd.services.foo.script",
    hash: "3f2a9c",
};

const BAR: SiteKey<'static> = SiteKey {
    path: "hosts/web/default.nix",
    sink: "systemd.services.bar.script",
    hash: "b71e04",
};

fn stale_sinks(config: &Config, seen: &[SiteKey<'_>]) -> Vec<(usize, String)> {
    config
        .stale_allows(seen.iter().copied())
        .into_iter()
        .map(|(i, allow)| (i, allow.sink.clone()))
        .collect()
}

// ---------------------------------------------------------------------
// matching (`src/config:V10`)
// ---------------------------------------------------------------------

#[test]
fn an_allow_matches_its_own_path_sink_and_hash() {
    let config = config(TWO);
    let allow = config.allowed(&FOO);
    assert_eq!(
        allow.map(|a| a.reason.as_str()),
        Some("upstream module expects an inline script")
    );
    assert_eq!(
        config.allowed(&BAR).map(|a| a.sink.as_str()),
        Some("systemd.services.bar.script")
    );
}

#[test]
fn an_edit_to_the_body_invalidates_the_allow() {
    // V10: the hash is the body's; a different body is a different site
    // as far as the allow is concerned, however small the edit.
    let config = config(TWO);
    let edited = SiteKey {
        hash: "3f2a9d",
        ..FOO
    };
    assert_eq!(config.allowed(&edited), None);
}

#[test]
fn another_sink_or_another_file_is_not_covered() {
    let config = config(TWO);
    let other_sink = SiteKey {
        sink: "systemd.services.baz.script",
        ..FOO
    };
    let other_file = SiteKey {
        path: "hosts/db/default.nix",
        ..FOO
    };
    assert_eq!(config.allowed(&other_sink), None);
    assert_eq!(config.allowed(&other_file), None);
}

#[test]
fn matching_is_exact_never_a_prefix_or_pattern() {
    // ⊥ wildcard (V9): a short hash or a parent sink covering a longer
    // one would be a blanket allow by another name.
    let config = config(TWO);
    for site in [
        SiteKey {
            hash: "3f2a",
            ..FOO
        },
        SiteKey {
            hash: "3f2a9c00",
            ..FOO
        },
        SiteKey {
            sink: "systemd.services.foo",
            ..FOO
        },
        SiteKey {
            path: "./hosts/web/default.nix",
            ..FOO
        },
    ] {
        assert_eq!(config.allowed(&site), None, "{site:?}");
    }
}

#[test]
fn the_first_matching_entry_is_the_one_reported() {
    // Two entries for one site: the report names the first, so the same
    // file always yields the same answer (`src:V11`).
    let text = format!(
        "{TWO}\n[[allow]]\npath = \"hosts/web/default.nix\"\n\
         sink = \"systemd.services.foo.script\"\nhash = \"3f2a9c\"\nreason = \"duplicate\"\n"
    );
    let config = config(&text);
    assert_eq!(
        config.allowed(&FOO).map(|a| a.reason.as_str()),
        Some("upstream module expects an inline script")
    );
}

// ---------------------------------------------------------------------
// staleness (`src/config:V9`)
// ---------------------------------------------------------------------

#[test]
fn every_allow_matched_by_a_seen_site_is_not_stale() {
    let config = config(TWO);
    assert_eq!(stale_sinks(&config, &[FOO, BAR]), vec![]);
}

#[test]
fn an_allow_no_seen_site_matches_is_stale() {
    let config = config(TWO);
    assert_eq!(
        stale_sinks(&config, &[FOO]),
        vec![(1, "systemd.services.bar.script".to_owned())]
    );
}

#[test]
fn an_allow_whose_body_changed_is_stale() {
    // The allow for the old body now covers nothing: the site is flagged
    // afresh AND the entry is reported, so the reviewer re-reads the new
    // body rather than carrying the old verdict over.
    let config = config(TWO);
    let edited = SiteKey {
        hash: "000000",
        ..BAR
    };
    assert_eq!(
        stale_sinks(&config, &[FOO, edited]),
        vec![(1, "systemd.services.bar.script".to_owned())]
    );
}

#[test]
fn with_no_sites_seen_every_allow_is_stale_in_file_order() {
    let config = config(TWO);
    assert_eq!(
        stale_sinks(&config, &[]),
        vec![
            (0, "systemd.services.foo.script".to_owned()),
            (1, "systemd.services.bar.script".to_owned()),
        ]
    );
}

#[test]
fn the_order_of_seen_sites_does_not_change_the_answer() {
    // The scan is parallel (`src/check:V95`): sites arrive in any order, and
    // the stale list must not show it (`src:V11`).
    let config = config(TWO);
    let unrelated = SiteKey {
        sink: "systemd.services.baz.script",
        ..FOO
    };
    assert_eq!(
        stale_sinks(&config, &[unrelated, FOO, FOO]),
        stale_sinks(&config, &[FOO, unrelated]),
    );
}

#[test]
fn duplicate_entries_for_one_seen_site_are_both_not_stale() {
    // A duplicate is redundant, which is `src/config:V89`'s concern; it
    // still matches something, so it is not stale.
    let text = format!(
        "{TWO}\n[[allow]]\npath = \"hosts/web/default.nix\"\n\
         sink = \"systemd.services.foo.script\"\nhash = \"3f2a9c\"\nreason = \"duplicate\"\n"
    );
    let config = config(&text);
    assert_eq!(
        stale_sinks(&config, &[FOO]),
        vec![(1, "systemd.services.bar.script".to_owned())]
    );
}

#[test]
fn a_config_without_allows_has_nothing_stale() {
    let config = config("version = 1\n");
    assert_eq!(stale_sinks(&config, &[FOO]), vec![]);
    assert_eq!(config.allowed(&FOO), None);
}

#[test]
fn an_allow_matches_the_site_key_directly() {
    let config = config(TWO);
    let first = config.allow.first();
    assert_eq!(first.map(|a| a.matches(&FOO)), Some(true));
    assert_eq!(first.map(|a| a.matches(&BAR)), Some(false));
}
