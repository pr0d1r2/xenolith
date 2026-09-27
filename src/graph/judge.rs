//! What each edge's extract should be: where it is
//! (`src/graph:V98`).
//!
//! Each load that resolved is read back to its site
//! ([`crate::extract::back`], `src/extract:V270`), the same way
//! `--relocate` reads it, so the warning and the verb that acts on it
//! cannot disagree. A warning, never a violation -- a config that moved
//! on, and the tree still runs as it is:
//!
//! * `misplaced-extract` -- today's placement config puts the extract
//!   elsewhere; `xnl extract --relocate` moves it.
//!
//! A load that cannot be read back is not judged: a warning about a
//! site nobody found would be a guess.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, LoadRef};

use crate::check::Langs;
use crate::config::Tree;
use crate::extract::back::back;
use crate::model::Warning;

#[cfg(test)]
mod tests;

/// The warning code of an extract today's config places elsewhere
/// (`src/graph:V98`).
pub const MISPLACED: &str = "misplaced-extract";

/// One load that resolved, as the scan met it.
pub(crate) struct Loaded<'a> {
    /// The host file, repo-root relative.
    pub(crate) name: String,
    /// The host that read the load.
    pub(crate) host: &'a dyn Host,
    /// The load.
    pub(crate) load: LoadRef,
    /// 1-based line of the load.
    pub(crate) line: usize,
    /// The extract it resolves to, repo-root relative.
    pub(crate) extract: String,
}

/// The warnings of `loads`, in their order.
pub(crate) fn warnings<'a>(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'a>,
    loads: &[Loaded<'a>],
) -> Vec<Warning> {
    let mut texts: BTreeMap<&str, Option<String>> = BTreeMap::new();
    let mut out = Vec::new();
    for l in loads {
        let text = texts
            .entry(l.name.as_str())
            .or_insert_with(|| fs::read_to_string(root.join(&l.name)).ok());
        let Some(text) = text.as_deref() else {
            continue;
        };
        let read = back(
            root, tree, langs, &l.name, l.host, text, &l.load, &l.extract,
        );
        let Ok(read) = read else {
            continue;
        };
        let at = format!("{}:{}", l.name, l.line);
        if let Some(to) = read.misplaced() {
            out.push(Warning {
                code: MISPLACED.to_owned(),
                file: Some(PathBuf::from(&l.extract)),
                message: format!(
                    "loaded by {at}, but the placement config now puts it at {to}; move it \
                     with `xnl extract --relocate {at}` (src/graph:V98)"
                ),
            });
        }
    }
    out
}
