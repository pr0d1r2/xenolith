//! Every shared vector, asserted against this port.
//!
//! `vectors.json` is nix-shebang's own `vectors` output, vendored (see
//! `VECTORS.md`). A row that disagrees means one of the two
//! implementations is wrong, which is the whole reason the file is shared
//! rather than transcribed.

use serde_json::Value;
use xenolith_shebang::{Prelude, Shebang, get, has, parse, strip, strip_preamble, strip_strict};

/// The prelude that reproduces nix-shebang's `stripStrict`: shebang plus
/// the exact bash strict line, nothing else.
fn bash_strict() -> Prelude {
    Prelude {
        // Some, because the nix implementation strips whatever shebang the
        // file carries; a prelude that declares one says "this file is
        // executable and the top line is mine".
        shebang: Some(Shebang::env("bash")),
        strict: Some("set -euo pipefail"),
    }
}

fn vectors() -> Vec<Value> {
    let raw = include_str!("vectors.json");
    match serde_json::from_str(raw) {
        Ok(Value::Array(rows)) => rows,
        other => panic!("vectors.json must be a JSON array, got {other:?}"),
    }
}

/// `serde_json` indexing panics on a missing key, and `indexing_slicing` is
/// denied workspace-wide for exactly that reason: a vectors.json that lost
/// a field should fail as a named assertion rather than as a panic inside
/// the harness.
fn field<'a>(row: &'a Value, key: &str) -> &'a Value {
    const NULL: Value = Value::Null;
    row.get(key).unwrap_or(&NULL)
}

fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    field(row, key)
        .as_str()
        .unwrap_or_else(|| panic!("row {:?} has no string {key}", field(row, "name")))
}

#[test]
fn has_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        assert_eq!(
            has(text(&row, "input")),
            field(&row, "has").as_bool().unwrap_or_default(),
            "has disagrees on vector {name}"
        );
    }
}

#[test]
fn get_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        assert_eq!(
            get(text(&row, "input")),
            field(&row, "get").as_str(),
            "get disagrees on vector {name}"
        );
    }
}

#[test]
fn strip_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        assert_eq!(
            strip(text(&row, "input")),
            text(&row, "strip"),
            "strip disagrees on vector {name}"
        );
    }
}

#[test]
fn strip_strict_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        assert_eq!(
            strip_strict(text(&row, "input"), &bash_strict()),
            text(&row, "stripStrict"),
            "strip_strict disagrees on vector {name}"
        );
    }
}

#[test]
fn strip_preamble_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        assert_eq!(
            strip_preamble(text(&row, "input")),
            text(&row, "stripPreamble"),
            "strip_preamble disagrees on vector {name}"
        );
    }
}

#[test]
fn parse_matches_every_vector() {
    for row in vectors() {
        let name = text(&row, "name");
        let parsed = parse(text(&row, "input"));
        match (field(&row, "parse"), parsed) {
            (Value::Null, None) => {}
            (Value::Null, Some(got)) => panic!("vector {name} expects no shebang, got {got:?}"),
            (expected, None) => panic!("vector {name} expects {expected:?}, got none"),
            (expected, Some(got)) => {
                assert_eq!(
                    got.interpreter,
                    field(expected, "interpreter").as_str().unwrap_or_default(),
                    "interpreter disagrees on vector {name}"
                );
                assert_eq!(
                    got.is_env,
                    field(expected, "isEnv").as_bool().unwrap_or_default(),
                    "is_env disagrees on vector {name}"
                );
                assert_eq!(
                    got.resolved_interpreter(),
                    field(expected, "resolvedInterpreter")
                        .as_str()
                        .unwrap_or_default(),
                    "resolved_interpreter disagrees on vector {name}"
                );
                let want: Vec<&str> = field(expected, "args")
                    .as_array()
                    .map(|args| args.iter().filter_map(Value::as_str).collect())
                    .unwrap_or_default();
                assert_eq!(got.args, want, "args disagree on vector {name}");
            }
        }
    }
}

#[test]
fn every_vector_is_exercised() {
    // A vectors.json that shrank -- or a refresh that dropped rows -- must
    // not read as a suite that got faster.
    assert_eq!(vectors().len(), 13, "vector count changed; read the diff");
}
