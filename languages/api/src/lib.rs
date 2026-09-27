//! The contract every xenolith language crate implements.
//!
//! A language plays one of two roles, or both. A HOST is a file format
//! that can enclose foreign code -- nix holding shell in an indented
//! string, pkl holding a hk step, yaml holding a `run:` block. A GUEST is
//! the enclosed language itself. Extraction and inlining are one lens read
//! in two directions (`languages/api/src/lens:V34`), which is why both
//! live on the same pair of traits rather than in an engine that knows
//! about particular languages.
//!
//! # What this crate may depend on
//!
//! `xenolith-shebang`, and nothing else (`languages/api:V32`). No grammar
//! crate: a grammar here would land in every language crate's tree whether
//! it parses that language or not, and the whole point of the feature
//! split (`nix:C8`) is that a consumer compiles the grammars they use.
//!
//! # Purity
//!
//! No trait function touches the filesystem, a process, the environment or
//! the clock (`languages/api:V36`). Text goes in, values come out, and the
//! engines in the root crate own every side effect. That is what makes
//! `.:C3` -- same input, same bytes out -- a property each crate can be
//! tested for on its own.

#![forbid(unsafe_code)]

pub mod holes;
pub mod lens;
pub mod shebang;
pub mod site;

#[cfg(test)]
mod tests;

use std::fmt;
use std::path::{Path, PathBuf};

pub use crate::shebang::{Prelude, Shebang};
pub use crate::site::{Delim, DelimKind, GuestEnv, Placement, Site};

/// Every language xenolith knows, whether or not this build compiled its
/// crate in.
///
/// CLOSED and UNGATED (`languages/api:V33`). A host that finds shell in a
/// nix string must be able to SAY "shell" even in a build without
/// `lang-shell`, so the engine can report a compiled-out guest by name
/// (`src/check:V42`) instead of shrugging. Adding a language starts with a
/// variant here.
///
/// The declaration order is the sort order, and the registry iterates in
/// it (`src/registry:V41`), so output ordering is a property of this type rather
/// than of each caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LangId {
    /// AWK, a guest only.
    Awk,
    /// CSS, a guest of HTML.
    Css,
    /// Dockerfile, a host whose `RUN` lines hold shell.
    Dockerfile,
    /// HTML, a host of inline script and style.
    Html,
    /// jq, a guest only.
    Jq,
    /// JavaScript, a guest of HTML.
    Js,
    /// just, a host whose recipe bodies hold shell.
    Just,
    /// Nix, a host of shell in strings and script attributes.
    Nix,
    /// Perl, a guest only.
    Perl,
    /// Pkl, a host of hk steps and other command strings.
    Pkl,
    /// Python, a guest only.
    Python,
    /// Ruby, a host of SQL, shell and JavaScript heredocs.
    Ruby,
    /// Rust, a host of SQL and shell in raw strings.
    Rust,
    /// Shell, both a host (`bash -c`, heredocs) and a guest.
    Shell,
    /// SQL, a guest only.
    Sql,
    /// YAML, a host of GitHub Actions `run:` blocks.
    Yaml,
}

impl LangId {
    /// Every variant, in sort order.
    pub const ALL: &'static [LangId] = &[
        LangId::Awk,
        LangId::Css,
        LangId::Dockerfile,
        LangId::Html,
        LangId::Jq,
        LangId::Js,
        LangId::Just,
        LangId::Nix,
        LangId::Perl,
        LangId::Pkl,
        LangId::Python,
        LangId::Ruby,
        LangId::Rust,
        LangId::Shell,
        LangId::Sql,
        LangId::Yaml,
    ];

    /// The name used in config, in JSON output and in feature names
    /// (`lang-<name>`).
    ///
    /// One spelling for all three, because a language named `js` in a
    /// config file and `javascript` in a report is two names a user has to
    /// learn for one thing.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            LangId::Awk => "awk",
            LangId::Css => "css",
            LangId::Dockerfile => "dockerfile",
            LangId::Html => "html",
            LangId::Jq => "jq",
            LangId::Js => "js",
            LangId::Just => "just",
            LangId::Nix => "nix",
            LangId::Perl => "perl",
            LangId::Pkl => "pkl",
            LangId::Python => "python",
            LangId::Ruby => "ruby",
            LangId::Rust => "rust",
            LangId::Shell => "shell",
            LangId::Sql => "sql",
            LangId::Yaml => "yaml",
        }
    }

    /// The inverse of [`LangId::as_str`], exact-match only.
    ///
    /// Returns `None` rather than guessing. A config naming `Nix` or
    /// `bash` is a config with a mistake in it, and the mistake is worth
    /// an error that names it -- silently accepting near-misses turns a
    /// typo into a language nobody notices was never scanned.
    #[must_use]
    pub fn from_name(name: &str) -> Option<LangId> {
        LangId::ALL.iter().copied().find(|id| id.as_str() == name)
    }
}

impl fmt::Display for LangId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// A half-open byte range into the host source, `[start, end)`.
///
/// Byte offsets rather than line and column: every span here comes from a
/// grammar node (`languages/api/src/site:V38`), parsers report bytes, and
/// converting to a human position is the reporting layer's job, done once,
/// at the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// First byte of the range.
    pub start: usize,
    /// One past the last byte of the range.
    pub end: usize,
}

impl Span {
    /// A span over `[start, end)`.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Span {
        Span { start, end }
    }

    /// The number of bytes covered, saturating at zero for an inverted
    /// span.
    #[must_use]
    pub const fn len(self) -> usize {
        self.end.saturating_sub(self.start)
    }

    /// Whether the span covers nothing.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// The text this span covers, or `None` when it does not fit `src` or
    /// does not land on character boundaries.
    ///
    /// `None` rather than a panic, deliberately: this crate's subject is
    /// byte spans over other people's files, and a slicing panic
    /// mid-scan reports nothing at all about the files it had not yet
    /// reached.
    #[must_use]
    pub fn of(self, src: &str) -> Option<&str> {
        src.get(self.start..self.end)
    }
}

/// How to run a file of some guest language: `bash x.sh`, `jq -f x.jq`.
///
/// The guest owns this and the host never hardcodes it
/// (`languages/api:V35`); the host's job is to wrap this argv in its own
/// syntax -- `builtins.readFile`, a hk step, a `run:` line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoke {
    /// The command and its arguments, already including the file path.
    pub argv: Vec<String>,
}

/// A reference, found in a host file, to an extracted file.
///
/// The inverse direction of a [`Site`]: `sites` finds code to pull out,
/// `loads` finds what an earlier extraction put in, which is what lets
/// `xnl graph` prove every load resolves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadRef {
    /// Where the load sits in the host source.
    pub span: Span,
    /// The loaded path, relative to the site's runtime base
    /// (`languages/api/src/lens:V66`).
    pub path: PathBuf,
    /// The language of the file being loaded.
    pub guest: LangId,
}

/// Where a check command wants its file argument.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileArg {
    /// Appended after the last argument: `shellcheck -s bash FILE`.
    Append,
    /// Substituted for the literal `{file}` token already present in
    /// `argv`, which is how a user writes one in config
    /// (`[lint.<guest>] checks`, `src/lint` §I). Some tools need the path
    /// in the middle: `jq -n -f FILE`, `gawk --lint -f FILE /dev/null`.
    Placeholder,
}

/// How to read a check command's output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Format {
    /// JSON on stdout, in the shape the named tool emits. `src/lint`
    /// dispatches on this name (`src/lint:V92`); it is a tool identity
    /// rather than free text, and an unknown one falls back to `raw_tail`
    /// rather than dropping the findings.
    Json(&'static str),
    /// SARIF on stdout.
    Sarif,
    /// No machine-readable output: exit code plus whatever the tool
    /// printed, kept as `raw_tail` (`src/lint:V92`).
    Raw,
}

/// One check or fixer command for a guest or a host file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LintCmd {
    /// The command and its arguments, without the file unless `file_arg`
    /// is [`FileArg::Placeholder`].
    pub argv: Vec<String>,
    /// Where the file path goes.
    pub file_arg: FileArg,
    /// How to read what comes back.
    pub format: Format,
}

/// What a language crate can fail with.
///
/// Small and closed on purpose. These traits parse and rewrite text and do
/// nothing else, so there are exactly two ways to fail: the text did not
/// parse, or the operation is one this language does not offer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The host or guest source did not parse.
    Parse {
        /// Whose grammar rejected it.
        lang: LangId,
        /// What the parser said, in its own words.
        message: String,
    },
    /// The language does not implement this operation.
    ///
    /// Distinct from a parse failure, because the caller's response
    /// differs: a parse error is about the file, this is about the build.
    Unsupported {
        /// The language asked.
        lang: LangId,
        /// The operation asked for, named as the caller would say it:
        /// `inline`, `rewrite`, `loads`.
        operation: &'static str,
    },
}

impl Error {
    /// A parse failure, with whatever the parser said.
    #[must_use]
    pub fn parse(lang: LangId, message: impl Into<String>) -> Error {
        Error::Parse {
            lang,
            message: message.into(),
        }
    }

    /// An operation this language does not offer.
    #[must_use]
    pub const fn unsupported(lang: LangId, operation: &'static str) -> Error {
        Error::Unsupported { lang, operation }
    }

    /// The language the failure is about.
    #[must_use]
    pub const fn lang(&self) -> LangId {
        match *self {
            Error::Parse { lang, .. } | Error::Unsupported { lang, .. } => lang,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Parse { lang, message } => write!(f, "{lang}: parse failed: {message}"),
            Error::Unsupported { lang, operation } => {
                write!(f, "{lang}: does not support `{operation}`")
            }
        }
    }
}

impl std::error::Error for Error {}

/// The result of any trait function here.
pub type Result<T> = std::result::Result<T, Error>;

/// A language that can ENCLOSE foreign code.
///
/// A capability a language lacks is a missing impl, never a default
/// method returning an empty vector (`languages/api:V37`): an empty
/// result reads as "nothing here", and a host that silently finds no
/// sites is indistinguishable from a clean file. The defaults this trait
/// does have are refusals, [`Error::Unsupported`] naming the operation,
/// which is
/// what lets a language crate compile before it offers
/// [`Host::placement`]. The exception is [`Host::guest_by_shebang`]: it
/// answers a yes/no question whose `false` is the TRUE answer for every
/// host that never reads a shebang to pick a guest.
///
/// Implementations are pure (`languages/api:V36`) and return `Vec`s sorted
/// by span, so the engines can merge results from a parallel scan and
/// still produce byte-identical output (`src/check:V95`, `src:V11`).
pub trait Host {
    /// Which language this is.
    fn id(&self) -> LangId;

    /// Whether this host claims the file, by extension, filename, path
    /// shape or the shebang in `head` (the first line).
    ///
    /// Every file is offered to every host; a file no host claims is not
    /// scanned (`src/check:V13`).
    fn claims(&self, path: &Path, head: &str) -> bool;

    /// The sinks in `src` that hold guest code, sorted by span.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `src` is not valid source for this language.
    fn sites(&self, src: &str) -> Result<Vec<Site>>;

    /// References in `src` to already-extracted files, sorted by span.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `src` is not valid source for this language.
    fn loads(&self, src: &str) -> Result<Vec<LoadRef>>;

    /// The extract direction: body out of the host, a load of `path` in.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `src` does not parse, or
    /// [`Error::Unsupported`] when this host cannot express the load.
    fn rewrite(&self, src: &str, site: &Site, invoke: &Invoke, path: &Path) -> Result<String>;

    /// [`Host::rewrite`] for a site whose holes [`holes::bind`] made
    /// `params`: the host with a load that passes each hole, and the body
    /// the extract holds, given as `body` (bind's) and possibly rewritten
    /// into this host's own form (`languages/api/src/lens` §I).
    ///
    /// The default carries no holes: with no params it is `rewrite` and
    /// `body` unchanged, and with any it is [`Error::Unsupported`]
    /// (`languages/api:V37`) -- a load that dropped a hole would run
    /// the extract with nothing where the host's value was.
    ///
    /// # Errors
    ///
    /// Whatever `rewrite` fails with, and [`Error::Unsupported`] for
    /// params this host cannot pass.
    fn rewrite_bound(
        &self,
        src: &str,
        site: &Site,
        invoke: &Invoke,
        path: &Path,
        body: &str,
        params: &[holes::Param],
    ) -> Result<lens::Rewrite> {
        if !params.is_empty() {
            return Err(Error::unsupported(self.id(), "rewrite_bound"));
        }
        Ok(lens::Rewrite {
            src: self.rewrite(src, site, invoke, path)?,
            body: body.to_owned(),
        })
    }

    /// The inverse: a load replaced by the body it points at.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `src` does not parse, or
    /// [`Error::Unsupported`] when this host cannot inline.
    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String>;

    /// The body as its guest reads it: `raw`, the text between the
    /// delimiters of `delim`, with this host's indentation and escapes
    /// resolved -- nix strips a `''` string's common indent and decodes
    /// `''$` (`languages/api/src/lens:V39`).
    ///
    /// Holes arrive already replaced by the engine's placeholder word, so
    /// this sees host escapes and plain text only.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] for a delimiter kind this host does not
    /// write, and [`Error::Parse`] when `raw` is not the inside of a
    /// well-formed one.
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String>;

    /// The inverse of [`Host::unescape`]: `body` as it must be written
    /// between the delimiters of `delim` for this host to read it back
    /// unchanged -- which `inline` needs, and [`lens::escape_law`] checks
    /// (`languages/api/src/lens:V39`).
    ///
    /// The default is [`Error::Unsupported`]: a body written back raw is
    /// a different program the next time the host reads it, so a host
    /// without an escape must say so rather than pretend.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] for a delimiter kind this host does not
    /// write, or when it offers no escape at all.
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        let _ = (delim, body);
        Err(Error::unsupported(self.id(), "escape"))
    }

    /// Checks to run on files of this host language itself -- nix
    /// `statix`, yaml `actionlint` -- as opposed to on the extracts.
    fn checks(&self) -> Vec<LintCmd>;

    /// Fixers for the same.
    fn fixers(&self) -> Vec<LintCmd>;

    /// Whether `site`, one of [`Host::sites`] over `src`, has its guest
    /// named by a shebang in its body rather than decided by its sink
    /// (`languages/api` §I) -- nix's shebang-led attribute value
    /// (`languages/ci/nix:T157`). The engine needs to know: a guest the
    /// build lacks is a warning when a shebang named it, and follows
    /// `[langs] missing_guest` when the sink did (`src/check:V42`).
    ///
    /// `false` unless a host overrides it, which is the answer for every
    /// host that never picks a guest by shebang.
    fn guest_by_shebang(&self, _src: &str, _site: &Site) -> bool {
        false
    }

    /// Where this host would put the extract of `site`: layer D of
    /// extract resolution, overridden by `[extract] layout` and
    /// `[[extract.rule]]` (`src/extract:V45`). The name comes from site
    /// syntax -- a nix attribute path, a hk step name, a GitHub job id
    /// (`languages/api/src/site:V43`).
    ///
    /// The default is [`Error::Unsupported`], never an invented name
    /// (`languages/api:V37`): `xnl extract --verbose` explains every
    /// field by the layer that decided it (`src/extract:V48`), and a
    /// name no host chose would be explained as the host's.
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] when this host has no placement of its own.
    fn placement(&self, site: &Site) -> Result<Placement> {
        let _ = site;
        Err(Error::unsupported(self.id(), "placement"))
    }

    /// The ways out this host proposes for holes that cannot become
    /// parameters mechanically -- nix `replaceVars`, pass as an argument,
    /// an env var (`languages/api/src/holes:V40`). The engine shows each
    /// as a `Judgment` direction; none is applied automatically.
    ///
    /// The default is [`Error::Unsupported`], never an empty list
    /// (`languages/api:V37`), which would read as "these holes have no
    /// way out".
    ///
    /// # Errors
    ///
    /// [`Error::Unsupported`] when this host offers no advice.
    fn hole_advice(&self, site: &Site) -> Result<Vec<String>> {
        let _ = site;
        Err(Error::unsupported(self.id(), "hole_advice"))
    }
}

/// A language that can BE enclosed.
///
/// A guest need not be a host: python, sql, jq and awk appear inside other
/// files and enclose nothing, so their crates carry no host grammar at
/// all.
pub trait Guest {
    /// Which language this is.
    fn id(&self) -> LangId;

    /// The extension an extract of this language gets, without the dot.
    ///
    /// Takes the env because the dialect decides it
    /// (`languages/shells/shell:V51`): a zsh body extracted as `.sh` invites
    /// shellcheck to report zsh syntax as errors, which are findings
    /// about the wrong language.
    fn extension(&self, env: &GuestEnv) -> &'static str;

    /// How to run a file of this language (`languages/api:V35`).
    fn invoke(&self, path: &Path) -> Invoke;

    /// Whether this body is trivial enough to stay inline -- for shell, a
    /// single simple command (`languages/shells/shell:V3`).
    ///
    /// The judgement is the guest's, because triviality is a fact about
    /// the guest language and the host cannot know it.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `body` does not parse as this language.
    fn trivial(&self, body: &str) -> Result<bool>;

    /// The named constructs that make `body` more than trivial, sorted
    /// and deduplicated, in the vocabulary `[threshold.<guest>] allow`
    /// matches (`src/config` §I) -- for shell, `pipeline`, `and-or`,
    /// `sequence` and the rest (`languages/shells/shell:V3`). Empty exactly
    /// when [`Guest::trivial`] is true.
    ///
    /// The engine needs the NAMES, not only the verdict, because a
    /// threshold relaxes per construct (`src/config:V55`): `allow =
    /// ["pipeline"]` keeps `a | b` inline and still flags `a && b`.
    ///
    /// The default is [`Error::Unsupported`], never an empty list
    /// (`languages/api:V37`): a guest with no construct vocabulary is
    /// judged by `[threshold.<guest>] max_lines` / `max_bytes` instead,
    /// and an empty list would read as "trivial" to a caller that did not
    /// check.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `body` does not parse as this language, and
    /// [`Error::Unsupported`] when this guest names no constructs.
    fn constructs(&self, body: &str) -> Result<Vec<&'static str>> {
        let _ = body;
        Err(Error::unsupported(self.id(), "constructs"))
    }

    /// The reason `body` is owed a judgement INSTEAD of a verdict, when this
    /// guest meets dialect syntax it cannot judge -- zsh the bash grammar
    /// cannot read (`languages/shells/shell:V138`). Asked before
    /// [`Guest::trivial`]: a `Some` is the finding, and neither `trivial`
    /// nor [`Guest::constructs`] is consulted for that body.
    ///
    /// It takes the site's [`GuestEnv`] because the dialect decides it;
    /// `trivial` has no env, and without the dialect a rejected body is
    /// only broken code (`languages:V77`). The default is `None`: a guest
    /// with one dialect judges every body it can parse.
    fn unsupported(&self, body: &str, env: &GuestEnv) -> Option<&'static str> {
        let _ = (body, env);
        None
    }

    /// What goes above the body in an extract file, given the dialect and
    /// options in force at the site.
    ///
    /// The guest's to decide, because only the guest knows it: bash
    /// carries `set -euo pipefail`, sh cannot -- `pipefail` is not POSIX
    /// -- and sql carries nothing, because a `.sql` file is read rather
    /// than run. A host asked to decide this would be guessing about a
    /// language it does not parse.
    ///
    /// Takes the env, so the prelude can reproduce the options the site
    /// established (`languages/shells/shell:V82`): a body that ran under
    /// `set -e` inside its host must still run under it once extracted,
    /// or extraction changed what the code does.
    fn prelude(&self, env: &GuestEnv) -> Prelude;

    /// Whether an extract of this language gets the executable bit.
    ///
    /// True for anything with a shebang, false for a file an interpreter
    /// is pointed AT: `jq -f x.jq` reads its program, and marking it
    /// executable would suggest `./x.jq` works.
    fn executable(&self) -> bool;

    /// Default checks for extracts of this language, given the interpreter
    /// dialect and options in force at the site.
    fn checks(&self, env: &GuestEnv) -> Vec<LintCmd>;

    /// Default fixers for the same.
    fn fixers(&self, env: &GuestEnv) -> Vec<LintCmd>;

    /// Every variable name `body` reads or assigns, sorted and
    /// deduplicated -- the names a param must not take
    /// (`languages/api/src/holes:V40`). `body` holds
    /// [`holes::marker`] words where the holes were.
    ///
    /// The default is [`Error::Unsupported`], never an empty list
    /// (`languages/api:V37`), which would read as "no name can clash".
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `body` does not parse, and
    /// [`Error::Unsupported`] when this guest takes no params.
    fn vars(&self, body: &str) -> Result<Vec<String>> {
        let _ = body;
        Err(Error::unsupported(self.id(), "vars"))
    }

    /// `body` with each param's marker replaced by a reference to that env
    /// param, in the form its context needs -- shell `"$NAME"` as a word,
    /// `${NAME}` inside `"…"` -- or [`holes::Bound::Unexpanded`] naming
    /// the first param whose marker sits where this guest would not
    /// expand a reference (`languages/api/src/holes:V40`).
    ///
    /// The default is [`Error::Unsupported`]: holes of such a guest stay
    /// a judgement.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `body` does not parse, and
    /// [`Error::Unsupported`] when this guest takes no params.
    fn params(&self, body: &str, params: &[holes::Param]) -> Result<holes::Bound> {
        let _ = (body, params);
        Err(Error::unsupported(self.id(), "params"))
    }

    /// The references in `body` to any of `names`, sorted by span, each
    /// span covering what [`Guest::params`] wrote -- `"$NAME"` whole --
    /// so the inverse direction can put the hole back
    /// (`languages/api/src/lens:V34` (e)).
    ///
    /// The default is [`Error::Unsupported`], never an empty list.
    ///
    /// # Errors
    ///
    /// [`Error::Parse`] when `body` does not parse, and
    /// [`Error::Unsupported`] when this guest takes no params.
    fn param_refs(&self, body: &str, names: &[String]) -> Result<Vec<(Span, String)>> {
        let _ = (body, names);
        Err(Error::unsupported(self.id(), "param_refs"))
    }
}
