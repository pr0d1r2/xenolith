//! Where an extract of a nix site goes, and what to do about its holes
//! (`languages/ci/nix:T55`).
//!
//! Both answers come from the [`Site`] alone. The sink path already holds
//! everything the name is cut from (`languages/ci/nix:V53`), and the dir is a
//! `src/extract:V46` template because the site never learns which file it
//! was found in: the engine renders `{host_dir}/{host_stem}`.

use xenolith_lang_api::{Placement, Site};

use crate::sinks;

#[cfg(test)]
mod tests;

/// Beside the host file, in a directory named after it
/// (`languages/ci/nix:V53`): `nix/module.nix` puts its extracts under
/// `nix/module/`.
pub(crate) const DIR: &str = "{host_dir}/{host_stem}";

/// The host file's stem, as the `src/extract:V46` template the engine
/// renders: the fallback name's prefix (`languages/api/src/site:V43`).
const STEM: &str = "{host_stem}";

/// How many attribute segments the name keeps, counted from the end
/// (`languages/ci/nix:V53`): `systemd.services.foo.script` → `foo-script`.
const TAIL: usize = 2;

/// The placement of `site`: its name from the sink path, the dir beside
/// the host.
pub(crate) fn placement(site: &Site) -> Placement {
    Placement {
        name: name(&site.sink),
        dir: DIR.to_owned(),
    }
}

/// The extract name for a sink path (`languages/ci/nix:V53`).
///
/// The last [`TAIL`] ATTRIBUTE segments, kebab-cased and joined. Builder
/// segments are dropped first -- `writeShellApplication` in
/// `packages.deploy.writeShellApplication.text` names the function the
/// body passed through, not the thing it builds -- and with no attribute
/// left the name falls back to `{host_stem}-<sink>`
/// (`languages/api/src/site:V43`).
///
/// The sink path is split on `.`, so a quoted attribute holding a dot
/// (`"foo.conf"`) splits too; the name stays deterministic, only less
/// telling, and a rule in `xenolith.toml` names it better
/// (`src/extract:V45`).
pub(crate) fn name(sink: &str) -> String {
    let attrs: Vec<String> = sink
        .split('.')
        .filter(|segment| !sinks::is_builder(segment))
        .map(kebab)
        .filter(|segment| !segment.is_empty())
        .collect();
    let tail = attrs.get(attrs.len().saturating_sub(TAIL)..).unwrap_or(&[]);
    if !tail.is_empty() {
        return tail.join("-");
    }
    let sink = kebab(sink);
    if sink.is_empty() {
        STEM.to_owned()
    } else {
        format!("{STEM}-{sink}")
    }
}

/// `text` in kebab-case (`languages/api/src/site:V43`): lowercase ASCII
/// words, split at a lower-to-upper step (`ExecStartPre`) and at every
/// character that is not an ASCII letter or digit (`X11/xinit`), with no
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

/// The ways out for `${…}` holes that cannot become parameters
/// mechanically (`languages/ci/nix:V54`), nix's own first.
///
/// The same three for every site: which one fits is a judgement about
/// the body, and the engine shows them as `Judgment` directions without
/// applying any (`languages/api/src/holes:V40`). `<file>` is literal --
/// the file's path is the engine's to resolve (`src/extract:V45`), and a
/// placement guess here could name a file the rules moved.
pub(crate) fn hole_advice() -> Vec<String> {
    [
        "replaceVars: write each `${…}` as `@var@` in the extract and load it \
         with `replaceVars ./<file> { var = …; }`",
        "argv: pass each `${…}` to the extract as an argument",
        "env: pass each `${…}` to the extract as an environment variable",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}
