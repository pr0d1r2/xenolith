//! Shebang lines, and the guest each one names.
//!
//! Everything in `xenolith-shebang` is re-exported here so a language
//! crate has ONE dependency and one surface (`languages/shebang` §G).
//! What this module ADDS is the part that needs [`LangId`] and therefore
//! cannot live in that crate: the map from an interpreter to the language
//! it runs.
//!
//! The direction matters. `xenolith-shebang` knows about text; this knows
//! about languages; and the dependency points that way round because the
//! api crate may depend on the shebang crate and not the reverse
//! (`languages/api:V32`).

pub use xenolith_shebang::{
    Prelude, Shebang, get, has, parse, strip, strip_preamble, strip_strict, wrap,
};

use crate::LangId;

#[cfg(test)]
mod tests;

/// The interpreters each language answers to, by basename.
///
/// A TABLE rather than a chain of `contains` checks: the property that
/// matters is that no interpreter appears twice, and a table is the shape
/// where that is visible to a reader and testable in one loop.
const INTERPRETERS: &[(LangId, &[&str])] = &[
    // Every shell dialect resolves to the same language; WHICH dialect is
    // a `GuestEnv` fact, because the same file can be checked as sh or as
    // bash depending on where it came from.
    (LangId::Shell, &["sh", "bash", "zsh", "dash", "ksh", "ash"]),
    (LangId::Python, &["python"]),
    (LangId::Awk, &["awk", "gawk", "mawk", "nawk", "busybox-awk"]),
    (LangId::Jq, &["jq", "gojq", "jaq"]),
    (LangId::Perl, &["perl"]),
    (LangId::Ruby, &["ruby"]),
    (LangId::Js, &["node", "nodejs", "deno", "bun"]),
];

/// The language a shebang names, or `None`.
///
/// `None` rather than a guess. An unrecognised interpreter answered as
/// "shell, probably" would put a tcl script through shellcheck and report
/// its findings as real, which is worse than saying nothing: a wrong
/// answer here becomes a violation with a file, a line and a rule id
/// (`src:V1`) that a reader has every reason to believe.
#[must_use]
pub fn guest_of(shebang: &Shebang) -> Option<LangId> {
    let name = basename(shebang.resolved_interpreter());
    let name = strip_version(name);
    INTERPRETERS
        .iter()
        .find(|(_, names)| names.contains(&name))
        .map(|(lang, _)| *lang)
}

/// Whether this shebang names `lang`.
#[must_use]
pub fn resolves_to(shebang: &Shebang, lang: LangId) -> bool {
    guest_of(shebang) == Some(lang)
}

/// The last path segment: `/usr/bin/python3` becomes `python3`.
fn basename(interpreter: &str) -> &str {
    interpreter.rsplit('/').next().unwrap_or(interpreter)
}

/// `python3.12` becomes `python`, `ruby3.3` becomes `ruby`.
///
/// The version suffix records which interpreter was on the author's
/// machine, not which language the file is written in -- and a table
/// listing `python3`, `python3.11`, `python3.12` and so on would be a
/// table that goes stale every release.
fn strip_version(name: &str) -> &str {
    let trimmed = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    if trimmed.is_empty() { name } else { trimmed }
}
