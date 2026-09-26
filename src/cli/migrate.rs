//! `xnl migrate`: legacy per-file allowlists into `xenolith.toml`
//! (`src/cli:T97`, `src/cli` §I, `src/config:C16`).
//!
//! The hooks xenolith replaces kept one list per host language at the
//! repo root -- `.nix-embedded-shell-allowlist`,
//! `.pkl-embedded-shell-allowlist` and siblings of the same shape: one
//! repo-relative path per line, `#` lines as comments, blank lines as
//! separators. Each entry grandfathers a whole FILE.
//!
//! xenolith has no file-level allow: an `[[allow]]` names ONE site by
//! host path, sink and body hash, and a wildcard path is refused
//! (`src/config:V9`, `src/config:V10`). So a migration cannot be a
//! rewrite of the list; it has to know which sites each listed file
//! holds and which of them would be flagged. That is exactly what the
//! check engine answers (`src:V152`), so migrate RUNS it -- over the
//! listed files, under the defaults -- and writes one entry per site it
//! flags. The lists' promise ("leave this file alone") becomes the
//! narrower one xenolith can keep ("leave THESE bodies alone"), and an
//! edit to a body after the migration is flagged as it should be.
//!
//! What the lists say that becomes no entry is reported, not dropped in
//! silence: a listed file that is gone (`legacy-missing`), and one
//! holding nothing the engine flags (`legacy-no-site`). An existing
//! `xenolith.toml` is refused (exit 2): §I says migrate creates the file
//! and never merges into one, so a hand-written config is never
//! rewritten by a tool that only knows how to write `[[allow]]`.
//!
//! The TOML is written here rather than by a serializer: the `toml`
//! crate is built without its writer (`src:C5`), and the output is one
//! scalar and a list of four-string tables -- the only part that needs
//! care is string escaping, [`toml_string`].

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::io::Write;
use std::path::{Component, Path, PathBuf};

use super::{EXIT_OK, refuse};
use crate::check::{CONFIG_FILE, Options, body_hash};
use crate::config::{Allow, Config};
use crate::model::{Rule, Violation, Warning};
use crate::registry;

#[cfg(test)]
mod tests;

/// The name every legacy list ends with, after `.<lang>`.
pub const LEGACY_SUFFIX: &str = "-embedded-shell-allowlist";

/// Warning code: a listed path that is not a file in the tree -- gone,
/// outside the root, a directory or a symlink (`src:V128`).
pub const LEGACY_MISSING: &str = "legacy-missing";

/// Warning code: a listed file holding no site xenolith flags, so there
/// is nothing for an `[[allow]]` to keep.
pub const LEGACY_NO_SITE: &str = "legacy-no-site";

/// One path line of a legacy list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// The listed path, repo-root relative, `./` and empty components
    /// dropped -- the spelling the engine names the file by.
    pub path: String,
    /// 1-based line in the list, for the warnings.
    pub line: usize,
    /// The comment block directly above the entry's group, its lines
    /// joined by a space; `None` when there is none.
    pub comment: Option<String>,
}

/// What a migration would write, and what it has to say.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Migration {
    /// One entry per flagged site, in report order (`src:V11`).
    pub allows: Vec<Allow>,
    /// Listed paths that became no entry, then the engine's own.
    pub warnings: Vec<Warning>,
}

/// Whether `name` is a legacy list's file name: `.<lang>` followed by
/// [`LEGACY_SUFFIX`], `<lang>` non-empty.
#[must_use]
pub fn is_legacy(name: &str) -> bool {
    name.strip_prefix('.')
        .and_then(|rest| rest.strip_suffix(LEGACY_SUFFIX))
        .is_some_and(|lang| !lang.is_empty() && !lang.contains('/'))
}

/// The entries of a legacy list, in file order.
///
/// A comment block directly above an entry is that entry's comment, and
/// is shared by the entries that follow it until a blank line or a new
/// block -- the way the lists were written, one paragraph explaining a
/// group. A block with a blank line after it is a header and belongs to
/// no entry. Surrounding whitespace, `\r` included, is not part of a line.
#[must_use]
pub fn parse_legacy(text: &str) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut block: Vec<&str> = Vec::new();
    let mut in_block = false;
    let mut current: Option<String> = None;
    for (index, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() {
            block.clear();
            in_block = false;
            current = None;
        } else if let Some(comment) = line.strip_prefix('#') {
            if !in_block {
                block.clear();
                in_block = true;
            }
            let comment = comment.trim();
            if !comment.is_empty() {
                block.push(comment);
            }
        } else {
            if in_block {
                current = (!block.is_empty()).then(|| block.join(" "));
                block.clear();
                in_block = false;
            }
            entries.push(Entry {
                path: normalise(line),
                line: index + 1,
                comment: current.clone(),
            });
        }
    }
    entries
}

/// `./a//b.nix` as the engine names it: `a/b.nix`. An absolute path or
/// one with `..` keeps its components, for [`unusable`] to refuse.
fn normalise(path: &str) -> String {
    let path = Path::new(path);
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

/// The legacy lists at `root`, sorted by name. Only the root: that is
/// the one place the old hooks read them from.
///
/// # Errors
///
/// The message to refuse with when `root` cannot be listed.
pub fn legacy_lists(root: &Path) -> Result<Vec<String>, String> {
    let dir = fs::read_dir(root).map_err(|e| format!("xnl: {}: {e}", root.display()))?;
    let mut lists: Vec<String> = dir
        .filter_map(Result::ok)
        .filter(|item| item.file_type().is_ok_and(|t| t.is_file()))
        .filter_map(|item| item.file_name().into_string().ok())
        .filter(|name| is_legacy(name))
        .collect();
    lists.sort();
    Ok(lists)
}

/// `text` as a TOML basic string: quotes, backslashes and control
/// characters escaped, everything else as is.
#[must_use]
pub fn toml_string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            // Writing to a String cannot fail.
            c if c.is_control() => {
                let _ = write!(out, "\\u{:04X}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// `xenolith.toml` holding `allows`: the version, then one `[[allow]]`
/// table per entry, keys in the order `src/config` §I lists them.
#[must_use]
pub fn render(allows: &[Allow]) -> String {
    let mut text = String::from("version = 1\n");
    for allow in allows {
        text.push_str("\n[[allow]]\n");
        for (key, value) in [
            ("path", &allow.path),
            ("sink", &allow.sink),
            ("hash", &allow.hash),
            ("reason", &allow.reason),
        ] {
            text.push_str(key);
            text.push_str(" = ");
            text.push_str(&toml_string(value));
            text.push('\n');
        }
    }
    text
}

/// The unified diff creating `name` with `text`.
#[must_use]
pub fn creation_diff(name: &str, text: &str) -> String {
    let lines: Vec<&str> = text.lines().collect();
    let mut diff = format!(
        "--- /dev/null\n+++ b/{name}\n@@ -0,0 +1,{} @@\n",
        lines.len()
    );
    for line in lines {
        diff.push('+');
        diff.push_str(line);
        diff.push('\n');
    }
    diff
}

/// Plan the migration of `lists` (names at `root`): run the engine over
/// every usable listed file and turn each site it flags into an
/// `[[allow]]` (`src/cli` §I).
///
/// # Errors
///
/// The message to refuse with: a list that cannot be read, the engine
/// refusing the run (`crate::check::CheckError`, exit 2), or a flagged
/// site that cannot be found again to hash.
pub fn plan(root: &Path, lists: &[String], strict_hosts: bool) -> Result<Migration, String> {
    let mut migration = Migration::default();
    // Each path once, from the first list and line naming it.
    let mut listed: BTreeMap<String, (&str, Entry)> = BTreeMap::new();
    for list in lists {
        let text = fs::read_to_string(root.join(list)).map_err(|e| format!("xnl: {list}: {e}"))?;
        for entry in parse_legacy(&text) {
            if let Some(problem) = unusable(root, &entry.path) {
                migration
                    .warnings
                    .push(warning(LEGACY_MISSING, list, &entry, problem));
                continue;
            }
            listed.entry(entry.path.clone()).or_insert((list, entry));
        }
    }
    if listed.is_empty() {
        // An empty `paths` would mean every tracked file (`src:V57`).
        return Ok(migration);
    }
    let options = Options {
        paths: listed.keys().map(PathBuf::from).collect(),
        strict_hosts,
    };
    let report =
        crate::check::check(root, &Config::default(), &options).map_err(|e| format!("xnl: {e}"))?;
    migration.warnings.extend(report.warnings().iter().cloned());
    let mut flagged = BTreeSet::new();
    for violation in report.violations() {
        if violation.rule != Rule::Xenolith {
            continue;
        }
        let path = violation.file.to_string_lossy().into_owned();
        let Some((list, entry)) = listed.get(&path) else {
            continue;
        };
        let hash = site_hash(root, violation).ok_or_else(|| {
            format!(
                "xnl: {path}:{}:{}: the site the engine flagged could not be found again \
                 to hash, so nothing was migrated",
                violation.line, violation.col
            )
        })?;
        let allow = Allow {
            path: path.clone(),
            sink: violation.sink.clone(),
            hash,
            reason: reason(list, entry),
        };
        if !migration
            .allows
            .iter()
            .any(|a| a.matches(&site_key(&allow)))
        {
            migration.allows.push(allow);
        }
        flagged.insert(path);
    }
    for (path, (list, entry)) in &listed {
        if !flagged.contains(path) {
            migration.warnings.push(warning(
                LEGACY_NO_SITE,
                list,
                entry,
                "holds no site xenolith flags, so it needs no [[allow]]",
            ));
        }
    }
    Ok(migration)
}

fn site_key(allow: &Allow) -> crate::config::SiteKey<'_> {
    crate::config::SiteKey {
        path: &allow.path,
        sink: &allow.sink,
        hash: &allow.hash,
    }
}

/// Why a listed path cannot be migrated, or `None` when it is a file in
/// the tree the engine can be asked about.
fn unusable(root: &Path, path: &str) -> Option<&'static str> {
    let as_path = Path::new(path);
    if as_path.is_absolute() || as_path.components().any(|c| c == Component::ParentDir) {
        return Some("is outside the repository root");
    }
    // Every directory on the way, not only the last component: a path
    // through a symlinked directory is one discovery refuses, and one
    // entry must not abort the whole migration (`src/cli:B1`).
    let mut dir = root.to_path_buf();
    let parents = as_path.parent().into_iter().flat_map(Path::components);
    for part in parents {
        dir.push(part);
        if fs::symlink_metadata(&dir).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Some("runs through a symlinked directory, which xnl never follows (src:V128)");
        }
    }
    match fs::symlink_metadata(root.join(as_path)) {
        Err(_) => Some("does not exist"),
        Ok(meta) if meta.file_type().is_symlink() => {
            Some("is a symlink, which xnl never scans (src:V128)")
        }
        Ok(meta) if !meta.is_file() => Some("is not a file"),
        Ok(_) => None,
    }
}

fn warning(code: &str, list: &str, entry: &Entry, problem: &str) -> Warning {
    Warning {
        code: code.to_owned(),
        file: Some(PathBuf::from(list)),
        message: format!(
            "line {}: `{}` {problem}; nothing migrated for it",
            entry.line, entry.path
        ),
    }
}

/// `migrated from <list>`, then `: <comment>` when the entry had one
/// (`src/cli` §I): the reason names where the allow came from, and keeps
/// what the list said about it.
fn reason(list: &str, entry: &Entry) -> String {
    match &entry.comment {
        Some(comment) => format!("migrated from {list}: {comment}"),
        None => format!("migrated from {list}"),
    }
}

/// The allow hash of the site `violation` reports (`src/config:V10`).
///
/// A violation carries the site's position and sink but not its hash, so
/// the flagged file is asked for its sites again, by the host that
/// reported it, and the one at that position and sink is hashed with the
/// engine's own [`body_hash`], over the same raw body the engine hashed.
fn site_hash(root: &Path, violation: &Violation) -> Option<String> {
    let src = fs::read_to_string(root.join(&violation.file)).ok()?;
    let host = registry::hosts()
        .iter()
        .find(|host| host.id() == violation.host)?;
    let sites = host.sites(&src).ok()?;
    let site = sites.iter().find(|site| {
        site.sink == violation.sink
            && position(&src, site.delim.open.start) == (violation.line, violation.col)
    })?;
    Some(body_hash(site.delim.body.of(&src).unwrap_or_default()))
}

/// 1-based line and character column of byte `offset`, as the engine
/// reports a site.
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

/// `<file>: warning: <code>: <message>`, as `xnl check` prints one.
fn human_warning(warning: &Warning) -> String {
    match &warning.file {
        Some(file) => format!(
            "{}: warning: {}: {}",
            file.display(),
            warning.code,
            warning.message
        ),
        None => format!("warning: {}: {}", warning.code, warning.message),
    }
}

/// Run `xnl migrate` from `root`, writing to `out` and `err`
/// (`src/cli` §I).
///
/// No legacy list: nothing to do, exit 0. An existing `xenolith.toml`:
/// refused, exit 2. Otherwise the warnings on stderr, then either the
/// diff creating the file on stdout (exit 1: there is a change to make,
/// as `xnl extract` without `--write`) or, with `--write`, the file
/// itself (exit 0). The legacy lists are left in place: deleting them is
/// the user's step, once the diff has been read (`.:T31`).
pub fn run(
    root: &Path,
    write: bool,
    verbose: bool,
    strict_hosts: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let lists = match legacy_lists(root) {
        Ok(lists) => lists,
        Err(message) => return refuse(err, &message),
    };
    if lists.is_empty() {
        if verbose {
            let _ = writeln!(
                err,
                "xnl: no legacy allowlist (.<lang>{LEGACY_SUFFIX}) at the root; nothing to \
                 migrate"
            );
        }
        return EXIT_OK;
    }
    let target = root.join(CONFIG_FILE);
    if fs::symlink_metadata(&target).is_ok() {
        return refuse(
            err,
            &format!(
                "xnl: {CONFIG_FILE} already exists; `xnl migrate` creates it and does not \
                 merge into one (src/cli §I). Move it aside, migrate, then carry its other \
                 keys over by hand."
            ),
        );
    }
    let migration = match plan(root, &lists, strict_hosts) {
        Ok(migration) => migration,
        Err(message) => return refuse(err, &message),
    };
    // As everywhere in the CLI: a failed write to a stream has nowhere
    // to be reported, and the exit code still says what happened.
    for warning in &migration.warnings {
        let _ = writeln!(err, "{}", human_warning(warning));
    }
    let text = render(&migration.allows);
    if verbose {
        let _ = writeln!(
            err,
            "{} [[allow]] from {}",
            migration.allows.len(),
            lists.join(", ")
        );
    }
    if !write {
        let _ = out.write_all(creation_diff(CONFIG_FILE, &text).as_bytes());
        return 1;
    }
    // `create_new`: a config appearing since the check above is not
    // overwritten either.
    let written = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&target)
        .and_then(|mut file| file.write_all(text.as_bytes()));
    match written {
        Ok(()) => EXIT_OK,
        Err(e) => refuse(err, &format!("xnl: {}: {e}", target.display())),
    }
}
