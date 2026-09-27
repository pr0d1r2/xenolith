//! The README's Languages block: every language xenolith knows, what the
//! default build does with it, and the spec that owns it (`dev:V340`).
//!
//! Three owners, none restated: the ids are `LangId::ALL` (the registry's
//! contract), the feature and the default set are the root `Cargo.toml`,
//! and the spec link is the `SPEC.md` the tree actually holds. It is the
//! table `xnl langs` prints for the default build, plus the one column a
//! binary cannot know -- where the rules for each language live.

use std::fmt::Write as _;

#[cfg(test)]
mod tests;

/// What the tree says about one language.
fn status(id: &str, declared: &[String], default: &[String]) -> (&'static str, String) {
    let feature = format!("lang-{id}");
    if !declared.contains(&feature) {
        ("planned", "—".to_string())
    } else if default.iter().any(|d| d == id) {
        ("default build", format!("`{feature}`"))
    } else {
        ("opt-in feature", format!("`{feature}`"))
    }
}

/// The node directory that owns `id`: `languages/<family>/<id>/SPEC.md`,
/// or `languages/<id>/SPEC.md` for a language with no family.
fn spec_of<'a>(id: &str, specs: &'a [String]) -> Option<&'a str> {
    specs
        .iter()
        .filter_map(|p| p.strip_suffix("/SPEC.md"))
        .find(|dir| dir.starts_with("languages/") && dir.rsplit('/').next() == Some(id))
}

/// The table, one row per id in the order given.
#[must_use]
pub fn render(ids: &[&str], declared: &[String], default: &[String], specs: &[String]) -> String {
    let mut s = String::from("| language | in `xnl` | cargo feature | spec |\n|---|---|---|---|\n");
    for id in ids {
        let (state, feature) = status(id, declared, default);
        let spec =
            spec_of(id, specs).map_or_else(|| "—".to_string(), |d| format!("[`{d}`]({d}/SPEC.md)"));
        let _ = writeln!(s, "| {id} | {state} | {feature} | {spec} |");
    }
    s
}
