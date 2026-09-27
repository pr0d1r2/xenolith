//! Where an extract of a recipe goes (`languages/ci/just` §I).
//!
//! `scripts/just/<recipe>` beside the justfile: the prototype the spec
//! keeps until `languages/ci/just:T184` decides between it and the
//! fleet's `scripts/<recipe>` (`languages/ci/just:R178`). Beside the
//! justfile rather than at the repo root, because a recipe's working dir
//! is its justfile's (`languages:V74`) and the load is written relative
//! to it.

use xenolith_lang_api::{Placement, Site};

#[cfg(test)]
mod tests;

/// The extract dir, as the `src/extract:V46` template the engine
/// renders.
pub(crate) const DIR: &str = "{host_dir}/scripts/just";

/// The host file's stem, the fallback name's prefix
/// (`languages/api/src/site:V43`).
const STEM: &str = "{host_stem}";

/// The placement of a recipe site.
pub(crate) fn placement(site: &Site) -> Placement {
    Placement {
        name: name(&site.sink),
        dir: DIR.to_owned(),
    }
}

/// The extract name for a recipe: its name kebab-cased
/// (`languages/api/src/site:V43`), or `{host_stem}` when that leaves
/// nothing (a recipe named `_`).
pub(crate) fn name(recipe: &str) -> String {
    let name = kebab(recipe);
    if name.is_empty() {
        STEM.to_owned()
    } else {
        name
    }
}

/// `text` in kebab-case: lowercase ASCII words, split at a lower-to-upper
/// step and at every character that is not an ASCII letter or digit,
/// with no empty word and no leading or trailing `-`.
pub(crate) fn kebab(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 4);
    let mut previous: Option<char> = None;
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() {
            let step = ch.is_ascii_uppercase()
                && previous.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
            if step && !out.is_empty() && !out.ends_with('-') {
                out.push('-');
            }
            out.push(ch.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
        previous = Some(ch);
    }
    while out.ends_with('-') {
        out.pop();
    }
    out
}
