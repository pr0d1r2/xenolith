//! HOLES as params: host interpolations inside a body -- nix `${…}`, pkl
//! `\(…)` -- become named env params of the extract, or the site stays a
//! judgement call (`languages/api/src/holes:V40`).
//!
//! An extract cannot carry host syntax: `${pkgs.foo}` copied into a `.sh`
//! file is a shell parameter expansion of a variable nobody set. So each
//! distinct hole gets a NAME, the extract refers to `$NAME` in whatever
//! form its context needs, and the load passes `NAME=<hole>` -- the hole
//! stays host syntax, in the host.
//!
//! [`bind`] runs the steps in order. Its parts are public for a host that
//! writes the body its own way -- nix writes `__NAME__`
//! (`languages/ci/nix:V174`) -- and so needs the holes and names without the
//! guest's references.

use std::fmt;

use crate::{Error, Guest, Host, Result, Site, Span};

/// What every marker starts with; the index and a `_` follow
/// ([`marker`]).
const MARKER: &str = "XNL_HOLE_";

/// Names nothing may shadow (`languages/api/src/holes:V40`): the shell's
/// own, the login's, and CI's.
const RESERVED: &[&str] = &[
    "PATH", "HOME", "IFS", "PWD", "OLDPWD", "SHELL", "USER", "LOGNAME", "TERM", "TMPDIR", "LANG",
    "SHLVL", "PS1", "PS2", "PS3", "PS4", "HOSTNAME", "UID", "EUID", "CI",
];

/// Prefixes whose every name is reserved: `LC_*`, `BASH*`, `ZSH*`,
/// `GITHUB_*`, `RUNNER_*` -- and the markers themselves.
const RESERVED_PREFIXES: &[&str] = &["LC_", "BASH", "ZSH", "GITHUB_", "RUNNER_", "XNL_HOLE"];

/// One distinct hole: the interpolation and the path tail attached to it,
/// with every place it occurs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hole {
    /// The host interpolation as written: `${pkgs.foo}`.
    pub expr: String,
    /// The `/`-led path tail after it, possibly empty: `/bin/foo`.
    pub tail: String,
    /// Every occurrence in the host source, hole and tail together, in
    /// source order.
    pub spans: Vec<Span>,
}

impl Hole {
    /// Hole and tail, the value the load assigns: `${pkgs.foo}/bin/foo`.
    #[must_use]
    pub fn text(&self) -> String {
        format!("{}{}", self.expr, self.tail)
    }
}

/// One param of an extract.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    /// The env name: `FOO_BIN`.
    pub name: String,
    /// Its value in host syntax, hole and tail: `${pkgs.foo}/bin/foo`.
    pub hole: String,
    /// The plain word standing in for every occurrence in the body handed
    /// to the guest: `XNL_HOLE_0_`.
    pub marker: String,
}

/// The `[threshold.load]` knobs V40 reads (`src/config` §I). Defaults
/// live in the config, their one source, not here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits<'a> {
    /// `max_params`: more distinct holes than this is a judgement.
    pub max_params: u64,
    /// `param_prefix`, prepended to every derived name.
    pub prefix: &'a str,
}

/// Why a site's holes cannot become params mechanically: the `why` of its
/// `Judgment` (`languages/api/src/holes:V40`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// More distinct holes than `[threshold.load] max_params`.
    TooMany {
        /// Distinct holes found.
        count: usize,
        /// The threshold.
        max: u64,
    },
    /// A hole sits where the guest does not expand a reference: inside
    /// `'…'`, a heredoc, a comment.
    Unexpanded {
        /// The param that would have stood there.
        name: String,
    },
    /// The guest offers no param support (`languages/api:V37`).
    Unsupported {
        /// The operation it refused.
        operation: &'static str,
    },
    /// The body already spells a marker, so markers could not say where
    /// the holes were.
    Marker,
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        const V40: &str = "languages/api/src/holes:V40";
        match self {
            Refusal::TooMany { count, max } => write!(
                f,
                "{count} holes, more than [threshold.load] max_params = {max} ({V40})"
            ),
            Refusal::Unexpanded { name } => write!(
                f,
                "a hole sits where the guest would not expand `{name}` -- quoted, a heredoc \
                 or a comment -- so it cannot become a param ({V40})"
            ),
            Refusal::Unsupported { operation } => write!(
                f,
                "the guest cannot take holes as params: it has no `{operation}` ({V40})"
            ),
            Refusal::Marker => write!(
                f,
                "the body already holds the marker text `{MARKER}`, so holes cannot be \
                 placed ({V40})"
            ),
        }
    }
}

/// What a guest's `params` made of a marked body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Bound {
    /// Every marker replaced by a reference to its param.
    Body(String),
    /// This param's marker sits where the guest does not expand a
    /// reference.
    Unexpanded(String),
}

/// What [`bind`] decided for a site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// Holes become params: the guest body the extract holds, and the
    /// params its load passes, in first-occurrence order.
    Mechanical {
        /// The body as the guest reads it, holes now param references.
        body: String,
        /// One per distinct hole.
        params: Vec<Param>,
    },
    /// Holes stay a judgement call, for this reason.
    Judgment(Refusal),
}

/// The marker for the hole at `index`: `XNL_HOLE_<index>_`.
///
/// A plain word, so every host's `unescape` passes it through and every
/// guest grammar reads it as one; the trailing `_` keeps `XNL_HOLE_1_`
/// from being a prefix of `XNL_HOLE_11_`.
#[must_use]
pub fn marker(index: usize) -> String {
    format!("{MARKER}{index}_")
}

/// The distinct holes of `site`, each with its attached path tail, in
/// first-occurrence order (`languages/api/src/holes:V40`).
///
/// A tail is a `/`-led run of `[A-Za-z0-9._+/-]`: `${pkgs.foo}/bin/foo`
/// is one value. It stops at anything else -- `;`, `:`, a quote --
/// because swallowing guest syntax into a param would move it out of the
/// program. Holes outside the body, or overlapping one already taken,
/// are skipped, as the engine's placeholder pass skips them.
#[must_use]
pub fn collect(src: &str, site: &Site) -> Vec<Hole> {
    let body = site.delim.body;
    let mut spans: Vec<Span> = site
        .holes
        .iter()
        .copied()
        .filter(|h| h.start >= body.start && h.end <= body.end && h.start < h.end)
        .collect();
    spans.sort_unstable();

    let mut holes: Vec<Hole> = Vec::new();
    let mut at = body.start;
    for (i, hole) in spans.iter().enumerate() {
        if hole.start < at {
            continue;
        }
        let Some(expr) = hole.of(src) else {
            continue;
        };
        let limit = spans
            .get(i + 1)
            .map_or(body.end, |next| next.start.min(body.end));
        let end = tail_end(src, hole.end, limit);
        let tail = src.get(hole.end..end).unwrap_or_default();
        let span = Span::new(hole.start, end);
        at = end;
        match holes.iter_mut().find(|h| h.expr == expr && h.tail == tail) {
            Some(known) => known.spans.push(span),
            None => holes.push(Hole {
                expr: expr.to_owned(),
                tail: tail.to_owned(),
                spans: vec![span],
            }),
        }
    }
    holes
}

/// Where the path tail starting at `from` ends, never past `limit`.
fn tail_end(src: &str, from: usize, limit: usize) -> usize {
    let bytes = src.as_bytes();
    if bytes.get(from) != Some(&b'/') {
        return from;
    }
    let path = |b: u8| b.is_ascii_alphanumeric() || b"._+/-".contains(&b);
    let mut end = from;
    while end < limit && bytes.get(end).copied().is_some_and(path) {
        end += 1;
    }
    end
}

/// The name a hole asks for before prefix and clashes
/// (`languages/api/src/holes:V40`): the `UPPER_SNAKE` last segment, plus
/// the tail's first directory as a kind suffix -- `${pkgs.foo}/bin/foo` →
/// `FOO_BIN`, `${cfg.port}` → `PORT`.
///
/// The last segment is the tail's last one when there is a tail, else
/// the expression's last word. No word at all gives `PARAM`; a leading
/// digit gets `PARAM_` in front, since an env name cannot start with one.
#[must_use]
pub fn base_name(hole: &Hole) -> String {
    let dirs: Vec<&str> = hole.tail.split('/').filter(|s| !s.is_empty()).collect();
    let name = match dirs.as_slice() {
        [] => snake(last_word(&hole.expr)),
        [only] => snake(only),
        [kind, .., last] => format!("{}_{}", snake(last), snake(kind)),
    };
    let name = name.trim_matches('_').to_owned();
    if name.is_empty() {
        "PARAM".to_owned()
    } else if name.starts_with(|c: char| c.is_ascii_digit()) {
        format!("PARAM_{name}")
    } else {
        name
    }
}

/// The last run of identifier characters in `expr`: `foo` in
/// `${pkgs.foo}`, `hello` in `${lib.getExe pkgs.hello}`.
fn last_word(expr: &str) -> &str {
    expr.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '-'))
        .rfind(|s| !s.is_empty())
        .unwrap_or_default()
}

/// `UPPER_SNAKE`: camelCase split, every other character an `_`, runs of
/// `_` collapsed.
fn snake(word: &str) -> String {
    let mut out = String::new();
    let mut prev_lower = false;
    for c in word.chars() {
        if c.is_ascii_alphanumeric() {
            if c.is_ascii_uppercase() && prev_lower {
                out.push('_');
            }
            out.push(c.to_ascii_uppercase());
            prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        } else {
            if !out.ends_with('_') {
                out.push('_');
            }
            prev_lower = false;
        }
    }
    out.trim_matches('_').to_owned()
}

/// Whether `name` is one no param may take (`languages/api/src/holes:V40`),
/// the markers included.
#[must_use]
pub fn reserved(name: &str) -> bool {
    RESERVED.contains(&name) || RESERVED_PREFIXES.iter().any(|p| name.starts_with(p))
}

/// The final names of `holes`, in order (`languages/api/src/holes:V40`):
/// `prefix` first; a name that is [`reserved`] or in `taken` (the vars
/// the body already reads or assigns) gets `_PARAM`; one an earlier hole
/// has, or still clashing, counts up `_2`, `_3`, ….
#[must_use]
pub fn names(holes: &[Hole], prefix: &str, taken: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::with_capacity(holes.len());
    for hole in holes {
        let base = format!("{prefix}{}", base_name(hole));
        let name = free_name(base, taken, &out);
        out.push(name);
    }
    out
}

/// The first name from `base` that clashes with nothing.
///
/// A `_PARAM` name is exempt from the reserved PREFIXES: `BASH_BIN` is
/// reserved as `BASH*`, and so would be every name built on it, so the
/// count would never end. `_PARAM` is this tool's own suffix, and no
/// shell or CI variable wears it. A count that lands on a reserved name
/// (`LC` → `LC_2`, a `LC_*`) switches to `_PARAM` the same way.
fn free_name(base: String, taken: &[String], used: &[String]) -> String {
    let busy = |name: &str| taken.iter().any(|t| t == name) || used.iter().any(|u| u == name);
    let mut stem = base;
    let mut suffixed = false;
    let mut name = stem.clone();
    let mut n = 2;
    loop {
        let refused = !suffixed && reserved(&name);
        if !refused && !busy(&name) {
            return name;
        }
        if refused || (!suffixed && taken.contains(&stem)) {
            stem = format!("{stem}_PARAM");
            suffixed = true;
            name.clone_from(&stem);
            n = 2;
            continue;
        }
        name = format!("{stem}_{n}");
        n += 1;
    }
}

/// The raw body with every hole occurrence replaced by its marker.
fn marked(src: &str, body: Span, holes: &[Hole]) -> String {
    let mut spans: Vec<(Span, usize)> = holes
        .iter()
        .enumerate()
        .flat_map(|(i, h)| h.spans.iter().map(move |s| (*s, i)))
        .collect();
    spans.sort_unstable();
    let mut out = String::new();
    let mut at = body.start;
    for (span, i) in spans {
        out.push_str(src.get(at..span.start).unwrap_or_default());
        out.push_str(&marker(i));
        at = span.end;
    }
    out.push_str(src.get(at..body.end).unwrap_or_default());
    out
}

/// Holes → params for one site (`languages/api/src/holes:V40`): collect
/// the distinct holes, refuse more than `max_params`, mark each in the raw
/// body, `unescape` it, ask the guest which names the body already uses,
/// name the params, and let the guest place a reference at each marker.
///
/// A body with no holes is its `unescape`, and the guest is not asked: a
/// guest without param support still extracts a body that needs none.
/// A guest refusing any step (`languages/api:V37`) is a judgement, not
/// an error.
///
/// Not asked here: whether the host can write the load one-liner and
/// whether it stays trivial (`languages/shells/shell:V3`) -- that is the host's
/// `rewrite`, given the params.
///
/// # Errors
///
/// Whatever `unescape` fails with, a guest parse error, or
/// [`Error::Parse`] when the site's body is not inside `src`.
pub fn bind<H: Host + ?Sized, G: Guest + ?Sized>(
    host: &H,
    guest: &G,
    src: &str,
    site: &Site,
    limits: &Limits<'_>,
) -> Result<Outcome> {
    let body = site.delim.body;
    let raw = body
        .of(src)
        .ok_or_else(|| Error::parse(host.id(), "the site's body is not inside the source"))?;
    let holes = collect(src, site);
    if holes.is_empty() {
        return Ok(Outcome::Mechanical {
            body: host.unescape(&site.delim, raw)?,
            params: Vec::new(),
        });
    }
    let count = holes.len();
    if u64::try_from(count).map_or(true, |c| c > limits.max_params) {
        return Ok(Outcome::Judgment(Refusal::TooMany {
            count,
            max: limits.max_params,
        }));
    }
    let occurrences: usize = holes.iter().map(|h| h.spans.len()).sum();
    let marked = marked(src, body, &holes);
    if marked.matches(MARKER).count() != occurrences {
        return Ok(Outcome::Judgment(Refusal::Marker));
    }
    let text = host.unescape(&site.delim, &marked)?;
    let taken = match guest.vars(&text) {
        Ok(taken) => taken,
        Err(e) => return refused(e),
    };
    let params: Vec<Param> = names(&holes, limits.prefix, &taken)
        .into_iter()
        .zip(&holes)
        .enumerate()
        .map(|(i, (name, hole))| Param {
            name,
            hole: hole.text(),
            marker: marker(i),
        })
        .collect();
    match guest.params(&text, &params) {
        Ok(Bound::Body(body)) => Ok(Outcome::Mechanical { body, params }),
        Ok(Bound::Unexpanded(name)) => Ok(Outcome::Judgment(Refusal::Unexpanded { name })),
        Err(e) => refused(e),
    }
}

/// A guest's refusal as a judgement; any other failure stays an error.
fn refused(e: Error) -> Result<Outcome> {
    match e {
        Error::Unsupported { operation, .. } => {
            Ok(Outcome::Judgment(Refusal::Unsupported { operation }))
        }
        other @ Error::Parse { .. } => Err(other),
    }
}

#[cfg(test)]
mod tests;
