//! `xnl langs`: every language xenolith knows, and whether this build
//! carries it (`src/cli` §I).
//!
//! EVERY `LangId`, compiled in or not (`languages/api:V33`): the point of
//! the verb is to answer "why was my `.sql` ignored?" with "this build
//! has no `lang-sql`", which a list of only the compiled-in languages
//! cannot say.
//!
//! What each entry carries today is id, feature and compiled-in, the
//! last two answered by the registry (`src/registry:V41`), which is the one place
//! a `lang-*` feature is read (`src:V30`). The rest of the §I entry --
//! role, sinks, delimiter kinds, checks, fixers -- widens the JSON shape
//! (`src/cli:V24`) and is a change of its own.

use std::fmt::Write as _;

use serde_json::{Value, json};
use xenolith_lang_api::LangId;

use crate::model::SCHEMA;
use crate::registry::{compiled_in, existing_feature};

#[cfg(test)]
mod tests;

fn state(id: LangId) -> &'static str {
    if existing_feature(id).is_none() {
        "planned"
    } else if compiled_in(id) {
        "compiled-in"
    } else {
        "compiled-out"
    }
}

/// One line per language, in `LangId` order, columns aligned:
/// `<id>  compiled-in|compiled-out  lang-<id>` or `<id>  planned`.
#[must_use]
pub fn human() -> String {
    let id_width = LangId::ALL
        .iter()
        .map(|id| id.as_str().len())
        .max()
        .unwrap_or(0);
    let state_width = "compiled-out".len();
    let mut out = String::new();
    for id in LangId::ALL {
        // Writing to a String cannot fail.
        match existing_feature(*id) {
            Some(feature) => {
                let _ = writeln!(
                    out,
                    "{:<id_width$}  {:<state_width$}  {feature}",
                    id.as_str(),
                    state(*id),
                );
            }
            None => {
                let _ = writeln!(
                    out,
                    "{:<id_width$}  {:<state_width$}",
                    id.as_str(),
                    state(*id),
                );
            }
        }
    }
    out
}

/// The JSON envelope every verb shares (`src/cli` §I, `src/cli:V24`),
/// with `langs` added and `violations`/`warnings` empty: listing
/// languages finds nothing. Pretty-printed and newline-terminated like
/// `Report::to_json`, keys sorted by `serde_json`'s map (`src:V11`).
#[must_use]
pub fn json() -> String {
    let langs: Vec<Value> = LangId::ALL
        .iter()
        .map(|id| {
            existing_feature(*id).map_or_else(
                || {
                    json!({
                        "compiled_in": false,
                        "id": id.as_str(),
                    })
                },
                |feature| {
                    json!({
                        "compiled_in": compiled_in(*id),
                        "feature": feature,
                        "id": id.as_str(),
                    })
                },
            )
        })
        .collect();
    let envelope = json!({
        "langs": langs,
        "schema": SCHEMA,
        "violations": [],
        "warnings": [],
    });
    let mut out = serde_json::to_string_pretty(&envelope).unwrap_or_default();
    out.push('\n');
    out
}
