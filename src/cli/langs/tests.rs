//! `xnl langs`: the mirror of `src/cli/langs.rs` (`src:C139`).
//!
//! What is pinned: EVERY `LangId` is listed, compiled in or not
//! (`languages/api:V33` -- a build without `lang-shell` must still be
//! able to say "shell"), in `LangId` order, and "compiled in" matches the
//! features this test binary was built with. `cargo hack --each-feature`
//! runs these under each subset (`src:V30`), so the last property is
//! checked against every build a consumer can make, not only the default.

use serde_json::Value;
use xenolith_lang_api::LangId;

use super::{human, json};
use crate::registry::feature;

/// What the build says, stated independently of the code under test.
fn built_with(id: LangId) -> bool {
    [
        (LangId::Just, cfg!(feature = "lang-just")),
        (LangId::Nix, cfg!(feature = "lang-nix")),
        (LangId::Pkl, cfg!(feature = "lang-pkl")),
        (LangId::Shell, cfg!(feature = "lang-shell")),
        (LangId::Tcl, cfg!(feature = "lang-tcl")),
    ]
    .contains(&(id, true))
}

// ---------------------------------------------------------------------
// human
// ---------------------------------------------------------------------

#[test]
fn human_is_one_line_per_language_in_langid_order() {
    let text = human();
    assert!(text.ends_with('\n'), "{text:?}");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), LangId::ALL.len(), "{text}");
    for (line, id) in lines.iter().zip(LangId::ALL) {
        let words: Vec<&str> = line.split_whitespace().collect();
        let state = if built_with(*id) {
            "compiled-in"
        } else {
            "compiled-out"
        };
        assert_eq!(words, vec![id.as_str(), state, &feature(*id)], "{line:?}");
    }
}

#[test]
fn human_columns_line_up() {
    // The second column starts at one offset on every line, so a reader
    // scans down it rather than hunting along each row.
    let text = human();
    let starts: Vec<usize> = text
        .lines()
        .map(|line| line.find("compiled-").unwrap_or(0))
        .collect();
    let first = starts.first().copied().unwrap_or(0);
    assert!(first > 0, "{text}");
    assert!(starts.iter().all(|s| *s == first), "{text}");
}

// ---------------------------------------------------------------------
// json
// ---------------------------------------------------------------------

fn parsed() -> Value {
    let text = json();
    assert!(text.ends_with('\n'), "{text:?}");
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"))
}

/// `value[key]` without the index operator (`indexing_slicing`): a
/// missing key reads as `null`, which every assertion below rejects.
fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).unwrap_or(&Value::Null)
}

fn langs(value: &Value) -> Vec<Value> {
    field(value, "langs")
        .as_array()
        .cloned()
        .unwrap_or_default()
}

#[test]
fn json_is_the_shared_envelope_plus_langs() {
    // `src/cli` §I: every verb's JSON is the one envelope, at the one
    // schema number (`src/cli:V24`).
    let value = parsed();
    assert_eq!(*field(&value, "schema"), crate::model::SCHEMA);
    assert_eq!(*field(&value, "violations"), Value::Array(vec![]));
    assert_eq!(*field(&value, "warnings"), Value::Array(vec![]));
    assert!(field(&value, "langs").is_array(), "{value}");
}

#[test]
fn json_lists_every_language_in_langid_order() {
    let value = parsed();
    let ids: Vec<String> = langs(&value)
        .iter()
        .filter_map(|l| field(l, "id").as_str().map(str::to_owned))
        .collect();
    let expected: Vec<&str> = LangId::ALL.iter().map(|id| id.as_str()).collect();
    assert_eq!(ids, expected);
}

#[test]
fn json_entries_carry_exactly_id_feature_and_compiled_in() {
    // Exactly: the rest of the `src/cli` §I entry (role, sinks, delims,
    // checks, fixers) comes from the registry (`src/registry:T46`), and its
    // arrival should be a deliberate change to this test, not a silent
    // widening of the shape (`src/cli:V24`).
    let value = parsed();
    let entries = langs(&value);
    assert_eq!(entries.len(), LangId::ALL.len());
    for (entry, id) in entries.iter().zip(LangId::ALL) {
        let keys: Vec<&str> = entry
            .as_object()
            .map(|o| o.keys().map(String::as_str).collect())
            .unwrap_or_default();
        assert_eq!(keys, vec!["compiled_in", "feature", "id"], "{entry}");
        assert_eq!(*field(entry, "feature"), feature(*id), "{entry}");
        assert_eq!(*field(entry, "compiled_in"), built_with(*id), "{entry}");
    }
}

#[test]
fn json_is_byte_stable() {
    // `src:V11`: same build, same bytes.
    assert_eq!(json(), json());
}
