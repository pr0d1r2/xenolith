//! The check engine: `xnl check` as a library call (`src/check:V152`).
//!
//! ONE pipeline, stage by stage, each stage owned elsewhere and only
//! JOINED here:
//!
//! 1. candidates -- [`crate::discover`] (`src/discover:V57`, `src/discover:V128`), minus
//!    what `[[exclude]]` and `[check] exclude` skip (`src/config:V79`);
//!    an excluded file is never opened.
//! 2. claims -- every registry host is offered every candidate
//!    (`languages:V56`); a file nobody claims is unclaimed (`src/check:V13`).
//! 3. sites -- `Host::sites`, from the grammar (`languages:V2`).
//! 4. guest -- the site's guest from the registry; compiled out, it goes
//!    to `[langs] missing_guest` and is never guessed about (`src/check:V42`).
//! 5. verdict -- `Guest::trivial` over the body `Host::unescape` gives
//!    (`languages/api/src/lens:V39`), then `[threshold]`, which only
//!    RELAXES (`src/config:V55`): a trivial body is never flagged. A body
//!    its host runs line by line may be judged a line at a time, up to
//!    the host's `max_lines` (`src/config:V240`).
//! 6. allow -- `[[allow]]` by path, sink and body hash (`src/config:V10`);
//!    an entry matching no site is `stale-allow` (`src/config:V9`).
//! 7. report -- [`Violation`]s (`src:V1`) into a [`Report`], which keeps
//!    them sorted (`src:V11`).
//!
//! Every stage from 1 on reads the config EFFECTIVE for the file at hand:
//! the root's, merged with each `xenolith.toml` on the way down to the
//! file's directory ([`Tree`], `src/config` §I discovery). The caller
//! hands in the root's; the nested ones are read here, since which
//! directories matter is known only once discovery has run.
//!
//! `src/cli` renders the report and maps exit codes; nothing here writes
//! to a stream or exits.

use std::collections::BTreeSet;
use std::fmt;
use std::fs;
use std::io::Read as _;
use std::path::{Component, Path, PathBuf};
use std::process::Command;

use xenolith_lang_api::{DelimKind, Error, Guest, GuestEnv, Host, LangId, Site, shebang};

use crate::cli::EXIT_USAGE;
use crate::config::tree::file_in;
use crate::config::{self, Allow, Config, Policy, SiteKey, Tree, TreeError, Verb};
use crate::discover::{DiscoverError, discover_with};
use crate::model::{Direction, Fix, Report, Rule, Violation, Warning};
use crate::registry::{self, MissingGuest};

#[cfg(test)]
mod tests;

/// The config file `xnl check` reads at the root (`src/config:C16`), and
/// in any directory beneath it (`src/config` §I, discovery).
pub const CONFIG_FILE: &str = config::FILE;

/// The warning code for a claimed file whose host could not parse it,
/// under `[parse] host_errors = "warn"` (`languages:V78`).
pub const HOST_PARSE_ERROR: &str = "host-parse-error";

/// The warning code for a file no host claims, under `[langs] unclaimed
/// = "warn"` (`src/check:V13`).
pub const UNCLAIMED: &str = "host-unsupported";

/// What stands in for a host interpolation (a hole) when a body is
/// handed to its guest. A hole is HOST syntax -- nix `${...}`, pkl `\(...)` --
/// and the guest's grammar would read it as its own, or fail on it; one
/// plain word keeps the body's shape (a word stays a word, a command
/// stays a command). It is in place before the host's `unescape` runs
/// (`languages/api/src/lens:V39`), which then sees host escapes and
/// plain text only.
const HOLE: &str = "XNL_HOLE";

/// What a run is asked to look at.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Options {
    /// The paths named, repo-root relative; empty means every tracked
    /// file (`src/discover:V57`).
    pub paths: Vec<PathBuf>,
    /// `--strict-hosts`: an unclaimed file refuses the run, whatever
    /// `[langs] unclaimed` says (`src/check:V13`).
    pub strict_hosts: bool,
}

/// Why a run was refused rather than carried out. Every variant is exit
/// 2 (`src/cli:V24`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// Discovery refused (`src/discover:V57`, `src/discover:V128`).
    Discover(DiscoverError),
    /// A `xenolith.toml` below the root cannot be read, does not parse,
    /// or declares another version than the chain above it
    /// (`src/config` §I discovery, `src/config:V70`).
    Config(TreeError),
    /// A site's guest is compiled out and `[langs] missing_guest` is
    /// `error` (`src/check:V42`).
    MissingGuest(MissingGuest),
    /// No compiled-in host claims a candidate, under `--strict-hosts` or
    /// `[langs] unclaimed = "error"` (`src/check:V13`).
    Unclaimed {
        /// The file, as reports name it.
        file: PathBuf,
        /// The language its extension names, when this build lacks that
        /// language's host (`src:V30`: say which feature would bring it).
        missing: Option<LangId>,
    },
    /// A path named on the command line lies outside the root, so no
    /// config, allow or exclude of this run can speak about it.
    Outside {
        /// The path as it was named.
        path: PathBuf,
        /// The root the run checks.
        root: PathBuf,
    },
}

impl CheckError {
    /// The process exit code: always 2.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for CheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CheckError::Discover(e) => e.fmt(f),
            CheckError::Config(e) => e.fmt(f),
            CheckError::MissingGuest(e) => e.fmt(f),
            CheckError::Outside { path, root } => write!(
                f,
                "{}: outside the root {}: xnl checks the tree it runs in; name files \
                 under it, or run xnl from a directory that holds this one",
                path.display(),
                root.display()
            ),
            CheckError::Unclaimed { file, missing } => {
                write!(
                    f,
                    "{}: host unsupported: no host in this build claims it",
                    file.display()
                )?;
                // Only a feature that exists is named (`src:V30`, `src/check:B10`).
                match missing.map(|id| (id, registry::existing_feature(id))) {
                    Some((id, Some(feature))) => {
                        write!(
                            f,
                            "; {id} is compiled out, rebuild with feature `{feature}`"
                        )?;
                    }
                    Some((id, None)) => write!(
                        f,
                        "; xenolith has no support for {id} in this build: no crate provides it yet"
                    )?,
                    None => {}
                }
                f.write_str(" (src/check:V13)")
            }
        }
    }
}

impl std::error::Error for CheckError {}

impl From<DiscoverError> for CheckError {
    fn from(e: DiscoverError) -> CheckError {
        CheckError::Discover(e)
    }
}

impl From<TreeError> for CheckError {
    fn from(e: TreeError) -> CheckError {
        CheckError::Config(e)
    }
}

impl From<MissingGuest> for CheckError {
    fn from(e: MissingGuest) -> CheckError {
        CheckError::MissingGuest(e)
    }
}

/// Check the tree at `root` under `config` (`src/check:V152`).
///
/// # Errors
///
/// [`CheckError`], exit 2: discovery refused, a nested `xenolith.toml`
/// refused, or a site's guest is compiled out under `[langs]
/// missing_guest = "error"`.
pub fn check(root: &Path, config: &Config, options: &Options) -> Result<Report, CheckError> {
    let langs = Langs {
        hosts: registry::hosts(),
        guests: registry::guests(),
    };
    check_with(root, config, options, &langs, &|| Command::new("git"))
}

/// The languages a run judges with: the registry's in production, fakes
/// in the tests, which is what lets the engine be tested in every
/// feature subset (`src:V30`).
pub(crate) struct Langs<'a> {
    pub(crate) hosts: &'a [&'a dyn Host],
    pub(crate) guests: &'a [&'a dyn Guest],
}

/// One site the run saw, for `[[allow]]` staleness (`src/config:V9`).
struct Seen {
    path: String,
    sink: String,
    hash: String,
    at: Located,
}

/// Where a finding is, and in whose languages (`src:V1`).
#[derive(Clone)]
struct Located {
    file: PathBuf,
    line: usize,
    col: usize,
    host: LangId,
    guest: LangId,
    sink: String,
    site: DelimKind,
}

/// [`check`], with the languages and the `git` command supplied by the
/// caller -- the seam the tests use (`tests:V150`).
pub(crate) fn check_with(
    root: &Path,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    git: &dyn Fn() -> Command,
) -> Result<Report, CheckError> {
    let paths = options
        .paths
        .iter()
        .map(|path| under_root(root, path))
        .collect::<Result<Vec<_>, _>>()?;
    let candidates = discover_with(root, &paths, git)?;
    let names: Vec<String> = candidates.files.iter().map(|f| repo_name(f)).collect();
    let tree = Tree::load(
        root,
        config.clone(),
        Verb::Check,
        names.iter().map(String::as_str),
    )?;
    let mut report = Report::new();
    for warning in candidates.warnings {
        report.warn(warning);
    }
    let mut seen = Vec::new();
    let mut scanned = BTreeSet::new();
    for file in &candidates.files {
        let name = repo_name(file);
        let config = tree.config_for(&name);
        if config.excluded(Verb::Check, &name).is_some() {
            continue;
        }
        let head = head(&root.join(file));
        let claimers: Vec<&dyn Host> = langs
            .hosts
            .iter()
            .copied()
            .filter(|host| host.claims(file, &head))
            .collect();
        if claimers.is_empty() {
            unclaimed(&mut report, config, options, langs, &name)?;
            continue;
        }
        let text = fs::read(root.join(file))
            .map_err(|e| e.to_string())
            .and_then(|bytes| String::from_utf8(bytes).map_err(|_| "not UTF-8".to_owned()));
        for host in claimers {
            let parsed = match &text {
                Ok(src) => host
                    .sites(src)
                    .map(|sites| (src, sites))
                    .map_err(|e| e.to_string()),
                Err(e) => Err(e.clone()),
            };
            match parsed {
                Ok((src, sites)) => {
                    scanned.insert(name.clone());
                    for site in &sites {
                        judge_site(
                            root,
                            &mut report,
                            &mut seen,
                            &tree,
                            langs,
                            host,
                            &name,
                            src,
                            site,
                        )?;
                    }
                }
                Err(detail) => host_error(&mut report, config, host.id(), &name, &detail),
            }
        }
    }
    let judged = Judged {
        scanned: &scanned,
        candidates: &names.iter().cloned().collect(),
        whole_tree: options.paths.is_empty(),
    };
    stale_allows(&mut report, &tree, root, &seen, &judged, langs);
    if judged.whole_tree {
        stale_excludes(&mut report, &tree, &names);
    }
    Ok(report)
}

/// Stage 2 for a file no host claims (`src/check:V13`): never scanned, and by
/// default never mentioned -- most of a tree (docs, images, lockfiles)
/// is in no language xenolith hosts. `warn` says so per file;
/// `--strict-hosts` or `error` refuses the run, naming the feature that
/// would bring the host when the extension names a language this build
/// lacks (`src:V30`).
fn unclaimed(
    report: &mut Report,
    config: &Config,
    options: &Options,
    langs: &Langs<'_>,
    name: &str,
) -> Result<(), CheckError> {
    let policy = if options.strict_hosts {
        Policy::Error
    } else {
        config.langs.unclaimed
    };
    let missing = Path::new(name)
        .extension()
        .and_then(|ext| LangId::from_name(&ext.to_string_lossy()))
        .filter(|id| !langs.hosts.iter().any(|host| host.id() == *id));
    match policy {
        Policy::Ignore => Ok(()),
        Policy::Warn => {
            report.warn(Warning {
                code: UNCLAIMED.to_owned(),
                file: Some(PathBuf::from(name)),
                message: format!(
                    "{name}: host unsupported: no host in this build claims it, so it was \
                     not scanned (src/check:V13)"
                ),
            });
            Ok(())
        }
        Policy::Error => Err(CheckError::Unclaimed {
            file: PathBuf::from(name),
            missing,
        }),
    }
}

/// Stages 4 to 6 for one site, under the config effective for its file.
#[allow(clippy::too_many_arguments)] // one call site, every argument a stage input
fn judge_site(
    root: &Path,
    report: &mut Report,
    seen: &mut Vec<Seen>,
    tree: &Tree,
    langs: &Langs<'_>,
    host: &dyn Host,
    name: &str,
    src: &str,
    site: &Site,
) -> Result<(), CheckError> {
    let config = tree.config_for(name);
    let lang = host.id();
    let raw = site.delim.body.of(src).unwrap_or_default();
    let hash = body_hash(raw);
    let (line, col) = position(src, site.delim.open.start);
    let at = Located {
        file: PathBuf::from(name),
        line,
        col,
        host: lang,
        guest: site.guest,
        sink: site.sink.clone(),
        site: site.delim.kind.clone(),
    };
    seen.push(Seen {
        path: name.to_owned(),
        sink: site.sink.clone(),
        hash: hash.clone(),
        at: at.clone(),
    });
    let Some(guest) = langs.guests.iter().find(|g| g.id() == site.guest) else {
        if host.guest_by_shebang(src, site) {
            // The file's own `#!` line named a language this build
            // cannot check: said once, never a refusal (`src/check:V42`,
            // `src/check:B8`), and the rest of the report stands.
            let interpreter = host
                .unescape(&site.delim, &guest_text(src, site))
                .ok()
                .and_then(|body| shebang::parse(&body))
                .map(|line| line.resolved_interpreter().to_owned());
            report.warn(registry::missing_shebang_guest(
                site.guest,
                Path::new(name),
                interpreter.as_deref(),
            ));
            return Ok(());
        }
        let warning =
            registry::on_missing_guest(config.langs.missing_guest, site.guest, Path::new(name))?;
        if let Some(warning) = warning {
            report.warn(warning);
        }
        return Ok(());
    };
    // The guest judges what runs, not the host's bytes
    // (`languages/api/src/lens:V39`). A body the host cannot unescape is
    // flagged as such: judging the raw bytes instead would be a verdict
    // on text nothing executes.
    let why = match host.unescape(&site.delim, &guest_text(src, site)) {
        Ok(body) => site_verdict(*guest, site, lang, &body, config),
        Err(e) => Some(format!("unparseable {lang} string: {e}")),
    };
    let Some(why) = why else {
        return Ok(());
    };
    let key = SiteKey {
        path: name,
        sink: &site.sink,
        hash: &hash,
    };
    if config.allowed(&key).is_some() {
        return Ok(());
    }
    // The allow belongs in the file that governs the site, written
    // relative to it (`src/config` §I), so it holds when that subtree is
    // checked on its own (`src/config:V88`).
    let dir = tree.nearest(name);
    let rel = name
        .strip_prefix(dir)
        .and_then(|rest| rest.strip_prefix('/'))
        .unwrap_or(name);
    let file = file_in(dir);
    let violation = at.violation(
        Rule::Xenolith,
        why,
        vec![
            extract_direction(root, tree, langs, host, name, src, site, line),
            Direction {
                kind: Fix::Judgment,
                action: format!(
                    "or keep it inline, with a reason: [[allow]] path = \"{rel}\", sink = \"{}\", \
                     hash = \"{hash}\", reason = \"...\" in {file}",
                    site.sink
                ),
            },
        ],
    );
    report.push_site(violation, hash);
    Ok(())
}

/// The first direction of a xenolith (`src/check:B12`): `Mechanical` -- run
/// `xnl extract <file>:<line>` -- exactly when that command would move the
/// site, by the extract engine's own verdict ([`crate::extract::viable`]),
/// since a mechanical direction is a fix SARIF offers to apply
/// (`src/cli:V102`); else a `Judgment` carrying the refusal the command
/// would print.
#[allow(clippy::too_many_arguments)] // one call site, every argument a stage input
fn extract_direction(
    root: &Path,
    tree: &Tree,
    langs: &Langs<'_>,
    host: &dyn Host,
    name: &str,
    src: &str,
    site: &Site,
    line: usize,
) -> Direction {
    let command = format!("`xnl extract {}`", shell_word(&format!("{name}:{line}")));
    match crate::extract::viable(root, tree, langs, name, src, host, site) {
        Ok(()) => Direction {
            kind: Fix::Mechanical,
            action: format!("run {command}"),
        },
        Err(why) => Direction {
            kind: Fix::Judgment,
            action: format!("extract it to a file of its own by hand: {command} refuses it: {why}"),
        },
    }
}

/// `word` as ONE shell word, for a command a direction prints to be
/// pasted (`src/check:B7`): bare when every byte is one no POSIX shell treats
/// specially, else single-quoted, a `'` inside written `'\''`.
fn shell_word(word: &str) -> String {
    let plain = |c: char| c.is_ascii_alphanumeric() || "_-./:@%+,".contains(c);
    if !word.is_empty() && word.chars().all(plain) {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', "'\\''"))
}

impl Located {
    fn violation(&self, rule: Rule, why: String, directions: Vec<Direction>) -> Violation {
        Violation {
            rule,
            file: self.file.clone(),
            line: self.line,
            col: self.col,
            host: self.host,
            guest: self.guest,
            sink: self.sink.clone(),
            site: self.site.clone(),
            why,
            directions,
        }
    }
}

/// Stage 5 for a site: its body judged whole ([`body_verdict`]), then,
/// for a body its host runs line by line
/// ([`DelimKind::runs_line_by_line`]), relaxed to one verdict per line
/// when it holds no more lines than the host's `[threshold.<host>]
/// max_lines` (`src/config:V240`). Each line is then the guest's whole
/// program -- just gives each a fresh shell (`languages/ci/just:V180`).
///
/// Relaxing only (`src/config:V55`): a body that passes whole is final,
/// and the host is asked for its ceiling by id, never named here.
///
/// The ONE verdict (`src/check:V152`): `src/extract` judges a body read
/// back with it too (`src/extract:V270`, `src/graph:V100`), so `xnl
/// inline` and `inlineable-extract` cannot disagree with `xnl check`.
pub(crate) fn site_verdict(
    guest: &dyn Guest,
    site: &Site,
    host: LangId,
    body: &str,
    config: &Config,
) -> Option<String> {
    let whole = body_verdict(guest, &site.env, body, config)?;
    let Some(max) = config
        .line_ceiling(host)
        .filter(|_| site.delim.kind.runs_line_by_line())
    else {
        return Some(whole);
    };
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    let count = u64::try_from(lines.len()).unwrap_or(u64::MAX);
    if count < 2 {
        return Some(whole);
    }
    if count > max {
        return Some(format!(
            "{whole}; {count} lines, over threshold.{host} max_lines = {max}"
        ));
    }
    lines.iter().enumerate().find_map(|(i, line)| {
        body_verdict(guest, &site.env, line, config)
            .map(|why| format!("body line {}: {why}", i + 1))
    })
}

/// Stage 5 for one body: dialect syntax the guest cannot judge is its
/// own finding, a judgement rather than "unparseable"
/// (`languages/shells/shell:V138`); past that, [`verdict`].
fn body_verdict(guest: &dyn Guest, env: &GuestEnv, body: &str, config: &Config) -> Option<String> {
    guest
        .unsupported(body, env)
        .map(str::to_owned)
        .or_else(|| verdict(guest, body, config))
}

/// Stage 5: `None` when the body may stay inline, else the reason it may
/// not (`src:V1` `why`).
///
/// `trivial` first, and a trivial body is final: `[threshold]` only
/// RELAXES (`src/config:V55`). A body the guest cannot parse is flagged
/// (`languages:V77`) -- calling it trivial would leave broken code
/// inline. Past that, a guest that names its constructs is relaxed by
/// `[threshold.<guest>] allow` when every construct is listed; one that
/// names none (`Error::Unsupported`) by `max_lines` / `max_bytes`.
fn verdict(guest: &dyn Guest, body: &str, config: &Config) -> Option<String> {
    let id = guest.id();
    match guest.trivial(body) {
        Ok(true) => return None,
        Ok(false) => {}
        Err(e) => return Some(unparseable(id, &e)),
    }
    match guest.constructs(body) {
        Ok(names) => {
            let allow = config.construct_allow(id);
            let over: Vec<&str> = names
                .iter()
                .copied()
                .filter(|name| !allow.iter().any(|a| a == name))
                .collect();
            if over.is_empty() && !names.is_empty() {
                return None;
            }
            let listed = if over.is_empty() {
                "more than trivial".to_owned()
            } else {
                over.join(", ")
            };
            Some(format!(
                "non-trivial {id}: {listed}; a script belongs in its own file"
            ))
        }
        Err(Error::Unsupported { .. }) => {
            let text = body.trim();
            let lines = u64::try_from(text.lines().count()).unwrap_or(u64::MAX);
            let bytes = u64::try_from(text.len()).unwrap_or(u64::MAX);
            match config.size_ceiling(id) {
                Some((max_lines, max_bytes)) if lines <= max_lines && bytes <= max_bytes => None,
                Some((max_lines, max_bytes)) => Some(format!(
                    "non-trivial {id}: {lines} lines, {bytes} bytes, over \
                     threshold.{id} max_lines = {max_lines}, max_bytes = {max_bytes}"
                )),
                None => Some(format!("non-trivial {id}")),
            }
        }
        Err(e) => Some(unparseable(id, &e)),
    }
}

fn unparseable(id: LangId, e: &Error) -> String {
    format!("unparseable {id}: {e}")
}

/// `[parse] host_errors` for a claimed file its host could not read
/// (`languages:V78`).
fn host_error(report: &mut Report, config: &Config, host: LangId, name: &str, detail: &str) {
    let message = format!("{name} did not parse as {host}, so it was not checked: {detail}");
    match config.parse.host_errors {
        Policy::Ignore => {}
        Policy::Warn => report.warn(Warning {
            code: HOST_PARSE_ERROR.to_owned(),
            file: Some(PathBuf::from(name)),
            message,
        }),
        Policy::Error => report.push(
            unsited(PathBuf::from(name), 1, host, String::new()).violation(
                Rule::HostParseError,
                message,
                vec![Direction {
                    kind: Fix::Judgment,
                    action: format!("fix the {host} syntax error in {name}"),
                }],
            ),
        ),
    }
}

/// A location for a finding that is not at a site: a host file that did
/// not parse, an allow whose site is gone.
///
/// `src:V1` has a slot for a guest and a delimiter kind on EVERY
/// violation, and these findings have neither. Until the model says what
/// such a finding carries, the file's own language stands in for both
/// languages and the delimiter is `argv-string` -- the kind with the
/// least host syntax. Flagged to the spec owner rather than decided
/// here.
fn unsited(file: PathBuf, line: usize, host: LangId, sink: String) -> Located {
    Located {
        file,
        line,
        col: 1,
        host,
        guest: host,
        sink,
        site: DelimKind::ArgvString,
    }
}

/// Stage 6, second half: every `[[allow]]` no seen site matches is
/// `stale-allow` (`src/config:V9`), judged file by file: each
/// `xenolith.toml` read, its entries rebased to the root, so an entry in
/// a nested file can only be matched -- or found stale -- by a site in
/// that file's subtree (`src/config` §I).
///
/// Judged only where the run could have seen the site: an entry naming a
/// file this run scanned to sites, or -- on a whole-tree run -- one
/// naming no candidate at all (the file is gone or untracked). Never an
/// entry naming a candidate that was not scanned: excluded, claimed by
/// no host in this build (`src:V30`), or not parsed by its host
/// (`src/check:B2`) -- that file's sites were not looked at, so nothing about
/// them can be judged unmatched.
///
/// Reported AT the site when one with the entry's path and sink still
/// exists (the body changed, so the hash no longer matches), else at the
/// entry in the `xenolith.toml` that declares it.
fn stale_allows(
    report: &mut Report,
    tree: &Tree,
    root: &Path,
    seen: &[Seen],
    judged: &Judged<'_>,
    langs: &Langs<'_>,
) {
    let keys: Vec<SiteKey<'_>> = seen
        .iter()
        .map(|s| SiteKey {
            path: &s.path,
            sink: &s.sink,
            hash: &s.hash,
        })
        .collect();
    for (dir, layer) in tree.layers() {
        let file = file_in(dir);
        let config_text = fs::read_to_string(root.join(&file)).unwrap_or_default();
        for (index, allow) in layer.stale_allows(keys.iter().copied()) {
            if !judged.covers(&allow.path) {
                continue;
            }
            let drifted = seen
                .iter()
                .find(|s| s.path == allow.path && s.sink == allow.sink);
            let (at, why) = match drifted {
                Some(s) => (
                    s.at.clone(),
                    format!(
                        "[[allow]] #{} in {file} is stale: the body changed (hash {} now, {} \
                         allowed)",
                        index + 1,
                        s.hash,
                        allow.hash
                    ),
                ),
                None => (
                    gone(allow, index, &file, &config_text, root, langs),
                    format!(
                        "[[allow]] #{} is stale: no site `{}` in {}",
                        index + 1,
                        allow.sink,
                        allow.path
                    ),
                ),
            };
            report.push(at.violation(
                Rule::StaleAllow,
                why,
                vec![Direction {
                    kind: Fix::Judgment,
                    action: format!(
                        "update its hash if the new body should stay inline, or delete the \
                         entry from {file}"
                    ),
                }],
            ));
        }
    }
}

/// Every exclude entry, in every config read, whose glob matches none of
/// `tracked` is `stale-exclude` (`src/config:V79`, `src/check:B5`). Asked only
/// on a whole-tree run: named paths are a partial view of the tree.
///
/// A WARNING, though V79 calls it a violation: `src:V1` gives every
/// violation a host, a guest and a site, and an exclude has none. Which
/// shape an unsited finding takes is the spec owner's open decision;
/// until it is made, the finding is reported without inventing those
/// fields.
fn stale_excludes(report: &mut Report, tree: &Tree, tracked: &[String]) {
    for (dir, layer) in tree.layers() {
        let file = file_in(dir);
        for (key, exclude) in layer.stale_excludes(tracked.iter().map(String::as_str)) {
            // The glob as its file spells it, not rebased to the root.
            let glob = dir
                .is_empty()
                .then_some(exclude.glob.as_str())
                .or_else(|| exclude.glob.strip_prefix(&format!("{dir}/")))
                .unwrap_or(&exclude.glob);
            report.warn(Warning {
                code: Rule::StaleExclude.as_str().to_owned(),
                file: Some(PathBuf::from(&file)),
                message: format!(
                    "{key} (glob `{glob}`) matches no tracked file, so it skips nothing: \
                     delete it from {file} (src/config:V79)"
                ),
            });
        }
    }
}

/// Which files the run can judge an `[[allow]]` about (`src/config:V9`).
struct Judged<'a> {
    /// Files at least one claiming host parsed to sites.
    scanned: &'a BTreeSet<String>,
    /// Every candidate, scanned or not.
    candidates: &'a BTreeSet<String>,
    /// No paths were named: every tracked file was a candidate.
    whole_tree: bool,
}

impl Judged<'_> {
    /// Whether an entry naming `path` can be found stale by this run.
    fn covers(&self, path: &str) -> bool {
        self.scanned.contains(path) || (self.whole_tree && !self.candidates.contains(path))
    }
}

/// Where to report an allow whose site is gone: its entry in `file`, the
/// config that declares it, in the language of the host that claims the
/// file it names.
fn gone(
    allow: &Allow,
    index: usize,
    file: &str,
    config_text: &str,
    root: &Path,
    langs: &Langs<'_>,
) -> Located {
    let path = Path::new(&allow.path);
    let head = head(&root.join(path));
    let host = langs
        .hosts
        .iter()
        .find(|host| host.claims(path, &head))
        .map_or(LangId::Shell, |host| host.id());
    let line = config_text
        .lines()
        .enumerate()
        .filter(|(_, line)| line.trim() == "[[allow]]")
        .nth(index)
        .map_or(1, |(n, _)| n + 1);
    unsited(PathBuf::from(file), line, host, allow.sink.clone())
}

/// The content hash an `[[allow]]` keys a body by (`src/config:V10`):
/// 64-bit FNV-1a over the body's bytes as the host file holds them,
/// between the delimiters, as 16 lowercase hex digits.
///
/// Raw host text rather than the unescaped guest text, so the key is
/// the same whether or not the host's `unescape` has landed. FNV rather
/// than a cryptographic hash: an allow is a note to self, not a security
/// boundary, and std has no stable portable hasher (`src:C5`).
#[must_use]
pub fn body_hash(body: &str) -> String {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0100_0000_01b3;
    let hash = body
        .bytes()
        .fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME));
    format!("{hash:016x}")
}

/// The body as the guest should read it: the text between the
/// delimiters, each hole replaced by [`HOLE`].
fn guest_text(src: &str, site: &Site) -> String {
    let body = site.delim.body;
    let mut holes: Vec<_> = site
        .holes
        .iter()
        .filter(|h| h.start >= body.start && h.end <= body.end)
        .collect();
    holes.sort();
    let mut out = String::new();
    let mut at = body.start;
    for hole in holes {
        if hole.start < at {
            continue;
        }
        out.push_str(src.get(at..hole.start).unwrap_or_default());
        out.push_str(HOLE);
        at = hole.end;
    }
    out.push_str(src.get(at..body.end).unwrap_or_default());
    out
}

/// 1-based line and column of byte `offset`; the column counts
/// characters, so a report lines up with what an editor shows.
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

/// The first line of the file, for `Host::claims` (a shebang), read
/// without reading the rest: an unclaimed file is not scanned
/// (`src/check:V13`), and a large binary should not be loaded to learn that.
fn head(path: &Path) -> String {
    let mut buf = Vec::new();
    if let Ok(file) = fs::File::open(path) {
        let _ = file.take(1024).read_to_end(&mut buf);
    }
    let first = buf.split(|b| *b == b'\n').next().unwrap_or_default();
    String::from_utf8_lossy(first).into_owned()
}

/// A named `path` as the root spells it (`src/check:B4`): relative to `root`,
/// `.` and `..` resolved lexically, `.` for the root itself. An absolute
/// path is matched against the root as given (made absolute from the
/// working directory) and as the filesystem resolves it, so `/tmp/r/a`
/// and `/private/tmp/r/a` name one file on a system where one is a link
/// to the other.
///
/// # Errors
///
/// [`CheckError::Outside`] when `path` is not under `root`.
fn under_root(root: &Path, path: &Path) -> Result<PathBuf, CheckError> {
    let outside = || CheckError::Outside {
        path: path.to_path_buf(),
        root: root.to_path_buf(),
    };
    let rel = if path.is_absolute() {
        let lexical = lexical(path).ok_or_else(outside)?;
        let given = std::env::current_dir()
            .map(|cwd| cwd.join(root))
            .ok()
            .and_then(|root| lexical_root(&root));
        let resolved = fs::canonicalize(root).ok();
        [given, resolved]
            .into_iter()
            .flatten()
            .find_map(|root| lexical.strip_prefix(root).ok().map(Path::to_path_buf))
            .ok_or_else(outside)?
    } else {
        lexical(path).ok_or_else(outside)?
    };
    if rel.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(rel)
    }
}

/// `path` with `.` dropped and `..` taken back, or `None` when a `..`
/// climbs above the path's start.
fn lexical(path: &Path) -> Option<PathBuf> {
    let mut out = PathBuf::new();
    for part in path.components() {
        match part {
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            other => out.push(other),
        }
    }
    Some(out)
}

/// An absolute root, lexically resolved.
fn lexical_root(root: &Path) -> Option<PathBuf> {
    lexical(root).filter(|root| root.is_absolute())
}

/// A candidate's name as config and reports spell it: repo-root
/// relative, `/`-separated, without `./`.
pub(crate) fn repo_name(path: &Path) -> String {
    if path.is_absolute() {
        return path.display().to_string();
    }
    let parts: Vec<String> = path
        .components()
        .filter(|c| !matches!(c, Component::CurDir))
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    parts.join("/")
}
