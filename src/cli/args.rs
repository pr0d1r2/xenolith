//! What the user typed, as a value: the `xnl` argument parser.
//!
//! Separate from the dispatch (`src/cli/mod.rs`) because it answers a
//! separate question. The parser decides what was ASKED; the dispatch
//! decides what to do about it -- today, for most verbs, refuse
//! (`src/cli:V24`). Keeping them apart is what lets a flag typo be
//! reported now, on a verb whose engine has not landed, instead of the
//! day it does.
//!
//! The grammar is `src/cli` §I's, and nothing looser:
//!
//! * the verb comes first, flags follow it; `--version` / `-V` stand in
//!   for a verb. `xnl --verbose check` is refused, so there is one place
//!   to look for flags.
//! * every flag is a long flag, and each verb accepts only its own (plus
//!   `--verbose` and `--strict-hosts`, which every verb takes). A flag
//!   of another verb is refused by name: `--write` on `check` is a user
//!   expecting a write that would never happen.
//! * `--format` takes its value as the next argument or after `=`. The
//!   last one given wins.
//! * `--` ends the flags; after it everything is a path.
//!
//! Arguments arrive as `OsStr`: hk hands over whatever filenames the tree
//! holds, and `std::env::args` panics on one that is not UTF-8. A path
//! stays bytes; only flags and their values must be text.

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests;

/// How a verb renders what it found (`src/cli` §I).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// `file:line:col` lines for a terminal. The default.
    Human,
    /// The versioned envelope (`src/cli:V24`).
    Json,
    /// SARIF 2.1.0 (`src/cli:V102`); the writer is `src/cli:T103`'s.
    Sarif,
}

impl OutputFormat {
    /// Every format, in the order usage lists them.
    pub const ALL: &'static [OutputFormat] =
        &[OutputFormat::Human, OutputFormat::Json, OutputFormat::Sarif];

    /// The name typed after `--format`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            OutputFormat::Human => "human",
            OutputFormat::Json => "json",
            OutputFormat::Sarif => "sarif",
        }
    }

    /// The inverse of [`OutputFormat::as_str`], exact-match only, as
    /// `LangId::from_name`: `JSON` is a typo worth naming, not a synonym.
    #[must_use]
    pub fn from_name(name: &str) -> Option<OutputFormat> {
        OutputFormat::ALL
            .iter()
            .copied()
            .find(|format| format.as_str() == name)
    }
}

/// What a scanning verb (`check`, `graph`, `lint`) runs over.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scan {
    /// How to render the result.
    pub format: OutputFormat,
    /// The paths as given, in the order given. Empty means "the tracked
    /// files" (`src/discover:V57`) -- the engine's call, not the parser's.
    pub paths: Vec<PathBuf>,
}

/// One `xnl extract` operand: `<path>` or `<path>:<line>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// The host file.
    pub path: PathBuf,
    /// The one site to extract, by 1-based line; `None` is every site in
    /// the file (`src/cli` §I).
    pub line: Option<usize>,
}

/// The verb asked for, with its own flags.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verb {
    /// `--version` / `-V`.
    Version,
    /// `xnl check`: detect embeds (`src/check:V152`).
    Check(Scan),
    /// `xnl extract`: move embeds into files of their own.
    Extract {
        /// `--write`: apply rather than print the diff.
        write: bool,
        /// `--relocate`: move extracts the layout no longer places
        /// (`src/extract:T101`).
        relocate: bool,
        /// At least one; the parser refuses none.
        targets: Vec<Target>,
    },
    /// `xnl graph`: host to extract load edges.
    Graph(Scan),
    /// `xnl lint`: run each extract's own linter.
    Lint {
        /// `--fix`: run fixers, then re-check.
        fix: bool,
        /// `--trust-config`: run commands a `xenolith.toml` defines
        /// (`src/lint:T92`).
        trust_config: bool,
        /// Paths and format.
        scan: Scan,
    },
    /// `xnl langs`: the languages this build knows. Its format is
    /// `human` or `json`, never `sarif` -- the parser refuses that.
    Langs {
        /// How to render the list.
        format: OutputFormat,
    },
    /// `xnl migrate`: legacy per-file allowlists into `xenolith.toml`
    /// (`src/cli:T97`). Takes no paths: the lists sit at the root under
    /// fixed names.
    Migrate {
        /// `--write`: create the file rather than print the diff.
        write: bool,
    },
}

impl Verb {
    /// The word typed to ask for this verb.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Verb::Version => "--version",
            Verb::Check(_) => "check",
            Verb::Extract { .. } => "extract",
            Verb::Graph(_) => "graph",
            Verb::Lint { .. } => "lint",
            Verb::Langs { .. } => "langs",
            Verb::Migrate { .. } => "migrate",
        }
    }
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    /// The verb and its own flags.
    pub verb: Verb,
    /// `--verbose`: say what was done even on success.
    pub verbose: bool,
    /// `--strict-hosts`: an unclaimed file is an error (`src/check:V13`).
    pub strict_hosts: bool,
}

/// A command line the parser refused, and why -- one line, naming the
/// word it could not accept.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Usage(pub String);

/// Every flag any verb takes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Flag {
    Verbose,
    StrictHosts,
    Format,
    Write,
    Relocate,
    Fix,
    TrustConfig,
}

impl Flag {
    const fn name(self) -> &'static str {
        match self {
            Flag::Verbose => "--verbose",
            Flag::StrictHosts => "--strict-hosts",
            Flag::Format => "--format",
            Flag::Write => "--write",
            Flag::Relocate => "--relocate",
            Flag::Fix => "--fix",
            Flag::TrustConfig => "--trust-config",
        }
    }
}

/// Which [`Verb`] a grammar row builds.
#[derive(Clone, Copy)]
enum Kind {
    Check,
    Extract,
    Graph,
    Lint,
    Langs,
    Migrate,
}

/// One verb's grammar: which flags, which formats.
struct Grammar {
    kind: Kind,
    verb: &'static str,
    flags: &'static [Flag],
    formats: &'static [OutputFormat],
}

const SCANNING: &[OutputFormat] = OutputFormat::ALL;
const LISTING: &[OutputFormat] = &[OutputFormat::Human, OutputFormat::Json];

/// `src/cli` §I, one row per verb.
const GRAMMARS: &[Grammar] = &[
    Grammar {
        kind: Kind::Check,
        verb: "check",
        flags: &[Flag::Verbose, Flag::StrictHosts, Flag::Format],
        formats: SCANNING,
    },
    Grammar {
        kind: Kind::Extract,
        verb: "extract",
        flags: &[
            Flag::Verbose,
            Flag::StrictHosts,
            Flag::Write,
            Flag::Relocate,
        ],
        formats: &[],
    },
    Grammar {
        kind: Kind::Graph,
        verb: "graph",
        flags: &[Flag::Verbose, Flag::StrictHosts, Flag::Format],
        formats: SCANNING,
    },
    Grammar {
        kind: Kind::Lint,
        verb: "lint",
        flags: &[
            Flag::Verbose,
            Flag::StrictHosts,
            Flag::Format,
            Flag::Fix,
            Flag::TrustConfig,
        ],
        formats: SCANNING,
    },
    Grammar {
        kind: Kind::Langs,
        verb: "langs",
        flags: &[Flag::Verbose, Flag::StrictHosts, Flag::Format],
        formats: LISTING,
    },
    Grammar {
        kind: Kind::Migrate,
        verb: "migrate",
        flags: &[Flag::Verbose, Flag::StrictHosts, Flag::Write],
        formats: &[],
    },
];

/// What the flags said, before it is shaped into a [`Verb`].
#[derive(Default)]
struct Seen {
    set: Vec<Flag>,
    format: Option<OutputFormat>,
    operands: Vec<OsString>,
}

impl Seen {
    fn has(&self, flag: Flag) -> bool {
        self.set.contains(&flag)
    }

    fn scan(&self) -> Scan {
        Scan {
            format: self.format.unwrap_or(OutputFormat::Human),
            paths: self.operands.iter().map(PathBuf::from).collect(),
        }
    }
}

/// Parse `args` (without the program name).
///
/// # Errors
///
/// [`Usage`] naming the first word that is not part of `src/cli` §I's
/// grammar, or the operand a verb cannot take.
pub fn parse<A: AsRef<OsStr>>(args: &[A]) -> Result<Invocation, Usage> {
    let Some((first, rest)) = args.split_first() else {
        return Err(Usage("no verb given".to_owned()));
    };
    let first = first.as_ref();
    if first == "--version" || first == "-V" {
        // Version looks at nothing after it: `xnl --version` must answer
        // even when a wrapper appends arguments.
        return Ok(Invocation {
            verb: Verb::Version,
            verbose: false,
            strict_hosts: false,
        });
    }
    let Some(grammar) = GRAMMARS.iter().find(|g| first == g.verb) else {
        return Err(Usage(unknown_verb(first)));
    };
    let seen = read_flags(grammar, rest)?;
    let verb = match grammar.kind {
        Kind::Check => Verb::Check(seen.scan()),
        Kind::Graph => Verb::Graph(seen.scan()),
        Kind::Lint => Verb::Lint {
            fix: seen.has(Flag::Fix),
            trust_config: seen.has(Flag::TrustConfig),
            scan: seen.scan(),
        },
        Kind::Extract => {
            if seen.operands.is_empty() {
                return Err(Usage(
                    "`extract` needs at least one <path>[:line]: extracting \
                     the whole tree is too big a rewrite to happen by \
                     leaving out an argument"
                        .to_owned(),
                ));
            }
            Verb::Extract {
                write: seen.has(Flag::Write),
                relocate: seen.has(Flag::Relocate),
                targets: seen
                    .operands
                    .iter()
                    .map(|op| target(op))
                    .collect::<Result<_, _>>()?,
            }
        }
        Kind::Langs => {
            no_paths(grammar, &seen)?;
            Verb::Langs {
                format: seen.format.unwrap_or(OutputFormat::Human),
            }
        }
        Kind::Migrate => {
            no_paths(grammar, &seen)?;
            Verb::Migrate {
                write: seen.has(Flag::Write),
            }
        }
    };
    Ok(Invocation {
        verb,
        verbose: seen.has(Flag::Verbose),
        strict_hosts: seen.has(Flag::StrictHosts),
    })
}

/// Refuse the first operand of a verb that takes none.
fn no_paths(grammar: &Grammar, seen: &Seen) -> Result<(), Usage> {
    match seen.operands.first() {
        Some(extra) => Err(Usage(format!(
            "`{}` takes no paths, got `{}`",
            grammar.verb,
            Path::new(extra).display()
        ))),
        None => Ok(()),
    }
}

fn unknown_verb(word: &OsStr) -> String {
    let shown = word.to_string_lossy();
    if is_flag(word) {
        format!("`{shown}` is not a verb; flags go after the verb")
    } else {
        format!("unknown verb `{shown}`")
    }
}

/// `-x`, `--x`: a word starting with `-` that is not `-` itself (a lone
/// dash is a path, by convention).
fn is_flag(word: &OsStr) -> bool {
    let bytes = word.as_encoded_bytes();
    bytes.len() > 1 && bytes.first() == Some(&b'-')
}

fn read_flags<A: AsRef<OsStr>>(grammar: &Grammar, rest: &[A]) -> Result<Seen, Usage> {
    let verb = grammar.verb;
    let mut seen = Seen::default();
    let mut words = rest.iter().map(AsRef::as_ref);
    let mut flags_over = false;
    while let Some(word) = words.next() {
        if flags_over || !is_flag(word) {
            seen.operands.push(word.to_owned());
            continue;
        }
        let Some(text) = word.to_str() else {
            return Err(Usage(format!(
                "`{}` is not a flag of `{verb}` (and not UTF-8); \
                 put `--` before a path that starts with `-`",
                word.to_string_lossy()
            )));
        };
        if text == "--" {
            flags_over = true;
            continue;
        }
        let (name, attached) = match text.split_once('=') {
            Some((name, value)) => (name, Some(value)),
            None => (text, None),
        };
        let Some(flag) = grammar.flags.iter().copied().find(|f| f.name() == name) else {
            return Err(Usage(format!("`{verb}` has no flag `{name}`")));
        };
        if flag == Flag::Format {
            let value = match attached {
                Some(value) => Some(value.to_owned()),
                None => words
                    .clone()
                    .next()
                    .filter(|next| !is_flag(next))
                    .map(|next| next.to_string_lossy().into_owned()),
            };
            if attached.is_none() && value.is_some() {
                words.next();
            }
            seen.format = Some(format(grammar, value.as_deref())?);
        } else if attached.is_some() {
            return Err(Usage(format!("`{name}` takes no value, got `{text}`")));
        } else if !seen.has(flag) {
            seen.set.push(flag);
        }
    }
    Ok(seen)
}

fn format(grammar: &Grammar, value: Option<&str>) -> Result<OutputFormat, Usage> {
    let choices: Vec<&str> = grammar.formats.iter().map(|f| f.as_str()).collect();
    let choices = choices.join(", ");
    let verb = grammar.verb;
    match value {
        None | Some("") => Err(Usage(format!(
            "`--format` needs a value for `{verb}`: {choices}"
        ))),
        Some(name) => OutputFormat::from_name(name)
            .filter(|f| grammar.formats.contains(f))
            .ok_or_else(|| {
                Usage(format!(
                    "`{verb}` has no format `{name}`; it takes {choices}"
                ))
            }),
    }
}

/// `<path>` or `<path>:<line>`. Only an all-digit suffix after the LAST
/// colon is a line: a colon is a legal filename character, so `a:b.nix`
/// and `c:` are files.
fn target(operand: &OsStr) -> Result<Target, Usage> {
    let whole = Target {
        path: PathBuf::from(operand),
        line: None,
    };
    // A name that is not UTF-8 cannot be split as text; it is a path
    // with no line, which is still a complete request.
    let Some(text) = operand.to_str() else {
        return Ok(whole);
    };
    let Some((path, digits)) = text.rsplit_once(':') else {
        return Ok(whole);
    };
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Ok(whole);
    }
    if path.is_empty() {
        return Err(Usage(format!("`{text}` names a line but no file")));
    }
    match digits.parse::<usize>() {
        Ok(0) => Err(Usage(format!(
            "`{text}`: lines count from 1, so line 0 names no site"
        ))),
        Ok(line) => Ok(Target {
            path: PathBuf::from(path),
            line: Some(line),
        }),
        Err(_) => Err(Usage(format!("`{text}`: the line number is too large"))),
    }
}
