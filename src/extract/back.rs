//! A load read back: the site an extract came from, found by putting
//! its body back (`src/extract:V270`).
//!
//! An extract on disk says nothing of where it came from; its host's
//! load says only which file. What `--relocate` and `xnl inline` need,
//! and what `xnl graph` judges (`src/graph:V98`, `src/graph:V100`), is
//! the SITE: its sink, its guest env, the placement today's config
//! gives it. The lens answers without a second opinion: the body --
//! the file minus the prelude its site gives it
//! (`languages/api/src/lens:V63`) -- goes back through
//! [`Host::inline`], in memory, and the host's own [`Host::sites`]
//! finds the site in the text that wrote.
//!
//! The prelude depends on the site, and the site is what is being
//! looked for, so the file is read twice when it must be: first with
//! the guest's plain prelude, then, if the site found asks for another,
//! with that one.
//!
//! [`again`] is the proof: the site extracted a second time must give
//! the same bytes, so a move never rewrites content and an inline is an
//! exact inverse (`src/extract:V99`, `src/extract:V101`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::process::Command;

use xenolith_lang_api::{GuestEnv, Host, LoadRef, Site, shebang};

use super::{ExtractError, HostEdit, Planned, Read, edit_file, place, plan, position, prelude};
use crate::check::{Langs, site_verdict};
use crate::config::{Config, Tree};
use crate::graph;

#[cfg(test)]
mod tests;

/// One load, read back.
pub(crate) struct Back<'a> {
    /// The extract, repo-root relative.
    pub(crate) extract: String,
    /// Its bytes on disk.
    pub(crate) text: String,
    /// Its body: `text` without the prelude its site gives it.
    pub(crate) body: String,
    /// The host with the body put back where the load was.
    pub(crate) inlined: String,
    /// The site that put back, placed under today's config.
    pub(crate) planned: Planned<'a>,
}

impl Back<'_> {
    /// Where today's config puts the extract, when that is not where
    /// it is (`src/graph:V98`); the `src/extract:V47` suffixed form of
    /// that path counts as placed, since a run with a collision gives
    /// it. `None` too when the site cannot be placed or would be
    /// refused: a path nobody would write is no place to move to.
    pub(crate) fn misplaced(&self) -> Option<&str> {
        let placed = &self.planned.placed;
        if self.planned.refused.is_some() || placed.path.is_empty() {
            return None;
        }
        let suffixed = place::suffixed(&placed.path, &place::suffix(&self.planned.site.sink));
        (self.extract != placed.path && self.extract != suffixed).then_some(placed.path.as_str())
    }

    /// Whether the body may stay inline under `config` where it goes
    /// back: `xnl check`'s own verdict on that site
    /// ([`crate::check::site_verdict`], `src/check:V152`,
    /// `src/graph:V100`), never a copy of it (`src/extract:B2`).
    pub(crate) fn trivial(&self, config: &Config) -> bool {
        let planned = &self.planned;
        let host = planned.host.id();
        site_verdict(planned.guest, &planned.site, host, &self.body, config).is_none()
    }
}

/// Every load in the whole tree that resolves, by the extract it loads:
/// (host, line, col) each.
pub(crate) type Loaders = BTreeMap<String, Vec<(String, usize, usize)>>;

/// The [`Loaders`] of the tree at `root` -- what `src/extract:V270`
/// counts, since a run over named hosts alone cannot see who else
/// loads a file.
///
/// # Errors
///
/// [`ExtractError::Graph`] when the whole-tree graph is refused.
pub(crate) fn loaders(
    root: &Path,
    config: &Config,
    strict_hosts: bool,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
) -> Result<Loaders, ExtractError> {
    let options = graph::Options {
        paths: Vec::new(),
        strict_hosts,
    };
    let found =
        graph::graph_with(root, config, &options, langs, git).map_err(ExtractError::Graph)?;
    let mut by = Loaders::new();
    for edge in found.edges {
        let host = edge.host.to_string_lossy().into_owned();
        by.entry(edge.extract.to_string_lossy().into_owned())
            .or_default()
            .push((host, edge.line, edge.col));
    }
    Ok(by)
}

/// Read the load `load` of `extract` in `src`, the host file `name`
/// that `host` reads.
///
/// # Errors
///
/// Why the load cannot be read back: its guest is not in this build,
/// the extract cannot be read, the host cannot put the body back, or
/// no single site sits where the load was.
#[allow(clippy::too_many_arguments)] // every argument names the load
pub(crate) fn back<'a>(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'a>,
    name: &str,
    host: &'a dyn Host,
    src: &str,
    load: &LoadRef,
    extract: &str,
) -> Result<Back<'a>, String> {
    let guest = langs
        .guests
        .iter()
        .copied()
        .find(|g| g.id() == load.guest)
        .ok_or_else(|| {
            format!(
                "its guest, {}, is not in this build (src/check:V42)",
                load.guest
            )
        })?;
    let text = fs::read(root.join(extract))
        .map_err(|e| format!("{extract} cannot be read: {e}"))
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|_| format!("{extract} is not UTF-8"))
        })?;
    let first = shebang::strip_strict(&text, &guest.prelude(&GuestEnv::default())).to_owned();
    let at = PutBack {
        tree,
        langs,
        name,
        host,
        src,
        load,
    };
    let (mut inlined, mut planned) = at.put(&first)?;
    let body = shebang::strip_strict(&text, &prelude(&planned)?).to_owned();
    if body != first {
        (inlined, planned) = at.put(&body)?;
    }
    Ok(Back {
        extract: extract.to_owned(),
        text,
        body,
        inlined,
        planned,
    })
}

/// Where a body goes back.
struct PutBack<'s, 'a> {
    tree: &'s Tree,
    langs: &'s Langs<'a>,
    name: &'s str,
    host: &'a dyn Host,
    src: &'s str,
    load: &'s LoadRef,
}

impl<'a> PutBack<'_, 'a> {
    /// `body` put back: the host text, and its site placed.
    fn put(&self, body: &str) -> Result<(String, Planned<'a>), String> {
        let host = self.host;
        let inlined = host
            .inline(self.src, self.load, body)
            .map_err(|e| format!("the {} host cannot put it back: {e}", host.id()))?;
        let site = site_at(host, self.src, &inlined, self.load)?;
        let (line, col) = position(&inlined, site.delim.open.start);
        let guest = site.guest;
        let planned = plan(self.tree, self.langs, self.name, host, site, line, col)
            .ok_or_else(|| format!("its guest, {guest}, is not in this build (src/check:V42)"))?;
        Ok((inlined, planned))
    }
}

/// The one site of `after` -- `before` with the load inlined -- that
/// sits where the load was: inside the span the inline wrote (a nix
/// string in place of a `readFile`) -- or, when none is, the site whose
/// BODY holds that span (a just recipe, whose site opens at its header,
/// before the load line; `src/extract:B1`).
///
/// The span written is the load's, widened to every byte the inline
/// changed: a host may replace what encloses the load too, as nix does
/// the parentheses of a load passed as an argument (`src/extract:B3`).
fn site_at(host: &dyn Host, before: &str, after: &str, load: &LoadRef) -> Result<Site, String> {
    let (start, end) = written(before, after, load);
    let sites = host
        .sites(after)
        .map_err(|e| format!("the host does not parse with the body put back: {e}"))?;
    let within = |s: &Site| s.delim.open.start >= start && s.delim.close.end <= end;
    let holding = |s: &Site| s.delim.body.start <= start && end <= s.delim.body.end;
    let found: Vec<Site> = if sites.iter().any(within) {
        sites.into_iter().filter(within).collect()
    } else {
        sites.into_iter().filter(holding).collect()
    };
    let mut inside = found.into_iter();
    match (inside.next(), inside.next()) {
        (Some(site), None) => Ok(site),
        (None, _) => Err("no site sits where the load was once its body is put back".to_owned()),
        (Some(_), Some(_)) => Err("more than one site sits where the load was".to_owned()),
    }
}

/// The byte span of `after` the inline of `load` wrote: from the first
/// byte that differs from `before`, or the load's start if earlier, to
/// the last, or the load's end as shifted, if later.
fn written(before: &str, after: &str, load: &LoadRef) -> (usize, usize) {
    let (b, a) = (before.as_bytes(), after.as_bytes());
    let prefix = b.iter().zip(a).take_while(|(x, y)| x == y).count();
    let room = b.len().min(a.len()) - prefix;
    let suffix = b
        .iter()
        .rev()
        .zip(a.iter().rev())
        .take(room)
        .take_while(|(x, y)| x == y)
        .count();
    let shifted = (load.span.end + a.len()).saturating_sub(b.len());
    (prefix.min(load.span.start), (a.len() - suffix).max(shifted))
}

/// `back`'s site extracted again, to `path`, from the host with its body
/// put back: the edit that gives, proven as every extraction is
/// (`src/extract:V4`, `src/extract:V5`), and the extract it writes
/// checked against the file read back -- a move or an inline never
/// rewrites what the extract holds (`src/extract:V270`).
///
/// # Errors
///
/// The refusal extracting it again met, or the bytes that differ.
pub(crate) fn again(
    root: &Path,
    name: &str,
    back: &mut Back<'_>,
    path: &str,
) -> Result<HostEdit, String> {
    path.clone_into(&mut back.planned.placed.path);
    let file = Read {
        text: back.inlined.clone(),
        host: back.planned.host,
    };
    let edit = edit_file(root, name, &file, &[&back.planned]).map_err(|refusals| {
        refusals
            .into_iter()
            .next()
            .map(|r| r.message)
            .unwrap_or_default()
    })?;
    match edit.extracts.as_slice() {
        [new] if new.text == back.text => Ok(edit),
        _ => Err(format!(
            "extracting its site again does not give the bytes {} holds, so the move would \
             rewrite it (src/extract:V270)",
            back.extract
        )),
    }
}
