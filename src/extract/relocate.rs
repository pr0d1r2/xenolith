//! `xnl extract --relocate`: extracts moved to where today's placement
//! config puts them (`src/extract:V99`, `src/extract:V270`).
//!
//! The operands are host files, as for `xnl extract`, and a `:line`
//! names one load. Each load of a named host is read back
//! ([`super::back`]); one whose extract is not where the config places
//! it -- a `misplaced-extract` (`src/graph:V98`) -- moves: the site put
//! back is extracted again to its expected path, which rewrites the
//! load, and the old file goes. The move is proven before anything is
//! written: the new extract must hold the old one's bytes, and the host
//! must read back as every extraction must (`src/extract:V4`).
//!
//! Refused, never guessed (`src/extract:V270`): an extract loaded
//! other than once in the whole tree, since moving it would leave the
//! other load dangling; two moves to one path (`src/extract:V47`,
//! `src/extract:V6`); any refusal in a host leaves that host untouched
//! (`src/extract:V64`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::process::Command;

use xenolith_lang_api::Host;

use super::back::{Loaders, again, back, loaders};
use super::{
    Edit, ExtractError, Gone, HostEdit, Options, Read, Refusal, excluded, position, scan, targets,
};
use crate::check::Langs;
use crate::config::{Config, Tree, Verb};
use crate::graph::resolve::resolve;
use crate::registry;

#[cfg(test)]
mod tests;

/// Plan the relocation of every misplaced extract `options.targets`
/// load, under `config`.
///
/// # Errors
///
/// [`ExtractError`], exit 2, when the run cannot start: discovery, a
/// config that does not load, the whole-tree graph refused.
pub fn relocate(root: &Path, config: &Config, options: &Options) -> Result<Edit, ExtractError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    relocate_with(root, config, options, &langs, &|| Command::new("git"))
}

/// One extract to move.
struct Move {
    host: String,
    /// The load's first byte in the host as read.
    start: usize,
    line: usize,
    extract: String,
    to: String,
}

/// [`relocate`], with the languages and `git` supplied (`tests:V150`).
pub(crate) fn relocate_with(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
) -> Result<Edit, ExtractError> {
    let mut edit = Edit::default();
    let mut wanted = targets(root, &options.targets, git, &mut edit)?;
    let tree = Tree::load(
        root,
        config.clone(),
        Verb::Extract,
        wanted.keys().map(String::as_str),
    )?;
    wanted.retain(|name, _| match excluded(&tree, name) {
        Some(why) => {
            edit.explain.push(format!("{name}: skipped: {why}"));
            false
        }
        None => true,
    });
    let loaded = loaders(root, config, options.strict_hosts, langs, git)?;
    let mut read = BTreeMap::new();
    let mut moves = Vec::new();
    for (name, lines) in &wanted {
        let Some(file) = scan(root, langs, name, &mut edit) else {
            continue;
        };
        let mut found = BTreeSet::new();
        for m in misplaced(root, &tree, langs, name, &file, lines.as_ref(), &mut edit) {
            found.insert(m.line);
            moves.push(m);
        }
        for line in lines.iter().flatten().filter(|l| !found.contains(*l)) {
            edit.refusals.push(Refusal {
                file: name.clone(),
                line: *line,
                message: "no misplaced extract is loaded here, so nothing to relocate".to_owned(),
            });
        }
        read.insert(name.clone(), file);
    }
    let refused = refuse(&moves, &loaded);
    for (name, file) in &read {
        let mut mine: Vec<&Move> = moves.iter().filter(|m| &m.host == name).collect();
        if mine.is_empty() {
            continue;
        }
        let stopped: Vec<Refusal> = mine
            .iter()
            .filter_map(|m| refused.get(&(m.host.clone(), m.line)).map(|why| (m, why)))
            .map(|(m, why)| Refusal {
                file: name.clone(),
                line: m.line,
                message: why.clone(),
            })
            .collect();
        if !stopped.is_empty() {
            edit.refusals.extend(whole(name, stopped, mine.len()));
            continue;
        }
        // Back to front: each move rewrites only its own load and what
        // follows it (`src/extract:V64`).
        mine.sort_by_key(|m| std::cmp::Reverse(m.start));
        match move_file(root, &tree, langs, name, file, &mine) {
            Ok(host) => edit.hosts.push(host),
            Err(refusal) => edit.refusals.extend(whole(name, vec![refusal], mine.len())),
        }
    }
    edit.hosts.sort_by(|a, b| a.path.cmp(&b.path));
    edit.refusals.sort();
    Ok(edit)
}

/// The loads of `file`, the host `name`, whose extracts today's config
/// places elsewhere, on `lines` when given; every other load is said
/// under `--verbose`, and a host whose loads cannot be read is refused.
fn misplaced(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'_>,
    name: &str,
    file: &Read<'_>,
    lines: Option<&BTreeSet<usize>>,
    edit: &mut Edit,
) -> Vec<Move> {
    let loads = match file.host.loads(&file.text) {
        Ok(loads) => loads,
        Err(e) => {
            edit.refusals.push(Refusal {
                file: name.to_owned(),
                line: 0,
                message: format!("its loads cannot be read, so none can be relocated: {e}"),
            });
            return Vec::new();
        }
    };
    let mut moves = Vec::new();
    for load in loads {
        let (line, _) = position(&file.text, load.span.start);
        if lines.is_some_and(|lines| !lines.contains(&line)) {
            continue;
        }
        let at = format!("{name}:{line}");
        let Ok(extract) = resolve(root, name, &load.path) else {
            edit.explain.push(format!(
                "{at}: skipped: the load does not resolve (src/graph:V7)"
            ));
            continue;
        };
        let to = match back(
            root, tree, langs, name, file.host, &file.text, &load, &extract,
        ) {
            Ok(read) => read.misplaced().map(str::to_owned),
            Err(why) => {
                edit.explain
                    .push(format!("{at}: {extract}: not judged: {why}"));
                continue;
            }
        };
        match to {
            Some(to) => moves.push(Move {
                host: name.to_owned(),
                start: load.span.start,
                line,
                extract,
                to,
            }),
            None => edit.explain.push(format!(
                "{at}: {extract}: already where the config places it"
            )),
        }
    }
    moves
}

/// The moves refused before any is tried, by (host, line): an extract
/// loaded other than once in the tree, and two moves to one path.
fn refuse(moves: &[Move], loaded: &Loaders) -> BTreeMap<(String, usize), String> {
    let mut refused = BTreeMap::new();
    for m in moves {
        let count = loaded.get(&m.extract).map_or(0, Vec::len);
        if count != 1 {
            refused.insert(
                (m.host.clone(), m.line),
                format!(
                    "{} is loaded {count} time(s) in the tree; moving it would leave another \
                     load dangling (src/extract:V270)",
                    m.extract
                ),
            );
        }
        if moves.iter().filter(|o| o.to == m.to).count() > 1 {
            refused.insert(
                (m.host.clone(), m.line),
                format!(
                    "`{}` is where another relocated extract goes too (src/extract:V47, \
                     src/extract:V6)",
                    m.to
                ),
            );
        }
    }
    refused
}

/// `src/extract:V64`: a host with any move refused keeps every load.
fn whole(name: &str, mut refused: Vec<Refusal>, moves: usize) -> Vec<Refusal> {
    let others = moves.saturating_sub(refused.len());
    if others > 0 {
        refused.push(Refusal {
            file: name.to_owned(),
            line: 0,
            message: format!(
                "left untouched with its {others} other move(s): a file is relocated whole or \
                 not at all (src/extract:V64)"
            ),
        });
    }
    refused
}

/// Every move of one host, one after the other on the text the last
/// one left: the load read back again, its site extracted to the new
/// path, the old file removed once the host is written.
fn move_file(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'_>,
    name: &str,
    file: &Read<'_>,
    moves: &[&Move],
) -> Result<HostEdit, Refusal> {
    let host: &dyn Host = file.host;
    let mut text = file.text.clone();
    let mut extracts = Vec::new();
    let mut removes = Vec::new();
    for m in moves {
        let refusal = |message: String| Refusal {
            file: name.to_owned(),
            line: m.line,
            message,
        };
        let load = host
            .loads(&text)
            .ok()
            .and_then(|loads| loads.into_iter().find(|l| l.span.start == m.start))
            .ok_or_else(|| {
                refusal("the load is gone once the moves after it were made".to_owned())
            })?;
        let mut read =
            back(root, tree, langs, name, host, &text, &load, &m.extract).map_err(&refusal)?;
        let placed = again(root, name, &mut read, &m.to).map_err(&refusal)?;
        text = placed.after;
        extracts.extend(placed.extracts);
        removes.push(Gone {
            path: m.extract.clone(),
            text: read.text,
            to: Some(m.to.clone()),
        });
    }
    extracts.sort_by(|a, b| a.path.cmp(&b.path));
    removes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(HostEdit {
        path: name.to_owned(),
        before: file.text.clone(),
        after: text,
        extracts,
        removes,
    })
}
