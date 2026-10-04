//! Extract roots: the prefixes under which a file can be an extract
//! (`src/graph:V50`, `src/graph` §I roots).
//!
//! Three sources, each read as narrowly as the spec allows, because a
//! root too wide makes every stray script under it an `orphan-extract`
//! the user never asked about:
//!
//! * a rule `path` -- its literal text before the first `{`;
//! * `[extract] root` -- only under the `mirror` and `central` layouts,
//!   the two that place extracts under it;
//! * a host's `Placement.dir` -- asked per site, so only for sites the
//!   run sees, with the host file's own variables rendered.
//!
//! A prefix that comes out empty, absolute or climbing out of the root
//! is no root: the orphan scan never walks the whole repository.

use std::collections::BTreeSet;
use std::path::Path;

use crate::config::{Config, Layout};

#[cfg(test)]
mod tests;

/// The extract roots a run collected, as repo-root relative prefixes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Roots {
    prefixes: BTreeSet<String>,
}

impl Roots {
    /// Add the roots `config` names: every rule `path`, and the layout
    /// `root` when the layout places under it.
    pub fn config(&mut self, config: &Config) {
        if matches!(config.extract.layout, Layout::Mirror | Layout::Central) {
            self.prefixes.extend(dir_prefix(&config.extract.root));
        }
        for rule in &config.extract.rules {
            self.prefixes
                .extend(rule.path.as_deref().and_then(rule_prefix));
        }
    }

    /// Add the placement directory `dir` a host gave for a site of the
    /// host file `host`.
    pub fn placement(&mut self, dir: &str, host: &str) {
        self.prefixes.extend(dir_prefix(&render_dir(dir, host)));
    }

    /// The root `path` lies under, if any.
    #[must_use]
    pub fn covering(&self, path: &str) -> Option<&str> {
        self.prefixes
            .iter()
            .map(String::as_str)
            .find(|prefix| path.starts_with(prefix))
    }

    /// Every root, sorted, each once.
    pub fn prefixes(&self) -> impl Iterator<Item = &str> {
        self.prefixes.iter().map(String::as_str)
    }
}

/// A rule `path` template's root: its literal text before the first
/// `{`, normalised; `None` when that is empty, absolute or escapes.
#[must_use]
pub fn rule_prefix(template: &str) -> Option<String> {
    let (literal, _) = split(template);
    clean(literal, false)
}

/// A directory template's root: cut like a rule path, and when no `{`
/// cut it short, the whole directory with a trailing `/`.
#[must_use]
pub fn dir_prefix(template: &str) -> Option<String> {
    let (literal, cut) = split(template);
    clean(literal, !cut)
}

/// `template` with the host file's variables filled in: `{host_dir}`
/// (`.` at the root, so the result stays relative) and `{host_stem}`.
/// Every other variable is left for the cut.
#[must_use]
pub fn render_dir(template: &str, host: &str) -> String {
    let path = Path::new(host);
    let dir = path
        .parent()
        .map(|p| p.to_string_lossy().into_owned())
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| ".".to_owned());
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    template
        .replace("{host_dir}", &dir)
        .replace("{host_stem}", &stem)
}

/// The text before the first `{`, and whether a `{` was there.
fn split(template: &str) -> (&str, bool) {
    template
        .split_once('{')
        .map_or((template, false), |(literal, _)| (literal, true))
}

/// `literal` without `.` and empty segments, `..` taken back; `None`
/// when absolute, climbing above the root, or empty. Under `dir`, the
/// last segment is a whole directory and the result ends in `/`.
fn clean(literal: &str, dir: bool) -> Option<String> {
    if literal.starts_with('/') {
        return None;
    }
    let mut segments: Vec<&str> = literal.split('/').collect();
    let last = segments.pop().unwrap_or_default();
    let mut out: Vec<&str> = Vec::new();
    let whole = segments.into_iter().chain(dir.then_some(last));
    for segment in whole {
        match segment {
            "" | "." => {}
            ".." => {
                out.pop()?;
            }
            other => out.push(other),
        }
    }
    let partial = if dir { "" } else { last };
    if partial == "." || partial == ".." {
        return None;
    }
    let mut prefix = out.join("/");
    if !out.is_empty() {
        prefix.push('/');
    }
    prefix.push_str(partial);
    (!prefix.is_empty()).then_some(prefix)
}
