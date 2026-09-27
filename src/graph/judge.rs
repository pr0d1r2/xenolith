//! What each edge's extract should be: where it is, and whether it is
//! still worth a file (`src/graph:V98`, `src/graph:V100`) -- and which
//! sink its load sits in (`src/graph` §I json).
//!
//! Each load that resolved is read back to its site
//! ([`crate::extract::back`], `src/extract:V270`), the same way
//! `--relocate` and `xnl inline` read it, so the warning and the verb
//! that acts on it cannot disagree. Two warnings, never violations --
//! both are a config or an edit that moved on, and the tree still runs
//! as it is:
//!
//! * `misplaced-extract` -- today's placement config puts the extract
//!   elsewhere; `xnl extract --relocate` moves it.
//! * `inlineable-extract` -- its body is trivial for its guest now, by
//!   the verdict `xnl check` gives a site (`src/check:V152`); `xnl
//!   inline` puts it back. Said only when the run sees one load of the
//!   extract, since inline refuses a shared one.
//!
//! A load that cannot be read back is not judged: a warning about a
//! site nobody found would be a guess. Its sink is still asked for,
//! with the extract's text put back as it is ([`sink_at`]), since the
//! sink does not depend on the body; no single site there → `""`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, LoadRef};

use crate::check::Langs;
use crate::config::Tree;
use crate::extract::back::{back, sink_at};
use crate::model::Warning;

#[cfg(test)]
mod tests;

/// The warning code of an extract today's config places elsewhere
/// (`src/graph:V98`).
pub const MISPLACED: &str = "misplaced-extract";

/// The warning code of an extract whose body may stay inline now
/// (`src/graph:V100`).
pub const INLINEABLE: &str = "inlineable-extract";

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

/// What the loads were judged to be.
#[derive(Debug, Default)]
pub(crate) struct Judged {
    /// The warnings, in the loads' order.
    pub(crate) warnings: Vec<Warning>,
    /// Each load's sink, one per load and in its order; `""` where no
    /// single site holds the load.
    pub(crate) sinks: Vec<String>,
}

/// The warnings and sinks of `loads`.
pub(crate) fn judge<'a>(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'a>,
    loads: &[Loaded<'a>],
) -> Judged {
    let mut count: BTreeMap<&str, usize> = BTreeMap::new();
    for l in loads {
        *count.entry(l.extract.as_str()).or_default() += 1;
    }
    let mut texts: BTreeMap<&str, Option<String>> = BTreeMap::new();
    let mut out = Judged::default();
    for l in loads {
        let text = texts
            .entry(l.name.as_str())
            .or_insert_with(|| fs::read_to_string(root.join(&l.name)).ok());
        let Some(text) = text.as_deref() else {
            out.sinks.push(String::new());
            continue;
        };
        let read = back(
            root, tree, langs, &l.name, l.host, text, &l.load, &l.extract,
        );
        let Ok(read) = read else {
            let body = fs::read(root.join(&l.extract)).unwrap_or_default();
            let body = String::from_utf8_lossy(&body);
            out.sinks
                .push(sink_at(l.host, text, &l.load, &body).unwrap_or_default());
            continue;
        };
        out.sinks.push(read.sink().to_owned());
        let at = format!("{}:{}", l.name, l.line);
        if let Some(to) = read.misplaced() {
            out.warnings.push(Warning {
                code: MISPLACED.to_owned(),
                file: Some(PathBuf::from(&l.extract)),
                message: format!(
                    "loaded by {at}, but the placement config now puts it at {to}; move it \
                     with `xnl extract --relocate {at}` (src/graph:V98)"
                ),
            });
        }
        let once = count.get(l.extract.as_str()) == Some(&1);
        if once && read.trivial(tree.config_for(&l.name)) {
            out.warnings.push(Warning {
                code: INLINEABLE.to_owned(),
                file: Some(PathBuf::from(&l.extract)),
                message: format!(
                    "its {} body is trivial now and may stay inline in {at}; put it back with \
                     `xnl inline {}` (src/graph:V100)",
                    l.load.guest, l.extract
                ),
            });
        }
    }
    out
}
