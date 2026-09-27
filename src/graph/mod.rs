//! The graph engine: `xnl graph` as a library call (`src/graph:V7`).
//!
//! Which host loads which extract, and the two ways that can be wrong:
//! a load pointing at nothing (`dangling-load`) and an extract nothing
//! loads (`orphan-extract`). The parts live in their own modules:
//!
//! * [`roots`] -- where extracts live, so the orphan scan walks those
//!   directories and never the whole repository (`src/graph:V50`).
//! * [`resolve`] -- the file a load names, found without following a
//!   symlink (`src/graph:V72`).
//! * [`scan`] -- one run over the candidates: edges, dangling loads,
//!   and the orphan judgement at the end.
//!
//! Here: the types a caller sees, and the stages joined. Candidates come
//! from [`crate::discover`] (`src/discover:V57`, `src/discover:V128`), minus what
//! `[[exclude]]` and `[graph] exclude` skip (`src/config:V79`); every
//! stage reads the config effective for the file at hand, as `check`
//! does. `src/cli` renders the [`Graph`] and maps its exit code.

use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use serde_json::{Value, json};
use xenolith_lang_api::LangId;

use crate::check::{Langs, repo_name};
use crate::cli::EXIT_USAGE;
use crate::config::tree::file_in;
use crate::config::{Config, Tree, TreeError, Verb};
use crate::discover::{DiscoverError, discover_with};
use crate::model::{Report, SCHEMA};
use crate::registry;

pub mod resolve;
pub mod roots;
pub mod scan;

use self::scan::Scan;

#[cfg(test)]
mod tests;

/// The warning naming a host language whose `loads` this build cannot
/// answer, which leaves orphans unjudged (`src/graph` §I, orphan).
pub const LOADS_UNSUPPORTED: &str = "loads-unsupported";

/// What a run is asked to look at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The paths named; empty means every tracked file (`src/discover:V57`), and
    /// only then are orphans judged.
    pub paths: Vec<PathBuf>,
    /// `--strict-hosts` (`src/check:V13`).
    pub strict_hosts: bool,
}

/// One load that resolves: host file → extract (`src/graph` §I json).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    /// The host file, repo-root relative.
    pub host: PathBuf,
    /// The sink holding the load; empty until `LoadRef` names it.
    pub sink: String,
    /// 1-based line of the load.
    pub line: usize,
    /// 1-based column of the load, in characters.
    pub col: usize,
    /// The extract loaded, repo-root relative.
    pub extract: PathBuf,
    /// The extract's language, as the load says.
    pub guest: LangId,
    /// Holes passed as params; empty until `LoadRef` carries them.
    pub params: Vec<String>,
}

impl Edge {
    fn to_value(&self) -> Value {
        json!({
            "host": self.host.display().to_string(),
            "sink": self.sink,
            "line": self.line,
            "col": self.col,
            "extract": self.extract.display().to_string(),
            "guest": self.guest.as_str(),
            "params": self.params,
        })
    }
}

/// What `xnl graph` found: the edges, sorted (host, line, col,
/// extract), and the violations and warnings in a [`Report`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Graph {
    /// Every load that resolved.
    pub edges: Vec<Edge>,
    /// `dangling-load`, `orphan-extract`, `host-parse-error`; warnings.
    pub report: Report,
}

impl Graph {
    /// 0 clean, 1 any violation (`src/cli:V24`).
    #[must_use]
    pub fn exit_code(&self) -> i32 {
        self.report.exit_code()
    }

    /// The schema-1 envelope with `edges` beside `violations` and
    /// `warnings` (`src/graph` §I); keys sorted, one newline at the end.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut value: Value = serde_json::from_str(&self.report.to_json())
            .unwrap_or_else(|_| json!({ "schema": SCHEMA }));
        if let Value::Object(map) = &mut value {
            let edges = self.edges.iter().map(Edge::to_value).collect();
            map.insert("edges".to_owned(), Value::Array(edges));
        }
        let mut out = serde_json::to_string_pretty(&value)
            .unwrap_or_else(|_| format!("{{\"schema\": {SCHEMA}}}"));
        out.push('\n');
        out
    }
}

/// Why a run was refused rather than carried out; every variant exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// Discovery refused (`src/discover:V57`, `src/discover:V128`).
    Discover(DiscoverError),
    /// A nested `xenolith.toml` refused (`src/config` §I discovery).
    Config(TreeError),
    /// A file no host claims and no guest reads, under `--strict-hosts`
    /// or `[langs] unclaimed = "error"` (`src/check:V13`).
    Unclaimed {
        /// The file, as reports name it.
        file: PathBuf,
    },
    /// A named path outside the root.
    Outside {
        /// The path as discovery kept it.
        path: PathBuf,
    },
}

impl GraphError {
    /// The process exit code: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphError::Discover(e) => e.fmt(f),
            GraphError::Config(e) => e.fmt(f),
            GraphError::Unclaimed { file } => write!(
                f,
                "{}: host unsupported: no host in this build claims it and no guest reads \
                 it, so it cannot be graphed (src/check:V13)",
                file.display()
            ),
            GraphError::Outside { path } => write!(
                f,
                "{}: outside the root: xnl graphs the tree it runs in",
                path.display()
            ),
        }
    }
}

impl std::error::Error for GraphError {}

impl From<DiscoverError> for GraphError {
    fn from(e: DiscoverError) -> GraphError {
        GraphError::Discover(e)
    }
}

impl From<TreeError> for GraphError {
    fn from(e: TreeError) -> GraphError {
        GraphError::Config(e)
    }
}

/// Graph the tree at `root` under `config` (`src/graph:V7`).
///
/// # Errors
///
/// [`GraphError`], exit 2: discovery or a nested config refused, a named
/// path outside the root, or an unclaimed file under strict hosts.
pub fn graph(root: &Path, config: &Config, options: &Options) -> Result<Graph, GraphError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    graph_with(root, config, options, &langs, &|| Command::new("git"))
}

/// [`graph`] with the languages and `git` supplied by the caller -- the
/// seam the tests use (`tests:V150`).
pub(crate) fn graph_with(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
) -> Result<Graph, GraphError> {
    let candidates = discover_with(root, &options.paths, git)?;
    if let Some(path) = candidates.files.iter().find(|f| outside(f)) {
        return Err(GraphError::Outside { path: path.clone() });
    }
    let names: Vec<String> = candidates.files.iter().map(|f| repo_name(f)).collect();
    let tree = Tree::load(
        root,
        config.clone(),
        Verb::Graph,
        names.iter().map(String::as_str),
    )?;
    let mut scan = Scan::new(root, langs);
    for warning in candidates.warnings {
        scan.report.warn(warning);
    }
    for (dir, _) in tree.layers() {
        scan.roots.config(tree.config_for(&file_in(dir)));
    }
    for (file, name) in candidates.files.iter().zip(&names) {
        let config = tree.config_for(name);
        if config.excluded(Verb::Graph, name).is_some() {
            continue;
        }
        scan.file(file, name, config, options.strict_hosts)?;
    }
    Ok(scan.finish(options.paths.is_empty()))
}

/// A path discovery kept outside the root: absolute, or climbing out.
fn outside(path: &Path) -> bool {
    path.is_absolute() || matches!(path.components().next(), Some(Component::ParentDir))
}
