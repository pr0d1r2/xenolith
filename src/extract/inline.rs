//! `xnl inline`: an extract put back into its host, the exact inverse
//! of extracting it (`src/extract:V101`, `src/extract:V270`).
//!
//! The operands are extract files. Each is found in the whole tree's
//! graph (`src/graph:V7`) -- the one load of it -- and read back
//! ([`super::back`]): its body goes back through the host's own
//! [`xenolith_lang_api::Host::inline`]. Before anything is written the
//! result is proven the inverse it claims to be: extracting the site
//! again must give the host as it is and the extract's own bytes.
//!
//! Refused, exit 2 (`src/cli` §I, `src/extract:V270`): an extract
//! loaded more than once (shared) or not at all, and a body that is
//! not trivial for its guest -- put back, it would be a xenolith `xnl
//! check` flags, and a rerun of `xnl extract` would move it straight
//! out again (`src/extract:V5`).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::back::{again, back, loaders};
use super::{Edit, ExtractError, Gone, HostEdit, Read, Refusal, excluded, position, scan, words};
use crate::check::{Langs, repo_name};
use crate::config::{Config, Tree, Verb};
use crate::discover::discover_with;
use crate::registry;

#[cfg(test)]
mod tests;

/// What `xnl inline` is asked to put back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InlineOptions {
    /// The extract files; the CLI refuses none.
    pub extracts: Vec<PathBuf>,
    /// `--strict-hosts` (`src/check:V13`).
    pub strict_hosts: bool,
}

/// Plan putting `options.extracts` back into their hosts under
/// `config`.
///
/// # Errors
///
/// [`ExtractError`], exit 2, when the run cannot start: discovery, a
/// config that does not load, the whole-tree graph refused, a path
/// outside the root.
pub fn inline(root: &Path, config: &Config, options: &InlineOptions) -> Result<Edit, ExtractError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    inline_with(root, config, options, &langs, &|| Command::new("git"))
}

/// One extract to put back: its one load, by position in its host.
struct Wanted {
    extract: String,
    line: usize,
    col: usize,
}

/// [`inline`], with the languages and `git` supplied (`tests:V150`).
pub(crate) fn inline_with(
    root: &Path,
    config: &Config,
    options: &InlineOptions,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
) -> Result<Edit, ExtractError> {
    let mut edit = Edit::default();
    let found = discover_with(root, &options.extracts, git)?;
    edit.warnings.extend(found.warnings);
    let mut names = Vec::new();
    for file in &found.files {
        if file.is_absolute() {
            return Err(ExtractError::Outside(file.clone()));
        }
        names.push(repo_name(file));
    }
    let loaded = loaders(root, config, options.strict_hosts, langs, git)?;
    let mut by_host: BTreeMap<String, Vec<Wanted>> = BTreeMap::new();
    for name in names {
        let refuse = |edit: &mut Edit, message: String| {
            edit.refusals.push(Refusal {
                file: name.clone(),
                line: 0,
                message,
            });
        };
        match loaded.get(&name).map(Vec::as_slice).unwrap_or_default() {
            [] => refuse(
                &mut edit,
                "no host loads it, so there is nothing to put it back into (src/graph:V7)"
                    .to_owned(),
            ),
            [(host, line, col)] => by_host.entry(host.clone()).or_default().push(Wanted {
                extract: name.clone(),
                line: *line,
                col: *col,
            }),
            many => {
                let at: Vec<String> = many.iter().map(|(h, l, _)| format!("{h}:{l}")).collect();
                refuse(
                    &mut edit,
                    format!(
                        "loaded {} times ({}); an extract goes back only into its one host \
                         (src/cli §I, src/extract:V270)",
                        many.len(),
                        at.join(", ")
                    ),
                );
            }
        }
    }
    let tree = Tree::load(
        root,
        config.clone(),
        Verb::Extract,
        by_host.keys().map(String::as_str),
    )?;
    for (name, mut wanted) in by_host {
        if let Some(why) = excluded(&tree, &name) {
            for b in &wanted {
                edit.refusals.push(Refusal {
                    file: b.extract.clone(),
                    line: 0,
                    message: format!("its host {name} is skipped: {why}"),
                });
            }
            continue;
        }
        let Some(file) = scan(root, langs, &name, &mut edit) else {
            continue;
        };
        // Back to front: each inline rewrites only its own load and what
        // follows it (`src/extract:V64`).
        wanted.sort_by_key(|b| std::cmp::Reverse((b.line, b.col)));
        match inline_file(root, &tree, langs, &name, &file, &wanted) {
            Ok(host) => edit.hosts.push(host),
            Err(refused) => edit.refusals.extend(refused),
        }
    }
    edit.hosts.sort_by(|a, b| a.path.cmp(&b.path));
    edit.refusals.sort();
    Ok(edit)
}

/// Every extract of one host put back, one after the other on the text
/// the last one left; any refused leaves the host untouched
/// (`src/extract:V64`).
fn inline_file(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'_>,
    name: &str,
    file: &Read<'_>,
    wanted: &[Wanted],
) -> Result<HostEdit, Vec<Refusal>> {
    let host = file.host;
    let config = tree.config_for(name);
    let mut text = file.text.clone();
    let mut removes = Vec::new();
    for b in wanted {
        let refused = |message: String| {
            let mut all = vec![Refusal {
                file: b.extract.clone(),
                line: 0,
                message: format!("loaded by {name}:{}: {message}", b.line),
            }];
            if wanted.len() > 1 {
                all.push(Refusal {
                    file: name.to_owned(),
                    line: 0,
                    message: format!(
                        "left untouched with its {} other extract(s): a host is inlined whole \
                         or not at all (src/extract:V64)",
                        wanted.len() - 1
                    ),
                });
            }
            all
        };
        let load = host
            .loads(&text)
            .ok()
            .and_then(|loads| {
                loads
                    .into_iter()
                    .find(|l| position(&text, l.span.start) == (b.line, b.col))
            })
            .ok_or_else(|| refused("its load is not where the graph found it".to_owned()))?;
        let mut read =
            back(root, tree, langs, name, host, &text, &load, &b.extract).map_err(&refused)?;
        if !read.trivial(config) {
            return Err(refused(format!(
                "its {} body is not trivial, so put back it would be a xenolith `xnl check` \
                 flags and `xnl extract` would move out again (src/extract:V270, \
                 src/extract:V5)",
                read.planned.guest.id()
            )));
        }
        let extract = b.extract.clone();
        let proof = again(root, name, &mut read, &extract).map_err(&refused)?;
        if words(&proof.after) != words(&text) {
            return Err(refused(
                "extracting its site again does not give the host back, so putting it back \
                 is not the exact inverse (src/extract:V101)"
                    .to_owned(),
            ));
        }
        text = read.inlined;
        removes.push(Gone {
            path: extract,
            text: read.text,
            to: None,
        });
    }
    removes.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(HostEdit {
        path: name.to_owned(),
        before: file.text.clone(),
        after: text,
        extracts: Vec::new(),
        removes,
    })
}
