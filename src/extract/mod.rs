//! The extract engine: `xnl extract` as a library call (`src/extract`
//! §G, `src/extract:T22`).
//!
//! What gets extracted is decided ONCE, by the check engine
//! (`src:V152`): a site is extracted exactly when `xenolith::check`
//! flags it `xenolith`, so the two verbs cannot disagree about what a
//! xenolith is -- an allowed site, a trivial one, one `[threshold]`
//! relaxes, all stay where they are. What this engine adds is the
//! move itself:
//!
//! 1. targets -- the paths named, through [`crate::discover`]
//!    (`src:V57`, `src:V128`), each with the lines it asks for;
//! 2. sites -- the claiming host's [`Host::sites`], kept when check
//!    flagged them;
//! 3. placement -- [`place`], per field (`src/extract:V45`), and the
//!    collision suffix across the whole run (`src/extract:V47`);
//! 4. rewrite -- the host's own [`Host::rewrite`], one site at a time;
//! 5. asserts -- the result proven before anything is written: every
//!    load reads back, inlining every extract gives the host back
//!    (`src/extract:V4`), no extracted site is still there
//!    (`src/extract:V5`), and no file on disk is overwritten with other
//!    bytes (`src/extract:V6`).
//!
//! The answer is an [`Edit`]: the plan, as data. Rendering it as a diff
//! is [`Edit::diff`] (`src/extract:C15`); carrying it out is
//! [`write::apply`] (`--write`). Anything the engine cannot do safely is
//! a [`Refusal`] in the edit, never a silent skip and never a guess.
//!
//! * [`place`] -- where each extract goes, field by field, and which
//!   layer decided it (`src/extract:V45`).
//! * [`diff`] -- unified diffs of whole files.
//! * [`write`] -- `--write`.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use xenolith_lang_api::{Guest, Host, Invoke, LangId, Prelude, Site, shebang};

use crate::check::{self, CheckError, Langs, repo_name};
use crate::cli::EXIT_USAGE;
use crate::config::{Base, Config, SiteKey, Strict, Tree, TreeError, Verb};
use crate::discover::{DiscoverError, discover_with};
use crate::model::{Rule, Warning};
use crate::registry;

use self::place::{Ask, Field, Placed, Vars};

pub mod diff;
pub mod place;
pub mod write;

#[cfg(test)]
mod tests;

/// One `xnl extract` operand: a host file (or a directory of them), and
/// optionally the one line whose site to extract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The path as named.
    pub path: PathBuf,
    /// The 1-based line of the site; `None` is every site.
    pub line: Option<usize>,
}

/// What a run is asked to extract.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// At least one target; the CLI refuses none.
    pub targets: Vec<Target>,
    /// `--strict-hosts` (`src:V13`).
    pub strict_hosts: bool,
}

/// A file the edit creates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewFile {
    /// Repo-root relative.
    pub path: String,
    /// The whole content: prelude, then the body (languages/api/src/lens:V63).
    pub text: String,
    /// Whether it gets the executable bit.
    pub executable: bool,
    /// Already on disk with exactly these bytes: nothing to write
    /// (`src/extract:V5`, `src/extract:V6`).
    pub present: bool,
}

/// One host file's part of the edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostEdit {
    /// The host, repo-root relative.
    pub path: String,
    /// Its bytes as read.
    pub before: String,
    /// Its bytes once every extracted site loads its file.
    pub after: String,
    /// The extracts, sorted by path.
    pub extracts: Vec<NewFile>,
}

/// Something the run will not do, and why. Exit 2 (`src/cli:V24`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Refusal {
    /// The host file, repo-root relative.
    pub file: String,
    /// The site's line, or 0 for the whole file.
    pub line: usize,
    /// Why, naming the rule of the spec that forbids it.
    pub message: String,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            write!(f, "{}: {}", self.file, self.message)
        } else {
            write!(f, "{}:{}: {}", self.file, self.line, self.message)
        }
    }
}

/// The plan `xnl extract` prints, and `--write` carries out.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Edit {
    /// Each host that changes, sorted by path.
    pub hosts: Vec<HostEdit>,
    /// What was refused, sorted.
    pub refusals: Vec<Refusal>,
    /// `--verbose`: per site, each field and the layer deciding it
    /// (`src/extract:V48`), and why a site was left alone.
    pub explain: Vec<String>,
    /// Warnings the scan raised on the way (`src/cli` §I).
    pub warnings: Vec<Warning>,
}

impl Edit {
    /// The unified diff of every change, extracts before their host
    /// (the order `--write` uses, `src/extract:V84`), each extract
    /// announced as `removing xenolith → <path>` (`src/cli` §I).
    #[must_use]
    pub fn diff(&self) -> String {
        let mut out = String::new();
        for host in &self.hosts {
            for extract in &host.extracts {
                out.push_str("removing xenolith → ");
                out.push_str(&extract.path);
                out.push('\n');
            }
            for extract in host.extracts.iter().filter(|e| !e.present) {
                out.push_str(&diff::unified(
                    "/dev/null",
                    &format!("b/{}", extract.path),
                    "",
                    &extract.text,
                ));
            }
            out.push_str(&diff::unified(
                &format!("a/{}", host.path),
                &format!("b/{}", host.path),
                &host.before,
                &host.after,
            ));
        }
        out
    }

    /// 2 when anything was refused, else 1 when there is a change to
    /// make, else 0 (`src/cli:V24`: the highest wins).
    #[must_use]
    pub fn exit_code(&self) -> u8 {
        if self.refusals.is_empty() {
            u8::from(!self.hosts.is_empty())
        } else {
            EXIT_USAGE
        }
    }
}

/// Why a whole run was refused. Exit 2.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtractError {
    /// A named path was refused by discovery (`src:V57`, `src:V128`).
    Discover(DiscoverError),
    /// A nested `xenolith.toml` was refused (`src/config` §I).
    Config(TreeError),
    /// The check engine refused (`src:V152`).
    Check(CheckError),
    /// A named path lies outside the root.
    Outside(PathBuf),
    /// `<dir>:<line>`: a line names a site in one file.
    LineOnDir(PathBuf),
}

impl fmt::Display for ExtractError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExtractError::Discover(e) => e.fmt(f),
            ExtractError::Config(e) => e.fmt(f),
            ExtractError::Check(e) => e.fmt(f),
            ExtractError::Outside(path) => write!(
                f,
                "{}: outside the root: xnl extracts in the tree it runs in",
                path.display()
            ),
            ExtractError::LineOnDir(path) => write!(
                f,
                "{}: a line names a site in one file, and this names a directory",
                path.display()
            ),
        }
    }
}

impl std::error::Error for ExtractError {}

impl From<DiscoverError> for ExtractError {
    fn from(e: DiscoverError) -> ExtractError {
        ExtractError::Discover(e)
    }
}

impl From<TreeError> for ExtractError {
    fn from(e: TreeError) -> ExtractError {
        ExtractError::Config(e)
    }
}

impl From<CheckError> for ExtractError {
    fn from(e: CheckError) -> ExtractError {
        ExtractError::Check(e)
    }
}

/// Plan the extraction of `options.targets` under `config`.
///
/// # Errors
///
/// [`ExtractError`], exit 2, when the run as a whole cannot start: a
/// target discovery refuses, a config that does not load, a check
/// refusal. Anything narrower is a [`Refusal`] inside the edit.
pub fn extract(root: &Path, config: &Config, options: &Options) -> Result<Edit, ExtractError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    extract_with(root, config, options, &langs, &|| Command::new("git"))
}

/// A site check flagged, with everything needed to move it.
struct Planned<'a> {
    file: String,
    host: &'a dyn Host,
    guest: &'a dyn Guest,
    site: Site,
    line: usize,
    col: usize,
    placed: Placed,
    refused: Option<String>,
}

/// One host file the run read, and the sites it plans for it.
struct Read<'a> {
    text: String,
    host: &'a dyn Host,
}

/// [`extract`], with the languages and `git` supplied -- the seam the
/// tests use (`tests:V150`).
pub(crate) fn extract_with(
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
    // An excluded file is never read (`src/config:V79`), and says so
    // under `--verbose` (`src/extract:V80`).
    wanted.retain(|name, _| match excluded(&tree, name) {
        Some(why) => {
            edit.explain.push(format!("{name}: skipped: {why}"));
            false
        }
        None => true,
    });
    let names: Vec<String> = wanted.keys().cloned().collect();
    let flagged = flagged(root, config, options, langs, git, &names, &mut edit)?;
    let mut read = BTreeMap::new();
    let mut planned = Vec::new();
    for (name, lines) in &wanted {
        let Some(file) = scan(root, langs, name, &mut edit) else {
            continue;
        };
        let mut found = BTreeSet::new();
        for site in file.host.sites(&file.text).unwrap_or_default() {
            let (line, col) = position(&file.text, site.delim.open.start);
            if lines.as_ref().is_some_and(|lines| !lines.contains(&line)) {
                continue;
            }
            let key = (name.clone(), site.sink.clone(), line, col);
            if !flagged.contains(&key) {
                let why = left_alone(&tree, langs, name, &file.text, &site);
                edit.explain
                    .push(format!("{name}:{line}:{col} {}: skipped: {why}", site.sink));
                continue;
            }
            found.insert(line);
            if let Some(p) = plan(&tree, langs, config, name, file.host, site, line, col) {
                planned.push(p);
            }
        }
        for line in lines.iter().flatten().filter(|l| !found.contains(*l)) {
            edit.refusals.push(Refusal {
                file: name.clone(),
                line: *line,
                message: "no site `xnl check` flags here, so nothing to extract".to_owned(),
            });
        }
        read.insert(name.clone(), file);
    }
    collisions(&mut planned);
    for (name, file) in &read {
        let sites: Vec<&Planned<'_>> = planned.iter().filter(|p| &p.file == name).collect();
        if !sites.is_empty() {
            rewrite_file(root, name, file, &sites, &mut edit);
        }
    }
    edit.hosts.sort_by(|a, b| a.path.cmp(&b.path));
    edit.refusals.sort();
    Ok(edit)
}

/// Why `name` is not read at all, or `None` when it is: `[[exclude]]`
/// or `[extract] exclude` (`src/extract:V80`), or a `[check] exclude`
/// -- check never judges that file, so nothing in it is flagged, and
/// saying "may stay inline" of every site would be a guess.
fn excluded(tree: &Tree, name: &str) -> Option<String> {
    let config = tree.config_for(name);
    if let Some(e) = config.excluded(Verb::Extract, name) {
        return Some(format!(
            "excluded by `{}` ({}) (src/extract:V80)",
            e.glob, e.reason
        ));
    }
    config.excluded(Verb::Check, name).map(|e| {
        format!(
            "not judged: xnl check skips it, excluded by `{}` ({})",
            e.glob, e.reason
        )
    })
}

/// Why a site check did not flag is left where it is (`src/extract:V80`).
fn left_alone(tree: &Tree, langs: &Langs<'_>, name: &str, text: &str, site: &Site) -> String {
    let hash = check::body_hash(site.delim.body.of(text).unwrap_or_default());
    let key = SiteKey {
        path: name,
        sink: &site.sink,
        hash: &hash,
    };
    if let Some(allow) = tree.config_for(name).allowed(&key) {
        return format!("allowed by [[allow]] ({}) (src/extract:V80)", allow.reason);
    }
    if !langs.guests.iter().any(|g| g.id() == site.guest) {
        return format!("its guest, {}, is not in this build (src:V42)", site.guest);
    }
    "xnl check does not flag it, so it may stay inline".to_owned()
}

/// The files each target names, repo-root relative, each with the
/// lines asked for (`None`: every site).
fn targets(
    root: &Path,
    targets: &[Target],
    git: &dyn Fn() -> Command,
    edit: &mut Edit,
) -> Result<BTreeMap<String, Option<BTreeSet<usize>>>, ExtractError> {
    let mut wanted: BTreeMap<String, Option<BTreeSet<usize>>> = BTreeMap::new();
    for target in targets {
        let found = discover_with(root, std::slice::from_ref(&target.path), git)?;
        edit.warnings.extend(found.warnings);
        if target.line.is_some() && root.join(&target.path).is_dir() {
            return Err(ExtractError::LineOnDir(target.path.clone()));
        }
        for file in &found.files {
            if file.is_absolute() {
                return Err(ExtractError::Outside(target.path.clone()));
            }
            let entry = wanted
                .entry(repo_name(file))
                .or_insert_with(|| Some(BTreeSet::new()));
            match (target.line, entry.as_mut()) {
                (Some(line), Some(lines)) => {
                    lines.insert(line);
                }
                _ => *entry = None,
            }
        }
    }
    Ok(wanted)
}

/// The sites `xenolith::check` flags in `names`, as (file, sink, line,
/// col) -- the one decision of what is a xenolith (`src:V152`).
fn flagged(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
    names: &[String],
    edit: &mut Edit,
) -> Result<BTreeSet<(String, String, usize, usize)>, ExtractError> {
    // No file left means nothing to judge -- and an empty path list
    // would ask check for the whole tree.
    if names.is_empty() {
        return Ok(BTreeSet::new());
    }
    let check_options = check::Options {
        paths: names.iter().map(PathBuf::from).collect(),
        strict_hosts: options.strict_hosts,
    };
    let report = check::check_with(root, config, &check_options, langs, git)?;
    edit.warnings.extend(report.warnings().iter().cloned());
    Ok(report
        .violations()
        .iter()
        .filter(|v| v.rule == Rule::Xenolith)
        .map(|v| (repo_name(&v.file), v.sink.clone(), v.line, v.col))
        .collect())
}

/// The file's text and the host that reads it, or `None` when there is
/// nothing to extract from it; a file that cannot be read is refused.
fn scan<'a>(root: &Path, langs: &Langs<'a>, name: &str, edit: &mut Edit) -> Option<Read<'a>> {
    let head = fs::read(root.join(name)).map(|bytes| {
        let first = bytes.split(|b| *b == b'\n').next().unwrap_or_default();
        String::from_utf8_lossy(first).into_owned()
    });
    let head = head.unwrap_or_default();
    let host = langs
        .hosts
        .iter()
        .copied()
        .find(|host| host.claims(Path::new(name), &head))?;
    let text = fs::read(root.join(name))
        .map_err(|e| e.to_string())
        .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "not UTF-8".to_owned()));
    let refuse = |edit: &mut Edit, message: String| {
        edit.refusals.push(Refusal {
            file: name.to_owned(),
            line: 0,
            message,
        });
    };
    match text {
        Ok(text) => match host.sites(&text) {
            Ok(_) => Some(Read { text, host }),
            Err(e) => {
                refuse(edit, format!("did not parse as {}: {e}", host.id()));
                None
            }
        },
        Err(e) => {
            refuse(edit, format!("cannot be read: {e}"));
            None
        }
    }
}

/// Place one flagged site, or refuse it with the reason.
#[allow(clippy::too_many_arguments)] // one call site, every argument a stage input
fn plan<'a>(
    tree: &Tree,
    langs: &Langs<'a>,
    config: &Config,
    name: &str,
    host: &'a dyn Host,
    site: Site,
    line: usize,
    col: usize,
) -> Option<Planned<'a>> {
    let guest = langs
        .guests
        .iter()
        .copied()
        .find(|g| g.id() == site.guest)?;
    let effective = tree.config_for(name);
    let ask = Ask {
        host: host.id(),
        host_path: name,
        sink: &site.sink,
        guest: site.guest,
        ext: guest.extension(&site.env),
        placement: host.placement(&site).map_err(|e| e.to_string()),
        layout: effective.extract.layout,
        root: &effective.extract.root,
    };
    let rules = place::rules_for(tree, name);
    let (placed, refused) = match place::resolve(&ask, &rules) {
        Ok(placed) => {
            let refused = unsupported(&site, &placed, config, guest);
            (placed, refused)
        }
        Err(e) => (empty_placement(), Some(e)),
    };
    Some(Planned {
        file: name.to_owned(),
        host,
        guest,
        site,
        line,
        col,
        placed,
        refused,
    })
}

/// What the engine cannot yet extract without breaking
/// `src/extract:V4`, as the refusal (`src/extract` §I).
fn unsupported(site: &Site, placed: &Placed, config: &Config, guest: &dyn Guest) -> Option<String> {
    if !site.holes.is_empty() {
        return Some(format!(
            "{} host interpolation(s) in the body: each must become a named param \
             (languages/api/src/holes:V40), and no guest offers one yet \
             (languages/api/src/holes:T76); extract it by hand",
            site.holes.len()
        ));
    }
    if placed.companion.is_some() {
        return Some(
            "a rule sets `companion`, and companion creation arrives with src/extract:T51"
                .to_owned(),
        );
    }
    let rule_enforces = placed
        .prelude
        .as_ref()
        .is_some_and(|p| p.strict == Some(Strict::Enforce));
    let shell_enforces =
        guest.id() == LangId::Shell && config.extract.shell.strict == Strict::Enforce;
    if rule_enforces || shell_enforces {
        return Some(
            "`strict = \"enforce\"` adds options the body never ran under, a judgement \
             xnl does not make for you yet (src/extract §I)"
                .to_owned(),
        );
    }
    None
}

fn empty_placement() -> Placed {
    Placed {
        path: String::new(),
        name: String::new(),
        base: Base::Host,
        invoke: None,
        prelude: None,
        executable: None,
        companion: None,
        why: Vec::new(),
    }
}

/// `src/extract:V47` across the run: every path shared by two sites
/// takes each site's sink suffix; one still shared refuses both.
fn collisions(planned: &mut [Planned<'_>]) {
    let live: Vec<usize> = (0..planned.len())
        .filter(|&i| planned.get(i).is_some_and(|p| p.refused.is_none()))
        .collect();
    let mut paths: Vec<(String, String)> = live
        .iter()
        .filter_map(|&i| planned.get(i))
        .map(|p| (p.placed.path.clone(), p.site.sink.clone()))
        .collect();
    let still = place::disambiguate(&mut paths);
    for (at, (path, _)) in paths.into_iter().enumerate() {
        let Some(p) = live.get(at).and_then(|&i| planned.get_mut(i)) else {
            continue;
        };
        if still.contains(&at) {
            p.refused = Some(format!(
                "`{path}` is where another site goes too, even with the sink suffix \
                 (src/extract:V47, src/extract:V6)"
            ));
        }
        p.placed.path = path;
    }
}

/// One site's move, computed.
struct Move {
    load: String,
    invoke: Invoke,
    body: String,
    file: NewFile,
}

/// Rewrite one host for its planned sites and prove the result, or
/// refuse.
fn rewrite_file(root: &Path, name: &str, file: &Read<'_>, sites: &[&Planned<'_>], edit: &mut Edit) {
    let mut refuse = |line: usize, message: String| {
        edit.refusals.push(Refusal {
            file: name.to_owned(),
            line,
            message,
        });
    };
    let mut ready: Vec<(&Planned<'_>, Move)> = Vec::new();
    let mut refused = 0;
    for p in sites {
        let moved = match &p.refused {
            Some(why) => Err(why.clone()),
            None => movement(root, name, file, p),
        };
        match moved {
            Ok(m) => ready.push((p, m)),
            Err(why) => {
                refuse(p.line, why);
                refused += 1;
            }
        }
    }
    // All or nothing per file (`src/extract:V64`): a host half
    // extracted is a state nobody asked for, and a rerun would have to
    // guess which half it is looking at.
    if refused > 0 {
        if !ready.is_empty() {
            refuse(
                0,
                format!(
                    "left untouched with its {} other site(s): a file is extracted whole or \
                     not at all (src/extract:V64)",
                    ready.len()
                ),
            );
        }
        return;
    }
    // Back to front: a rewrite only moves bytes after its own site
    // (`src/extract:V64`).
    ready.sort_by_key(|(p, _)| std::cmp::Reverse(p.site.delim.open.start));
    let mut after = file.text.clone();
    for (p, m) in &ready {
        match p
            .host
            .rewrite(&after, &p.site, &m.invoke, Path::new(&m.load))
        {
            Ok(text) => after = text,
            Err(e) => {
                refuse(
                    p.line,
                    format!("the {} host cannot rewrite it: {e}", p.host.id()),
                );
                return;
            }
        }
    }
    let moves: Vec<&Move> = ready.iter().map(|(_, m)| m).collect();
    if let Err(why) = prove(file, &after, &ready) {
        refuse(0, why);
        return;
    }
    let mut extracts: Vec<NewFile> = moves.into_iter().map(|m| m.file.clone()).collect();
    extracts.sort_by(|a, b| a.path.cmp(&b.path));
    for (p, _) in &ready {
        explain(edit, p);
    }
    edit.hosts.push(HostEdit {
        path: name.to_owned(),
        before: file.text.clone(),
        after,
        extracts,
    });
}

/// The load, invoke and extract file of one placed site.
fn movement(root: &Path, name: &str, file: &Read<'_>, p: &Planned<'_>) -> Result<Move, String> {
    let raw = p.site.delim.body.of(&file.text).unwrap_or_default();
    let body = p
        .host
        .unescape(&p.site.delim, raw)
        .map_err(|e| format!("the body cannot be read as its guest reads it: {e}"))?;
    place::charset(&p.placed.path)?;
    let load = place::load_path(name, &p.placed.path, &p.placed.base);
    place::charset(&load)?;
    let invoke = invoke(p, &load)?;
    let prelude = prelude(p)?;
    let mut body_nl = body.clone();
    if !body_nl.is_empty() && !body_nl.ends_with('\n') {
        body_nl.push('\n');
    }
    let text = shebang::wrap(&body_nl, &prelude);
    if shebang::strip_strict(&text, &prelude) != body_nl {
        return Err(
            "the extract would not read back as its body (languages/api/src/lens:V63)".to_owned(),
        );
    }
    let path = p.placed.path.clone();
    write::guard(root, &path)?;
    let present = match fs::read(root.join(&path)) {
        Ok(bytes) if bytes == text.as_bytes() => true,
        Ok(_) => {
            return Err(format!(
                "{path} exists with other content, and extract never overwrites a file \
                 (src/extract:V6)"
            ));
        }
        Err(_) => false,
    };
    Ok(Move {
        load,
        invoke,
        body,
        file: NewFile {
            path,
            text,
            executable: p.placed.executable.unwrap_or_else(|| p.guest.executable()),
            present,
        },
    })
}

/// The argv the load runs: a rule's template rendered with the path as
/// loaded, else the guest's own `invoke` (`languages/api:V35`).
fn invoke(p: &Planned<'_>, load: &str) -> Result<Invoke, String> {
    let Some(template) = &p.placed.invoke else {
        return Ok(p.guest.invoke(Path::new(load)));
    };
    let (host_dir, host_file) = p.file.rsplit_once('/').unwrap_or(("", &p.file));
    let vars = Vars {
        name: p.placed.name.clone(),
        ext: p.guest.extension(&p.site.env).to_owned(),
        host_dir: host_dir.to_owned(),
        host_stem: stem_of(host_file),
        sink: p.site.sink.clone(),
        guest: p.site.guest.as_str().to_owned(),
        path: Some(load.to_owned()),
        path_stem: Some(stem_of(load)),
    };
    let argv = template
        .iter()
        .map(|word| place::render(word, &vars))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Invoke { argv })
}

/// `path` without the extension of its last component.
fn stem_of(path: &str) -> String {
    let file_at = path.rfind('/').map_or(0, |at| at + 1);
    match path.rfind('.') {
        Some(dot) if dot > file_at => path.get(..dot).unwrap_or(path).to_owned(),
        _ => path.to_owned(),
    }
}

/// The prelude the extract starts with: the guest's, with a rule's
/// shebang in its place when one is set.
fn prelude(p: &Planned<'_>) -> Result<Prelude, String> {
    let mut prelude = p.guest.prelude(&p.site.env);
    if let Some(line) = p.placed.prelude.as_ref().and_then(|r| r.shebang.as_deref()) {
        prelude.shebang = Some(
            shebang::parse(line)
                .ok_or_else(|| format!("rule prelude shebang `{line}` is not a `#!` line"))?,
        );
    }
    Ok(prelude)
}

/// The asserts before anything is written: every load reads back
/// (`languages/api/src/lens:V34` (c)), inlining gives the host back
/// (`src/extract:V4`), and no extracted site survives
/// (`src/extract:V5`).
fn prove(file: &Read<'_>, after: &str, ready: &[(&Planned<'_>, Move)]) -> Result<(), String> {
    let host = file.host;
    let loads = host.loads(after).map_err(|e| {
        format!(
            "cannot prove the rewrite lossless (src/extract:V4): the {} host cannot read its \
             loads back: {e}",
            host.id()
        )
    })?;
    let mut back = after.to_owned();
    // Loads in `ready` order are back to front already.
    for (p, m) in ready {
        let Some(load) = loads.iter().find(|l| l.path == Path::new(&m.load)) else {
            return Err(format!(
                "the {} host's load of {} is not read back as one (languages/api/src/lens:V34)",
                host.id(),
                m.load
            ));
        };
        back = host.inline(&back, load, &m.body).map_err(|e| {
            format!(
                "cannot prove the rewrite of `{}` lossless (src/extract:V4): {e}",
                p.site.sink
            )
        })?;
    }
    if words(&back) != words(&file.text) {
        return Err(
            "inlining the extracts does not give the host back, so the rewrite is not \
             lossless (src/extract:V4)"
                .to_owned(),
        );
    }
    let remaining = host
        .sites(after)
        .map_err(|e| format!("the rewritten host does not parse as {}: {e}", host.id()))?;
    for (p, _) in ready {
        let raw = p.site.delim.body.of(&file.text).unwrap_or_default();
        let survives = remaining
            .iter()
            .any(|s| s.sink == p.site.sink && s.delim.body.of(after) == Some(raw));
        if survives {
            return Err(format!(
                "`{}` is still a site after the rewrite, so a rerun would extract it again \
                 (src/extract:V5)",
                p.site.sink
            ));
        }
    }
    Ok(())
}

/// Text compared with whitespace normalised (`src/extract:V4`, until
/// `src/extract:T67`).
fn words(text: &str) -> Vec<&str> {
    text.split_whitespace().collect()
}

/// `--verbose`: each field of the site's placement and the layer that
/// decided it (`src/extract:V48`).
fn explain(edit: &mut Edit, p: &Planned<'_>) {
    let at = format!("{}:{}:{} {}", p.file, p.line, p.col, p.site.sink);
    for (field, layer) in &p.placed.why {
        let value = match field {
            Field::Path => p.placed.path.clone(),
            Field::Name => p.placed.name.clone(),
            Field::Base => match &p.placed.base {
                Base::Host => "host dir".to_owned(),
                Base::Root => "repo root".to_owned(),
                Base::Dir(dir) => dir.clone(),
            },
            Field::Invoke => p
                .placed
                .invoke
                .as_ref()
                .map_or_else(|| "guest default".to_owned(), |argv| argv.join(" ")),
            Field::Prelude => p
                .placed
                .prelude
                .as_ref()
                .and_then(|r| r.shebang.clone())
                .unwrap_or_else(|| "guest default".to_owned()),
            Field::Executable => p
                .placed
                .executable
                .unwrap_or_else(|| p.guest.executable())
                .to_string(),
            Field::Companion => p
                .placed
                .companion
                .clone()
                .unwrap_or_else(|| "none".to_owned()),
        };
        edit.explain
            .push(format!("{at}: {field} = {value} ({layer})"));
    }
}

/// 1-based line and column of byte `offset`, the column in characters,
/// as `xnl check` reports a site.
fn position(src: &str, offset: usize) -> (usize, usize) {
    let before = src.get(..offset).unwrap_or(src);
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, col)
}
