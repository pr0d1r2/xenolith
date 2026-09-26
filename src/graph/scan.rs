//! One graph run over the candidates (`src/graph:V7`, `src/graph` §I).
//!
//! Per file: a host that claims it is asked for its loads, each resolved
//! to an edge or a `dangling-load`, and for its sites, whose placement
//! dirs become extract roots; a guest that reads it makes it an extract.
//! At the end, and only when the whole tree was read and every host's
//! loads are known, an extract under a root that no edge reaches is an
//! `orphan-extract`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::Read as _;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{DelimKind, Error, Guest, Host, LangId, LoadRef, shebang};

use super::roots::Roots;
use super::{Edge, Graph, GraphError, LOADS_UNSUPPORTED, resolve};
use crate::check::{HOST_PARSE_ERROR, Langs, UNCLAIMED};
use crate::config::{Config, Policy};
use crate::model::{Direction, Fix, Report, Rule, Violation, Warning};

#[cfg(test)]
mod tests;

/// The state of one run.
pub(crate) struct Scan<'a> {
    root: &'a Path,
    langs: &'a Langs<'a>,
    /// Violations and warnings so far.
    pub(crate) report: Report,
    /// Where extracts live (`src/graph:V50`).
    pub(crate) roots: Roots,
    edges: Vec<Edge>,
    /// Every extract an edge reaches.
    loaded: BTreeSet<String>,
    /// Files a compiled-in guest reads, with that guest.
    extracts: Vec<(String, LangId)>,
    /// Host languages whose `loads` is unsupported, with a file count.
    unknown: BTreeMap<LangId, usize>,
    /// A claimed host file was not read, so its loads are unknown.
    unread: bool,
}

impl<'a> Scan<'a> {
    pub(crate) fn new(root: &'a Path, langs: &'a Langs<'a>) -> Scan<'a> {
        Scan {
            root,
            langs,
            report: Report::new(),
            roots: Roots::default(),
            edges: Vec::new(),
            loaded: BTreeSet::new(),
            extracts: Vec::new(),
            unknown: BTreeMap::new(),
            unread: false,
        }
    }

    /// One candidate, under the config effective for it.
    pub(crate) fn file(
        &mut self,
        file: &Path,
        name: &str,
        config: &Config,
        strict_hosts: bool,
    ) -> Result<(), GraphError> {
        let head = head(&self.root.join(file));
        let hosts: Vec<&dyn Host> = self
            .langs
            .hosts
            .iter()
            .copied()
            .filter(|host| host.claims(file, &head))
            .collect();
        let reader = reader(self.langs.guests, file, &head);
        if hosts.is_empty() && reader.is_none() {
            return self.unclaimed(config, strict_hosts, name);
        }
        if let Some(guest) = reader {
            self.extracts.push((name.to_owned(), guest));
        }
        if hosts.is_empty() {
            return Ok(());
        }
        let text = fs::read(self.root.join(file))
            .map_err(|e| e.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "not UTF-8".to_owned()));
        for host in hosts {
            match &text {
                Ok(src) => self.host(host, name, src, config),
                Err(detail) => self.host_error(config, host.id(), name, detail),
            }
        }
        Ok(())
    }

    /// A host file's loads and placements.
    fn host(&mut self, host: &dyn Host, name: &str, src: &str, config: &Config) {
        match host.loads(src) {
            Ok(loads) => {
                for load in &loads {
                    self.load(host.id(), name, src, load);
                }
            }
            Err(Error::Unsupported { .. }) => {
                *self.unknown.entry(host.id()).or_default() += 1;
            }
            Err(e) => return self.host_error(config, host.id(), name, &e.to_string()),
        }
        // A site the host cannot place adds no root (`src/graph` §I).
        for site in host.sites(src).unwrap_or_default() {
            if let Ok(placement) = host.placement(&site) {
                self.roots.placement(&placement.dir, name);
            }
        }
    }

    /// One load: an edge, or a `dangling-load` at the load.
    fn load(&mut self, host: LangId, name: &str, src: &str, load: &LoadRef) {
        let (line, col) = position(src, load.span.start);
        match resolve::resolve(self.root, name, &load.path) {
            Ok(extract) => {
                self.loaded.insert(extract.clone());
                self.edges.push(Edge {
                    host: PathBuf::from(name),
                    sink: String::new(),
                    line,
                    col,
                    extract: PathBuf::from(extract),
                    guest: load.guest,
                    params: Vec::new(),
                });
            }
            Err(why) => self.report.push(Violation {
                rule: Rule::DanglingLoad,
                file: PathBuf::from(name),
                line,
                col,
                host,
                guest: load.guest,
                sink: String::new(),
                site: DelimKind::ArgvString,
                why: format!(
                    "the load of `{}` does not resolve: {why}",
                    load.path.display()
                ),
                directions: vec![Direction {
                    kind: Fix::Judgment,
                    action: format!(
                        "restore the extract, or point the load in {name} at the file it \
                         meant"
                    ),
                }],
            }),
        }
    }

    /// `[parse] host_errors` for a claimed file its host could not read
    /// (`languages:V78`). Its loads are unknown whatever the policy.
    fn host_error(&mut self, config: &Config, host: LangId, name: &str, detail: &str) {
        self.unread = true;
        let message = format!("{name} did not parse as {host}, so its loads are unknown: {detail}");
        match config.parse.host_errors {
            Policy::Ignore => {}
            Policy::Warn => self.report.warn(Warning {
                code: HOST_PARSE_ERROR.to_owned(),
                file: Some(PathBuf::from(name)),
                message,
            }),
            Policy::Error => self.report.push(Violation {
                rule: Rule::HostParseError,
                file: PathBuf::from(name),
                line: 1,
                col: 1,
                host,
                guest: host,
                sink: String::new(),
                site: DelimKind::ArgvString,
                why: message,
                directions: vec![Direction {
                    kind: Fix::Judgment,
                    action: format!("fix the {host} syntax error in {name}"),
                }],
            }),
        }
    }

    /// A file nothing graphs (`src:V13`).
    fn unclaimed(
        &mut self,
        config: &Config,
        strict_hosts: bool,
        name: &str,
    ) -> Result<(), GraphError> {
        let policy = if strict_hosts {
            Policy::Error
        } else {
            config.langs.unclaimed
        };
        match policy {
            Policy::Ignore => Ok(()),
            Policy::Warn => {
                self.report.warn(Warning {
                    code: UNCLAIMED.to_owned(),
                    file: Some(PathBuf::from(name)),
                    message: format!(
                        "{name}: host unsupported: no host claims it and no guest reads it, \
                         so it was not graphed (src:V13)"
                    ),
                });
                Ok(())
            }
            Policy::Error => Err(GraphError::Unclaimed {
                file: PathBuf::from(name),
            }),
        }
    }

    /// The orphan judgement, then the graph (`src/graph:V7`). `whole` is
    /// a run over every tracked file.
    pub(crate) fn finish(mut self, whole: bool) -> Graph {
        for (lang, count) in &self.unknown {
            self.report.warn(Warning {
                code: LOADS_UNSUPPORTED.to_owned(),
                file: None,
                message: format!(
                    "{lang} cannot report its loads in this build ({count} file(s)), so no \
                     orphan-extract was judged: an extract only it loads would be reported \
                     wrongly (src/graph:V7)"
                ),
            });
        }
        if whole && self.unknown.is_empty() && !self.unread {
            for (name, guest) in &self.extracts {
                let Some(prefix) = self.roots.covering(name) else {
                    continue;
                };
                if !self.loaded.contains(name) {
                    self.report.push(orphan(name, *guest, prefix));
                }
            }
        }
        let mut edges = self.edges;
        edges.sort_by(|a, b| {
            (&a.host, a.line, a.col, &a.extract).cmp(&(&b.host, b.line, b.col, &b.extract))
        });
        Graph {
            edges,
            report: self.report,
        }
    }
}

/// An `orphan-extract` at the extract itself.
fn orphan(name: &str, guest: LangId, prefix: &str) -> Violation {
    Violation {
        rule: Rule::OrphanExtract,
        file: PathBuf::from(name),
        line: 1,
        col: 1,
        host: guest,
        guest,
        sink: String::new(),
        site: DelimKind::ArgvString,
        why: format!("no host loads this {guest} extract under the extract root `{prefix}`"),
        directions: vec![
            Direction {
                kind: Fix::Judgment,
                action: "load it from the host that should run it, or delete it if nothing \
                         does"
                    .to_owned(),
            },
            Direction {
                kind: Fix::Judgment,
                action: format!(
                    "or keep it with a reason: [graph] exclude = [{{ glob = \"{name}\", \
                     reason = \"...\" }}]"
                ),
            },
        ],
    }
}

/// The compiled-in guest that reads `file`, if any (`src/lint` §I
/// targets): a shebang decides, and one naming no compiled-in guest
/// decides "none"; without one, the extension a guest gives extracts.
pub(crate) fn reader(guests: &[&dyn Guest], file: &Path, head: &str) -> Option<LangId> {
    if let Some(bang) = shebang::parse(head) {
        let id = shebang::guest_of(&bang)?;
        return guests.iter().any(|g| g.id() == id).then_some(id);
    }
    let ext = file.extension()?.to_str()?;
    let env = xenolith_lang_api::GuestEnv::default();
    guests
        .iter()
        .find(|g| g.extension(&env) == ext)
        .map(|g| g.id())
}

/// 1-based line and column of byte `offset`, the column in characters.
pub(crate) fn position(src: &str, offset: usize) -> (usize, usize) {
    let before = src.get(..offset).unwrap_or(src);
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, col)
}

/// The first line of the file, read without reading the rest.
pub(crate) fn head(path: &Path) -> String {
    let mut buf = Vec::new();
    if let Ok(file) = fs::File::open(path) {
        let _ = file.take(1024).read_to_end(&mut buf);
    }
    let first = buf.split(|b| *b == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(first).into_owned()
}
