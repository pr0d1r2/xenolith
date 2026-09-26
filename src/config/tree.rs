//! Nested `xenolith.toml`: discovery and merge (`src/config` §I,
//! `src/config:V88`, `.:T91`).
//!
//! Any directory may hold a `xenolith.toml`. A file's EFFECTIVE config
//! is the chain from the run's root down to the file's own directory,
//! merged nearest last:
//!
//! * a scalar the nearer file sets overrides the farther one's -- and
//!   `threshold.shell.allow` is a scalar here, one value rather than a
//!   collection of entries;
//! * tables deep-merge: `[lint.<guest>]` and `[threshold.<guest>]` field
//!   by field;
//! * entry lists append: `[[allow]]`, `[[exclude]]` and the per-verb
//!   lists, `[[extract.rule]]`, `[lint] all`, `checks`, `fixers`.
//!
//! A nested file speaks about its OWN subtree: its globs and its allow
//! paths are relative to its directory, and are rebased to the root as
//! they are read, so every matcher downstream sees one spelling, the one
//! reports use. An entry rebased under `sub/` can only match under
//! `sub/`, which is what "staleness judged within the declaring file's
//! subtree" comes to.
//!
//! The chain starts at the directory the run starts in and never reads
//! above it (`src/config:V88`): a node checked on its own -- from a
//! crates.io package, or with `cd node && xnl check` -- has only its own
//! file and the defaults, and its verdict must not depend on where the
//! tree happens to sit.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::Path;

use super::{Config, FILE, Verb, parse};

#[cfg(test)]
mod tests;

/// Why the tree of configs was refused. Exit 2, like any config error
/// (`src/cli:V24`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeError {
    /// The `xenolith.toml` at fault, root relative.
    pub file: String,
    /// What is wrong with it.
    pub message: String,
}

impl fmt::Display for TreeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for TreeError {}

/// Every `xenolith.toml` a run reads, and the effective config of each
/// directory holding one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tree {
    /// The root's config: the file at the root, or the defaults.
    root: Config,
    /// Directory (root relative, no trailing `/`) -> (the file's own
    /// entries rebased to the root, the effective config there).
    nested: BTreeMap<String, (Config, Config)>,
}

/// The config file in `dir`, root relative: `xenolith.toml` at the root.
#[must_use]
pub fn file_in(dir: &str) -> String {
    if dir.is_empty() {
        FILE.to_owned()
    } else {
        format!("{dir}/{FILE}")
    }
}

impl Tree {
    /// A tree of one file, the root's.
    #[must_use]
    pub fn new(root: Config) -> Tree {
        Tree {
            root,
            nested: BTreeMap::new(),
        }
    }

    /// This tree with `config` read from `dir`'s `xenolith.toml`, its
    /// globs and allow paths still relative to `dir`.
    ///
    /// # Errors
    ///
    /// A [`TreeError`] naming both files when `dir`'s file declares a
    /// different `version` from the chain above it (`src/config:V70`).
    pub fn with(self, dir: &str, config: Config) -> Result<Tree, TreeError> {
        let mut layers: BTreeMap<String, Config> = self
            .nested
            .into_iter()
            .map(|(dir, (own, _))| (dir, own))
            .collect();
        let dir = clean(dir);
        if dir.is_empty() {
            return Tree::build(config, layers);
        }
        layers.insert(dir.clone(), rebased(&config, &dir));
        Tree::build(self.root, layers)
    }

    /// Read the `xenolith.toml` of every directory between `root` and
    /// each of `files` (root relative), `root`'s own excluded: that one
    /// is `config`, loaded by the caller. A directory without the file
    /// adds nothing; a path reaching outside the root (`..`, absolute)
    /// adds nothing either, since nothing above the root is read. A file
    /// the configs above it exclude from `verb` is not read either: an
    /// excluded tree is never opened, its configs included.
    ///
    /// # Errors
    ///
    /// A [`TreeError`] naming the file that cannot be read, does not
    /// parse, or breaks `src/config:V70`.
    pub fn load<'p, I>(root: &Path, config: Config, verb: Verb, files: I) -> Result<Tree, TreeError>
    where
        I: IntoIterator<Item = &'p str>,
    {
        let mut dirs = BTreeSet::new();
        for file in files {
            if file.starts_with('/') || file.split('/').any(|seg| seg == "..") {
                continue;
            }
            let mut dir = parent(file);
            while !dir.is_empty() {
                dirs.insert(dir.to_owned());
                dir = parent(dir);
            }
        }
        // Each layer is merged ONCE, onto its parent's resolved config
        // (`src/config:B2`): `dirs` is sorted, so a directory comes
        // before everything beneath it -- and so the configs above a
        // file decide whether `verb` excludes it before it is opened
        // (`src/config:V79`, `src/config:B3`).
        let mut tree = Tree::new(config);
        for dir in dirs {
            let name = file_in(&dir);
            if tree.config_for(&name).excluded(verb, &name).is_some() {
                continue;
            }
            let path = root.join(&name);
            if !path.is_file() {
                continue;
            }
            let text = fs::read_to_string(&path).map_err(|e| TreeError {
                file: name.clone(),
                message: e.to_string(),
            })?;
            let config = parse(&text).map_err(|e| TreeError {
                file: name.clone(),
                message: if e.key.is_empty() {
                    e.message
                } else {
                    format!("{}: {}", e.key, e.message)
                },
            })?;
            let own = rebased(&config, &dir);
            tree.insert(dir, own)?;
        }
        Ok(tree)
    }

    /// The effective config for the root-relative `path`: the chain down
    /// to its directory, merged nearest last.
    #[must_use]
    pub fn config_for(&self, path: &str) -> &Config {
        let dir = self.nearest(path);
        self.nested
            .get(dir)
            .map_or(&self.root, |(_, effective)| effective)
    }

    /// The directory of the `xenolith.toml` nearest to `path` -- the
    /// file that governs it, `""` for the root's.
    #[must_use]
    pub fn nearest(&self, path: &str) -> &str {
        let mut dir = parent(path);
        while !dir.is_empty() {
            if let Some((found, _)) = self.nested.get_key_value(dir) {
                return found;
            }
            dir = parent(dir);
        }
        ""
    }

    /// Every file read, root first then by directory: its directory and
    /// its own entries, globs and allow paths rebased to the root.
    pub fn layers(&self) -> impl Iterator<Item = (&str, &Config)> {
        std::iter::once(("", &self.root)).chain(
            self.nested
                .iter()
                .map(|(dir, (own, _))| (dir.as_str(), own)),
        )
    }

    /// Resolve every layer's effective config. `BTreeMap` order puts a
    /// directory before everything beneath it, since a prefix sorts
    /// first, so each parent is resolved before its children.
    fn build(root: Config, layers: BTreeMap<String, Config>) -> Result<Tree, TreeError> {
        let mut tree = Tree::new(root);
        for (dir, own) in layers {
            tree.insert(dir, own)?;
        }
        Ok(tree)
    }

    /// Add `dir`'s layer (`own`, already rebased), merged onto the
    /// config effective above it. Every directory above `dir` that holds
    /// a file must already be in the tree.
    fn insert(&mut self, dir: String, own: Config) -> Result<(), TreeError> {
        let above = self.nearest(&format!("{dir}/{FILE}")).to_owned();
        let parent = self.config_for(&format!("{dir}/{FILE}"));
        if parent.version != own.version {
            return Err(TreeError {
                file: file_in(&dir),
                message: format!(
                    "declares version {}, but {} declares {}: every xenolith.toml in one \
                     merge chain declares the same version (src/config:V70)",
                    own.version,
                    file_in(&above),
                    parent.version
                ),
            });
        }
        let effective = merge(parent, &own);
        self.nested.insert(dir, (own, effective));
        Ok(())
    }
}

/// The directory part of a root-relative path, `""` at the top.
fn parent(path: &str) -> &str {
    path.rsplit_once('/').map_or("", |(dir, _)| dir)
}

/// `dir` without `./`, a trailing `/`, or empty segments.
fn clean(dir: &str) -> String {
    dir.split('/')
        .filter(|seg| !seg.is_empty() && *seg != ".")
        .collect::<Vec<_>>()
        .join("/")
}

/// `rel`, written in a file in `dir`, as the root spells it.
fn rebase(dir: &str, rel: &str) -> String {
    let rel = rel.strip_prefix("./").unwrap_or(rel);
    let rel = rel.trim_start_matches('/');
    format!("{dir}/{rel}")
}

/// `config` read from `dir` with its globs and allow paths rebased to the
/// root.
fn rebased(config: &Config, dir: &str) -> Config {
    let mut out = config.clone();
    for allow in &mut out.allow {
        allow.path = rebase(dir, &allow.path);
    }
    let lists = [
        &mut out.exclude.all,
        &mut out.exclude.check,
        &mut out.exclude.extract,
        &mut out.exclude.graph,
        &mut out.exclude.lint,
    ];
    for list in lists {
        for exclude in list.iter_mut() {
            exclude.glob = rebase(dir, &exclude.glob);
        }
    }
    out
}

/// `child` over `parent`: every scalar `child` set overrides, tables
/// deep-merge, entry lists append (`src/config` §I).
fn merge(parent: &Config, child: &Config) -> Config {
    let mut out = parent.clone();
    for key in &child.set {
        override_scalar(&mut out, child, key);
        out.set.insert(key.clone());
    }
    out.allow.extend(child.allow.iter().cloned());
    out.extract
        .rules
        .extend(child.extract.rules.iter().cloned());
    out.lint.all.extend(child.lint.all.iter().cloned());
    let pairs = [
        (&mut out.exclude.all, &child.exclude.all),
        (&mut out.exclude.check, &child.exclude.check),
        (&mut out.exclude.extract, &child.exclude.extract),
        (&mut out.exclude.graph, &child.exclude.graph),
        (&mut out.exclude.lint, &child.exclude.lint),
    ];
    for (into, from) in pairs {
        into.extend(from.iter().cloned());
    }
    for (guest, lint) in &child.lint.guests {
        let entry = out.lint.guests.entry(*guest).or_default();
        entry.checks.extend(lint.checks.iter().cloned());
        entry.fixers.extend(lint.fixers.iter().cloned());
        if lint.extend.is_some() {
            entry.extend = lint.extend;
        }
    }
    for (guest, threshold) in &child.threshold.guests {
        let entry = out.threshold.guests.entry(*guest).or_default();
        if threshold.max_lines.is_some() {
            entry.max_lines = threshold.max_lines;
        }
        if threshold.max_bytes.is_some() {
            entry.max_bytes = threshold.max_bytes;
        }
    }
    out
}

/// Copy the scalar at `key` from `child` into `out`. Keys of the tables
/// and lists [`merge`] folds field by field are left to it.
fn override_scalar(out: &mut Config, child: &Config, key: &str) {
    match key {
        "extract.layout" => out.extract.layout = child.extract.layout,
        "extract.root" => out.extract.root.clone_from(&child.extract.root),
        "extract.depth" => out.extract.depth = child.extract.depth,
        "extract.inactive_rules" => out.extract.inactive_rules = child.extract.inactive_rules,
        "extract.shell.strict" => out.extract.shell.strict = child.extract.shell.strict,
        "langs.unclaimed" => out.langs.unclaimed = child.langs.unclaimed,
        "langs.missing_guest" => out.langs.missing_guest = child.langs.missing_guest,
        "lint.hosts" => out.lint.hosts = child.lint.hosts,
        "lint.timeout" => out.lint.timeout = child.lint.timeout,
        "parse.host_errors" => out.parse.host_errors = child.parse.host_errors,
        "threshold.shell.allow" => {
            out.threshold
                .shell_allow
                .clone_from(&child.threshold.shell_allow);
        }
        "threshold.exec.max_args" => out.threshold.exec_max_args = child.threshold.exec_max_args,
        "threshold.exec.max_len" => out.threshold.exec_max_len = child.threshold.exec_max_len,
        "threshold.load.max_params" => {
            out.threshold.load_max_params = child.threshold.load_max_params;
        }
        "threshold.load.param_prefix" => {
            out.threshold
                .load_param_prefix
                .clone_from(&child.threshold.load_param_prefix);
        }
        // `lint.all`, `lint.<guest>.extend`, `threshold.<guest>.*`:
        // folded by `merge`.
        _ => {}
    }
}
