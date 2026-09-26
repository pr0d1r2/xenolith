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

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use super::{Config, FILE};

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

// RED stub (`.:T91`): the shape the tests call, and no nesting yet --
// the root governs every file, and nothing below it is read.
impl Tree {
    /// A tree of one file, the root's.
    #[must_use]
    pub fn new(root: Config) -> Tree {
        Tree {
            root,
            nested: BTreeMap::new(),
        }
    }

    /// This tree with `config` read from `dir`'s `xenolith.toml`.
    ///
    /// # Errors
    ///
    /// None yet.
    pub fn with(self, _dir: &str, _config: Config) -> Result<Tree, TreeError> {
        Ok(self)
    }

    /// Read every nested `xenolith.toml` on the way to `files`.
    ///
    /// # Errors
    ///
    /// None yet.
    pub fn load<'p, I>(_root: &Path, config: Config, _files: I) -> Result<Tree, TreeError>
    where
        I: IntoIterator<Item = &'p str>,
    {
        Ok(Tree::new(config))
    }

    /// The effective config for `path`.
    #[must_use]
    pub fn config_for(&self, _path: &str) -> &Config {
        &self.root
    }

    /// The directory of the nearest `xenolith.toml`.
    #[must_use]
    pub fn nearest(&self, _path: &str) -> &str {
        ""
    }

    /// Every file read.
    pub fn layers(&self) -> impl Iterator<Item = (&str, &Config)> {
        std::iter::once(("", &self.root))
    }
}
