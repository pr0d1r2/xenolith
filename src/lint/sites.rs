//! Linting a site where it stands, before anything is extracted
//! (`src/lint:V93`, `src/lint` §I sites).
//!
//! A site's body is what its guest would read once extracted: holes as
//! plain marker words, the host's escapes and indent resolved
//! (`Host::unescape`), the guest's prelude on top (`wrap`). That text
//! goes to a temp file, the guest's checks run on it, and each finding
//! is carried back through the prelude, the unescape and the holes to a
//! `file:line:col` in the host -- the place a reader has to fix.
//!
//! The unescape is the host's, and it says nothing about where each
//! character came from. The way back is an alignment: every character
//! of the unescaped body is looked for on the same raw line, from where
//! the last one was found. Dropped indent and escape sequences are
//! stepped over that way; a character the host made up (a case change,
//! an escape with no literal trace) keeps the previous place, which is
//! still on the right line.

use std::fs;
use std::path::PathBuf;
use std::process;
use std::sync::atomic::{AtomicUsize, Ordering};

use xenolith_lang_api::{Guest, Host, Site, Span, holes, shebang};

use super::findings::Finding;
use super::plan::{self, Cmd, Configured};
use super::report::{Kind, LintReport};
use super::{Run, Target, extend, limit, untrusted};
use crate::config::Config;
use crate::model::Warning;

#[cfg(test)]
mod tests;

/// The warning for a site whose body could not be put in front of its
/// checks (`src/lint` §I sites).
pub const SITE_UNLINTED: &str = "site-unlinted";

/// 1-based line and column of byte `offset` in `src`; the column counts
/// characters, as `xnl check` reports them.
#[must_use]
pub fn position(src: &str, offset: usize) -> (usize, usize) {
    let before = src.get(..offset).unwrap_or(src);
    let line = before.matches('\n').count() + 1;
    let col = before
        .rsplit('\n')
        .next()
        .map_or(0, |last| last.chars().count())
        + 1;
    (line, col)
}

/// Where the text [`guest_text`] built came from in the host.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Pieces {
    /// `(start in text, start in host, marker)`, in order: a marker
    /// stands for a hole and maps, whole, to the hole's start.
    pieces: Vec<(usize, usize, bool)>,
    /// The host offset of the body's end.
    end: usize,
}

impl Pieces {
    /// The host byte offset of byte `at` of the text.
    #[must_use]
    pub fn host(&self, at: usize) -> usize {
        let Some(&(start, host, marker)) = self.pieces.iter().rev().find(|p| p.0 <= at) else {
            return self.end;
        };
        if marker { host } else { host + (at - start) }
    }
}

/// The site's raw body with each hole replaced by a marker word
/// (`holes::marker`), and the way back to host offsets.
#[must_use]
pub fn guest_text(src: &str, site: &Site) -> (String, Pieces) {
    let body = site.delim.body;
    let mut spans: Vec<Span> = site
        .holes
        .iter()
        .copied()
        .filter(|h| h.start >= body.start && h.end <= body.end && h.start < h.end)
        .collect();
    spans.sort_unstable();
    let mut text = String::new();
    let mut pieces = Pieces {
        pieces: Vec::new(),
        end: body.end,
    };
    let mut at = body.start;
    let mut index = 0;
    for hole in spans {
        if hole.start < at {
            continue;
        }
        pieces.pieces.push((text.len(), at, false));
        text.push_str(src.get(at..hole.start).unwrap_or_default());
        pieces.pieces.push((text.len(), hole.start, true));
        text.push_str(&holes::marker(index));
        index += 1;
        at = hole.end;
    }
    pieces.pieces.push((text.len(), at, false));
    text.push_str(src.get(at..body.end).unwrap_or_default());
    (text, pieces)
}

/// For each character of `unescaped`, the byte of `raw` it came from,
/// plus one entry for the end: see the module docs.
///
/// Raw lines the unescape dropped at the start (nix drops a `''`
/// string's empty first line) are skipped first, as many as the line
/// counts differ by and only while they are blank.
#[must_use]
pub fn align(raw: &str, unescaped: &str) -> Vec<usize> {
    let mut cursor = 0;
    let mut dropped = raw
        .matches('\n')
        .count()
        .saturating_sub(unescaped.matches('\n').count());
    while dropped > 0 {
        let rest = raw.get(cursor..).unwrap_or_default();
        let Some(end) = rest.find('\n') else {
            break;
        };
        if !rest.get(..end).unwrap_or_default().trim().is_empty() {
            break;
        }
        cursor += end + 1;
        dropped -= 1;
    }
    let mut offsets = Vec::with_capacity(unescaped.len() + 1);
    for c in unescaped.chars() {
        let rest = raw.get(cursor..).unwrap_or_default();
        let line = if c == '\n' {
            rest
        } else {
            rest.split('\n').next().unwrap_or_default()
        };
        match line.find(c) {
            Some(i) => {
                offsets.push(cursor + i);
                cursor += i + c.len_utf8();
            }
            None => offsets.push(cursor),
        }
    }
    offsets.push(if cursor >= raw.len() {
        raw.len()
    } else {
        cursor
    });
    offsets
}

/// Positions in the materialised file, back to the host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    /// Lines the prelude added above the body.
    prelude_lines: usize,
    /// The body as the guest reads it.
    unescaped: String,
    /// Host byte offset per character of `unescaped`, and its end.
    offsets: Vec<usize>,
    /// Host byte offset of the body's start.
    start: usize,
}

impl SourceMap {
    /// The host `(line, col)` of `line:col` in the materialised file. A
    /// place on the prelude, or none (line 0), is the body's start; a
    /// column past its line's end is the line's end, a line past the
    /// body the body's end.
    #[must_use]
    pub fn locate(&self, src: &str, line: usize, col: usize) -> (usize, usize) {
        if line <= self.prelude_lines {
            return position(src, self.start);
        }
        let wanted = line - self.prelude_lines;
        let mut index = 0;
        let mut lines = self.unescaped.split('\n');
        for _ in 1..wanted {
            match lines.next() {
                Some(text) => index += text.chars().count() + 1,
                None => break,
            }
        }
        let width = lines.next().map_or(0, |text| text.chars().count());
        let index = index + col.saturating_sub(1).min(width);
        let last = self.offsets.last().copied().unwrap_or(self.start);
        position(src, self.offsets.get(index).copied().unwrap_or(last))
    }
}

/// A site's body, ready for its checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Materialised {
    /// The file's content: prelude, then the unescaped body.
    pub text: String,
    /// The way back to the host.
    pub map: SourceMap,
}

/// The file a site's checks read, and its [`SourceMap`].
///
/// # Errors
///
/// Whatever `Host::unescape` refuses the body with.
pub fn materialise(
    host: &dyn Host,
    guest: &dyn Guest,
    src: &str,
    site: &Site,
) -> xenolith_lang_api::Result<Materialised> {
    let (raw, pieces) = guest_text(src, site);
    let unescaped = host.unescape(&site.delim, &raw)?;
    let prelude = shebang::wrap("", &guest.prelude(&site.env));
    let offsets = align(&raw, &unescaped)
        .into_iter()
        .map(|at| pieces.host(at))
        .collect();
    Ok(Materialised {
        text: shebang::wrap(&unescaped, &guest.prelude(&site.env)),
        map: SourceMap {
            prelude_lines: prelude.matches('\n').count(),
            unescaped,
            offsets,
            start: site.delim.body.start,
        },
    })
}

/// Temp dirs made by this process, so parallel tests never share one.
static MADE: AtomicUsize = AtomicUsize::new(0);

/// A temp dir that removes itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new() -> std::io::Result<TempDir> {
        let n = MADE.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("xnl-lint-{}-{n}", process::id()));
        fs::create_dir_all(&dir)?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

impl Run<'_> {
    /// `--sites`: every site of a host file whose guest this build has,
    /// linted in place (`src/lint:V93`). Checks only: a site is never
    /// rewritten, so no fixer runs.
    pub(super) fn sites(
        &self,
        report: &mut LintReport,
        config: &Config,
        host: &dyn Host,
        guests: &[&dyn Guest],
        name: &str,
    ) {
        let src = fs::read_to_string(self.root.join(name)).unwrap_or_default();
        let sites = match host.sites(&src) {
            Ok(sites) => sites,
            Err(e) => return unlinted(report, name, name, &e.to_string()),
        };
        for site in &sites {
            // A guest this build lacks is `xnl check`'s to report
            // (`src/check:V42`), not a lint result.
            let Some(guest) = guests.iter().copied().find(|g| g.id() == site.guest) else {
                continue;
            };
            let (line, col) = position(&src, site.delim.body.start);
            let place = Place {
                name,
                shown: format!("{name}:{line}:{col}"),
                src: &src,
            };
            match materialise(host, guest, &src, site) {
                Ok(body) => self.site(report, config, guest, site, &place, &body),
                Err(e) => unlinted(report, name, &place.shown, &e.to_string()),
            }
        }
    }

    /// One site's checks on its materialised body.
    fn site(
        &self,
        report: &mut LintReport,
        config: &Config,
        guest: &dyn Guest,
        site: &Site,
        place: &Place<'_>,
        body: &Materialised,
    ) {
        let Place { name, shown, src } = place;
        let (name, shown, src) = (*name, shown.as_str(), *src);
        let id = guest.id();
        let target = Target {
            name,
            arg: shown,
            kind: Kind::Site,
            guest: Some(id),
            dialect: site.env.dialect.clone(),
            limit: limit(config.lint.timeout),
        };
        let checks = guest.checks(&site.env);
        let planned = plan::plan(
            &checks,
            Configured::checks(
                config.lint.guests.get(&id),
                extend(config, id),
                &config.lint.all,
            ),
            self.trusted,
        );
        for cmd in &planned.untrusted {
            untrusted(report, &target, cmd, false);
        }
        if planned.run.is_empty() {
            return;
        }
        let file = TempDir::new().and_then(|dir| {
            let path = dir.0.join(format!("site.{}", guest.extension(&site.env)));
            fs::write(&path, &body.text).map(|()| (dir, path))
        });
        let (_dir, path) = match file {
            Ok(made) => made,
            Err(e) => return unlinted(report, name, shown, &format!("no temp file: {e}")),
        };
        let temp = path.display().to_string();
        for cmd in &planned.run {
            report.push(self.on_site(&target, cmd, &temp, src, body));
        }
    }

    /// One check on a site: run on the temp file, reported under the
    /// host's name with every position mapped back.
    fn on_site(
        &self,
        target: &Target<'_>,
        cmd: &Cmd,
        temp: &str,
        src: &str,
        body: &Materialised,
    ) -> super::Outcome {
        let mut outcome = self.ran_on(target, cmd, false, temp, || body.text.clone());
        outcome.raw_tail = outcome.raw_tail.map(|tail| tail.replace(temp, target.arg));
        outcome.findings = outcome
            .findings
            .into_iter()
            .map(|f| {
                let (line, col) = body.map.locate(src, f.line, f.col);
                Finding { line, col, ..f }
            })
            .collect();
        outcome
    }
}

/// Where a site is: its host file, the name `argv` shows for it, and
/// the host's text.
struct Place<'a> {
    name: &'a str,
    /// `<host>:<line>:<col>` of the body's start (`src/lint` §I sites).
    shown: String,
    src: &'a str,
}

/// A site left unlinted, and why (`src/lint` §I sites).
fn unlinted(report: &mut LintReport, file: &str, at: &str, why: &str) {
    report.warn(Warning {
        code: SITE_UNLINTED.to_owned(),
        file: Some(PathBuf::from(file)),
        message: format!("{at}: site not linted: {why}; `xnl check` reports it (src/lint:V93)"),
    });
}
