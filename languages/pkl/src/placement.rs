//! Where an extract of a hk step goes (`languages/pkl:T54`).
//!
//! Always `scripts/hk`, under the step's key (`languages/pkl:V52`): hk
//! steps are named once, in the config, and a script dir beside the
//! config is where the load `bash scripts/hk/<name>.sh {{files}}` looks.

use xenolith_lang_api::{Placement, Site};

#[cfg(test)]
mod tests;

/// The fixed extract dir for hk steps (`languages/pkl:V52`), repo-root
/// relative as a placement dir is (`languages/api/src/lens:V66`).
pub(crate) const DIR: &str = "scripts/hk";

/// The host file's stem, as the `src/extract:V46` template the engine
/// renders: the fallback name's prefix (`languages/api/src/site:V43`).
const STEM: &str = "{host_stem}";

/// The placement of a hk step site.
pub(crate) fn placement(site: &Site) -> Placement {
    Placement {
        name: name(&site.sink),
        dir: DIR.to_owned(),
    }
}

/// The extract name for a `<step>.<property>` sink: the step key,
/// kebab-cased (`languages/api/src/site:V43`). A step key may hold a dot
/// itself, and a property never does, so the split is at the LAST one. A
/// key that kebab-cases to nothing falls back to `{host_stem}-<sink>`.
pub(crate) fn name(sink: &str) -> String {
    let step = sink.rsplit_once('.').map_or(sink, |(step, _)| step);
    let step = kebab(step);
    if !step.is_empty() {
        return step;
    }
    let sink = kebab(sink);
    if sink.is_empty() {
        STEM.to_owned()
    } else {
        format!("{STEM}-{sink}")
    }
}

/// `text` in kebab-case (`languages/api/src/site:V43`): lowercase ASCII
/// words, split at a lower-to-upper step (`preCommit`) and at every
/// character that is not an ASCII letter or digit (`cargo_fmt`), with no
/// empty word and no leading or trailing `-`.
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
