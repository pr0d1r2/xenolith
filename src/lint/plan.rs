//! Which commands run on a file: the per-language linter map
//! (`src/lint:V8`, `src/lint` §I).
//!
//! The defaults are the language crate's -- `Guest::checks` for an
//! extract, `Host::checks` for a host file -- and never a literal here
//! (`src/config:V73`). A `xenolith.toml` adds to them (`extend = true`,
//! the table's default) or replaces them, and `[lint] all` adds to every
//! extract. A config command is text someone else may have written, so
//! it runs only under `--trust-config` (`src/lint:V91`); untrusted, it
//! is listed as skipped and the defaults still run.

use xenolith_lang_api::{FileArg, LintCmd};

use super::report::Source;
use crate::config::LintGuest;

#[cfg(test)]
mod tests;

/// The word a config command marks the file's place with.
pub const FILE: &str = "{file}";

/// One command, before it is given a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cmd {
    /// The command and its arguments; [`FILE`] where the path goes when
    /// `file_arg` is [`FileArg::Placeholder`].
    pub words: Vec<String>,
    /// Where the path goes.
    pub file_arg: FileArg,
    /// Default or config.
    pub source: Source,
}

impl Cmd {
    /// A language crate's command.
    #[must_use]
    pub fn builtin(cmd: &LintCmd) -> Cmd {
        Cmd {
            words: cmd.argv.clone(),
            file_arg: cmd.file_arg,
            source: Source::Default,
        }
    }

    /// A command as `xenolith.toml` writes it (`src/lint` §I): words
    /// split on whitespace, never handed to a shell, no quoting; a word
    /// that is exactly `{file}` is where the path goes, and with none the
    /// path is appended.
    #[must_use]
    pub fn configured(text: &str) -> Cmd {
        let words: Vec<String> = text.split_whitespace().map(str::to_owned).collect();
        let file_arg = if words.iter().any(|w| w == FILE) {
            FileArg::Placeholder
        } else {
            FileArg::Append
        };
        Cmd {
            words,
            file_arg,
            source: Source::Config,
        }
    }

    /// The argv for `file`. Empty when the command is: an empty command
    /// in config must not turn into "run the file".
    #[must_use]
    pub fn argv(&self, file: &str) -> Vec<String> {
        if self.words.is_empty() {
            return Vec::new();
        }
        match self.file_arg {
            FileArg::Append => {
                let mut argv = self.words.clone();
                argv.push(file.to_owned());
                argv
            }
            FileArg::Placeholder => self
                .words
                .iter()
                .map(|w| {
                    if w == FILE {
                        file.to_owned()
                    } else {
                        w.clone()
                    }
                })
                .collect(),
        }
    }

    /// The tool's name, as results report it: the first word.
    #[must_use]
    pub fn check(&self) -> String {
        self.words.first().cloned().unwrap_or_default()
    }
}

/// What to run on one extract, and what config asked for that is not run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// The commands to run, in order.
    pub run: Vec<Cmd>,
    /// Config commands held back for want of `--trust-config`.
    pub untrusted: Vec<Cmd>,
}

/// The config half of one extract's commands: its guest's
/// `[lint.<guest>]` list (checks or fixers), whether that list extends
/// the defaults, and `[lint] all` (checks only).
#[derive(Debug, Clone, Copy, Default)]
pub struct Configured<'a> {
    /// `[lint.<guest>] checks` or `fixers`.
    pub own: &'a [String],
    /// `[lint.<guest>] extend`, resolved through the defaults table.
    pub extend: bool,
    /// `[lint] all`.
    pub all: &'a [String],
}

impl<'a> Configured<'a> {
    /// The checks half of `entry`.
    #[must_use]
    pub fn checks(entry: Option<&'a LintGuest>, extend: bool, all: &'a [String]) -> Self {
        Configured {
            own: entry.map_or(&[], |e| e.checks.as_slice()),
            extend,
            all,
        }
    }

    /// The fixers half of `entry`; `[lint] all` holds checks only.
    #[must_use]
    pub fn fixers(entry: Option<&'a LintGuest>, extend: bool) -> Self {
        Configured {
            own: entry.map_or(&[], |e| e.fixers.as_slice()),
            extend,
            all: &[],
        }
    }
}

/// The commands for one extract: `defaults`, then config's own, then
/// `[lint] all` -- or, with `extend = false`, config's own in place of
/// the defaults. Untrusted, every config command is held back and the
/// defaults run whatever `extend` says (`src/lint:V91`).
#[must_use]
pub fn plan(defaults: &[LintCmd], configured: Configured<'_>, trusted: bool) -> Plan {
    let builtin = defaults.iter().map(Cmd::builtin);
    let from_config: Vec<Cmd> = configured
        .own
        .iter()
        .chain(configured.all)
        .map(|text| Cmd::configured(text))
        .collect();
    if !trusted {
        return Plan {
            run: builtin.collect(),
            untrusted: from_config,
        };
    }
    let mut run: Vec<Cmd> = if configured.extend {
        builtin.collect()
    } else {
        Vec::new()
    };
    run.extend(from_config);
    Plan {
        run,
        untrusted: Vec::new(),
    }
}
