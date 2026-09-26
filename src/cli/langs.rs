//! `xnl langs`: every language xenolith knows, and whether this build
//! carries it (`src/cli` §I).
//!
//! EVERY `LangId`, compiled in or not (`languages/api:V33`): the point of
//! the verb is to answer "why was my `.sql` ignored?" with "this build
//! has no `lang-sql`", which a list of only the compiled-in languages
//! cannot say.
//!
//! What each entry carries today is id, feature and compiled-in. The rest
//! of the §I entry -- role, sinks, delimiter kinds, checks, fixers -- is
//! read from the `Host`/`Guest` impls through the registry, which is
//! `src:T46`'s; until it lands, [`compiled_in`] is the one stand-in for
//! it, and it is the only thing here to replace.

use std::fmt::Write as _;

use serde_json::{Value, json};
use xenolith_lang_api::LangId;

use crate::model::SCHEMA;

#[cfg(test)]
mod tests;

/// Whether this build carries the crate for `id`.
///
/// A STAND-IN for the registry (`src:V41`, `src:T46`), which will answer
/// this as "`id` has an entry in `hosts()` or `guests()`". Until then the
/// `lang-*` features are read here, in one function, so the `cfg` sits in
/// exactly one place outside the registry-to-be -- and moves into it,
/// taking the `src:V30` no-leak rule with it, when `src:T46` lands.
///
/// A language absent from the table has no crate and so no feature:
/// nothing to compile in yet.
#[must_use]
pub fn compiled_in(id: LangId) -> bool {
    // A table rather than a `match`: with every feature on, a match of
    // `cfg!` arms is `matches!` in disguise and clippy says so, while
    // with some off it is not -- the table reads the same in every
    // build `cargo hack --each-feature` makes (`src:V30`).
    const BUILT: &[(LangId, bool)] = &[
        (LangId::Nix, cfg!(feature = "lang-nix")),
        (LangId::Pkl, cfg!(feature = "lang-pkl")),
        (LangId::Shell, cfg!(feature = "lang-shell")),
    ];
    BUILT.iter().any(|(lang, on)| *lang == id && *on)
}

/// The Cargo feature that carries `id`: `lang-<id>` (`src:C1`), spelled
/// with `LangId::as_str` like every other name for a language.
#[must_use]
pub fn feature(id: LangId) -> String {
    format!("lang-{id}")
}

fn state(id: LangId) -> &'static str {
    if compiled_in(id) {
        "compiled-in"
    } else {
        "compiled-out"
    }
}

/// One line per language, in `LangId` order, columns aligned:
/// `<id>  compiled-in|compiled-out  lang-<id>`.
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
        let _ = writeln!(
            out,
            "{:<id_width$}  {:<state_width$}  {}",
            id.as_str(),
            state(*id),
            feature(*id)
        );
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
            json!({
                "compiled_in": compiled_in(*id),
                "feature": feature(*id),
                "id": id.as_str(),
            })
        })
        .collect();
    let envelope = json!({
        "langs": langs,
        "schema": SCHEMA,
        "violations": [],
        "warnings": [],
    });
    let mut out = serde_json::to_string_pretty(&envelope)
        .unwrap_or_else(|_| format!("{{\"schema\": {SCHEMA}}}"));
    out.push('\n');
    out
}
