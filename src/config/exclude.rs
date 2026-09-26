//! `[[exclude]]` and the per-verb exclude lists: parse, match, and find
//! the stale ones (`src/config:V79`, `src/config:T80`).
//!
//! An exclude is a promise that a tracked file is skipped -- never read
//! -- by every verb (`[[exclude]]`) or by one (`[check] exclude`,
//! `[extract] exclude`, `[lint] exclude`, `[graph] exclude`, applied on
//! top). Like an allow, it carries its reason, and one that matches no
//! tracked file is itself a finding, `stale-exclude`: a skip nobody can
//! see the point of is a skip nobody removes.
//!
//! The engine calls [`Config::excluded`] before reading a candidate, and
//! [`Config::stale_excludes`] once with every tracked file.
//!
//! GLOBS are matched here rather than by a crate (`src:C5`); the syntax
//! is small and closed:
//!
//! * relative to the repo root (to the declaring file once nested
//!   configs land, `.:T91`), `/`-separated, anchored -- `*.png` is the
//!   top level only, `**/*.png` is everywhere;
//! * `*` any run within one segment, `?` one character, `[abc]`,
//!   `[a-z]`, `[!a]` one character of a class -- none crosses a `/`;
//! * a `**` segment spans zero or more segments;
//! * a pattern matching a DIRECTORY covers everything beneath it, so
//!   `vendor`, `vendor/` and `vendor/**` say the same thing;
//! * a leading `./` and a trailing `/` are dropped. No escapes.

use toml::{Table, Value};

use super::{Config, ConfigError, string, table, tables};

#[cfg(test)]
mod tests;

/// One exclude entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exclude {
    /// The glob, as written.
    pub glob: String,
    /// Why these files are skipped. Non-empty.
    pub reason: String,
}

/// A verb that reads tracked files and so honours a per-verb list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verb {
    /// `xnl check`.
    Check,
    /// `xnl extract`.
    Extract,
    /// `xnl graph`.
    Graph,
    /// `xnl lint`.
    Lint,
}

impl Verb {
    /// Every verb, in name order.
    pub const ALL: &'static [Verb] = &[Verb::Check, Verb::Extract, Verb::Graph, Verb::Lint];

    /// The verb's name, which is also its table in `xenolith.toml`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Verb::Check => "check",
            Verb::Extract => "extract",
            Verb::Graph => "graph",
            Verb::Lint => "lint",
        }
    }
}

/// Every exclude list in one file, each in file order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Excludes {
    /// `[[exclude]]`: every verb.
    pub all: Vec<Exclude>,
    /// `[check] exclude`.
    pub check: Vec<Exclude>,
    /// `[extract] exclude`.
    pub extract: Vec<Exclude>,
    /// `[graph] exclude`.
    pub graph: Vec<Exclude>,
    /// `[lint] exclude`.
    pub lint: Vec<Exclude>,
}

impl Excludes {
    /// The list one verb adds on top of [`Excludes::all`].
    #[must_use]
    pub fn verb(&self, verb: Verb) -> &[Exclude] {
        match verb {
            Verb::Check => &self.check,
            Verb::Extract => &self.extract,
            Verb::Graph => &self.graph,
            Verb::Lint => &self.lint,
        }
    }

    pub(super) fn verb_mut(&mut self, verb: Verb) -> &mut Vec<Exclude> {
        match verb {
            Verb::Check => &mut self.check,
            Verb::Extract => &mut self.extract,
            Verb::Graph => &mut self.graph,
            Verb::Lint => &mut self.lint,
        }
    }
}

impl Exclude {
    /// Whether this entry covers the repo-relative `path`: the glob
    /// matches the path itself or one of its parent directories.
    #[must_use]
    pub fn matches(&self, path: &str) -> bool {
        let pattern: Vec<&str> = segments(&self.glob);
        let path: Vec<&str> = segments(path);
        (1..=path.len()).any(|n| path.get(..n).is_some_and(|head| match_path(&pattern, head)))
    }
}

impl Config {
    /// The entry that makes `verb` skip `path` -- `[[exclude]]` first,
    /// then the verb's own list, first match in file order -- or `None`
    /// when the file is read (`src/config:V79`).
    #[must_use]
    pub fn excluded(&self, verb: Verb, path: &str) -> Option<&Exclude> {
        self.exclude
            .all
            .iter()
            .chain(self.exclude.verb(verb))
            .find(|e| e.matches(path))
    }

    /// Every entry whose glob matches no file in `tracked`, named by its
    /// key (`exclude[1]`, `lint.exclude[0]`) -- `[[exclude]]` first, then
    /// the verbs by name, each in file order (`src/config:V79`,
    /// `src:V11`). `tracked` may come in any order.
    pub fn stale_excludes<'p, I>(&self, tracked: I) -> Vec<(String, &Exclude)>
    where
        I: IntoIterator<Item = &'p str>,
    {
        let tracked: Vec<&str> = tracked.into_iter().collect();
        let lists = std::iter::once(("exclude".to_owned(), self.exclude.all.as_slice())).chain(
            Verb::ALL.iter().map(|verb| {
                (
                    format!("{}.exclude", verb.as_str()),
                    self.exclude.verb(*verb),
                )
            }),
        );
        lists
            .flat_map(|(at, list)| {
                list.iter()
                    .enumerate()
                    .map(move |(i, e)| (format!("{at}[{i}]"), e))
            })
            .filter(|(_, e)| !tracked.iter().any(|path| e.matches(path)))
            .collect()
    }
}

// ---------------------------------------------------------------------
// parsing
// ---------------------------------------------------------------------

/// An exclude list at `key` (`exclude`, `lint.exclude`): an array of
/// `{ glob, reason }` tables.
pub(super) fn parse_list(key: &str, value: &Value) -> Result<Vec<Exclude>, ConfigError> {
    tables(key, value)?
        .into_iter()
        .enumerate()
        .map(|(i, t)| parse_entry(&format!("{key}[{i}]"), t))
        .collect()
}

/// `[check]` or `[graph]`: a table holding `exclude` and nothing else.
pub(super) fn parse_verb_table(
    verb: Verb,
    value: &Value,
    into: &mut Excludes,
) -> Result<(), ConfigError> {
    let at = verb.as_str();
    for (leaf, value) in table(at, value)? {
        let key = format!("{at}.{leaf}");
        if leaf != "exclude" {
            return Err(ConfigError::unknown(&key));
        }
        *into.verb_mut(verb) = parse_list(&key, value)?;
    }
    Ok(())
}

fn parse_entry(at: &str, t: &Table) -> Result<Exclude, ConfigError> {
    for leaf in t.keys() {
        if leaf != "glob" && leaf != "reason" {
            return Err(ConfigError::unknown(&format!("{at}.{leaf}")));
        }
    }
    let required = |leaf: &str| -> Result<String, ConfigError> {
        let key = format!("{at}.{leaf}");
        let value = t
            .get(leaf)
            .ok_or_else(|| ConfigError::new(&key, "required"))?;
        let s = string(&key, value)?;
        if s.trim().is_empty() {
            return Err(ConfigError::new(key, "must not be empty"));
        }
        Ok(s)
    };
    let glob = required("glob")?;
    check_glob(&glob).map_err(|message| ConfigError::new(format!("{at}.glob"), message))?;
    let reason = required("reason").map_err(|e| ConfigError {
        message: format!(
            "{}: every exclude carries its reason (src/config:V79)",
            e.message
        ),
        ..e
    })?;
    Ok(Exclude { glob, reason })
}

/// Refuse a glob the matcher would read as something the user did not
/// write: an unclosed or empty class.
fn check_glob(glob: &str) -> Result<(), String> {
    let mut chars = glob.chars();
    while let Some(c) = chars.next() {
        if c == '[' {
            let rest: String = chars.clone().collect();
            match class_end(&rest) {
                Some(len) => {
                    chars.nth(len);
                }
                None => return Err(format!("unclosed or empty `[` class in `{glob}`")),
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------
// matching
// ---------------------------------------------------------------------

fn segments(s: &str) -> Vec<&str> {
    let s = s.strip_prefix("./").unwrap_or(s);
    s.split('/').filter(|seg| !seg.is_empty()).collect()
}

/// Whole-path match, `**` spanning segments.
fn match_path(pattern: &[&str], path: &[&str]) -> bool {
    match pattern.split_first() {
        None => path.is_empty(),
        Some((&"**", rest)) => {
            (0..=path.len()).any(|skip| path.get(skip..).is_some_and(|tail| match_path(rest, tail)))
        }
        Some((seg, rest)) => path.split_first().is_some_and(|(head, tail)| {
            match_segment(
                &seg.chars().collect::<Vec<_>>(),
                &head.chars().collect::<Vec<_>>(),
            ) && match_path(rest, tail)
        }),
    }
}

/// One segment: `*`, `?`, `[...]`, literals.
fn match_segment(pattern: &[char], text: &[char]) -> bool {
    match pattern.split_first() {
        None => text.is_empty(),
        Some(('*', rest)) => {
            (0..=text.len()).any(|skip| text.get(skip..).is_some_and(|t| match_segment(rest, t)))
        }
        Some(('?', rest)) => text
            .split_first()
            .is_some_and(|(_, t)| match_segment(rest, t)),
        Some(('[', rest)) => {
            let body: String = rest.iter().collect();
            let Some(len) = class_end(&body) else {
                // Refused at parse; a hand-built entry matches nothing.
                return false;
            };
            let (class, after) = (rest.get(..len), rest.get(len + 1..));
            match (class, after, text.split_first()) {
                (Some(class), Some(after), Some((c, t))) => {
                    in_class(class, *c) && match_segment(after, t)
                }
                _ => false,
            }
        }
        Some((lit, rest)) => text
            .split_first()
            .is_some_and(|(c, t)| c == lit && match_segment(rest, t)),
    }
}

/// Char count of a class body up to (not including) its `]`, or `None`
/// when unclosed or empty. `s` starts just after the `[`.
fn class_end(s: &str) -> Option<usize> {
    let chars: Vec<char> = s.chars().collect();
    let start = usize::from(matches!(chars.first(), Some('!' | '^')));
    let len = chars.iter().skip(start).position(|c| *c == ']')?;
    (len > 0).then_some(start + len)
}

fn in_class(class: &[char], c: char) -> bool {
    let (negate, items) = match class.split_first() {
        Some(('!' | '^', rest)) => (true, rest),
        _ => (false, class),
    };
    let mut hit = false;
    let mut i = 0;
    while let Some(&lo) = items.get(i) {
        if let (Some('-'), Some(&hi)) = (items.get(i + 1), items.get(i + 2)) {
            hit |= (lo..=hi).contains(&c);
            i += 3;
        } else {
            hit |= lo == c;
            i += 1;
        }
    }
    c != '/' && hit != negate
}
