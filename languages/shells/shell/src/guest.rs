//! Shell as a guest: the extract file, and how it runs
//! (`languages/shells/shell:V51`, `languages/shells/shell:V82`).
//!
//! The prelude is where extraction either preserves the program or
//! changes it. A body that ran inside a host under `set -e` has to run
//! under `set -e` as a file; one that did not must not acquire it. So the
//! options the host observed are reproduced exactly -- not normalised to
//! a house style, and not dropped because this crate does not recognise a
//! name.

use std::path::Path;

use xenolith_lang_api::holes::{Bound, Param};
use xenolith_lang_api::{
    FileArg, Format, Guest, GuestEnv, Invoke, LangId, LintCmd, Prelude, Result, Shebang, Span,
};

use crate::classify::{ZSH_UNSUPPORTED, classify, classify_in};
use crate::params;

#[cfg(test)]
mod tests;

/// Shell, as both a guest and (later) a host.
///
/// A unit struct: everything it answers comes from the [`GuestEnv`] the
/// host established, so there is no per-instance state to get out of step
/// with the site. That is also what lets the registry hold one
/// `&'static dyn Guest` for shell (`src/registry:V41`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShellGuest;

/// The shell families this crate distinguishes (`languages/shells/shell:V82`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Family {
    /// POSIX sh, and the shells that stand in for it: dash, ksh.
    Posix,
    /// bash.
    Bash,
    /// zsh, whose options are `setopt` names rather than `set` flags.
    Zsh,
}

/// `set` flags this crate knows, in the order a rendered line uses them.
///
/// A FIXED order, so the same option set always renders the same bytes
/// (`src:V11`). Two flags in either order mean the same thing to a
/// shell and are two different diffs to a reviewer.
const SET_FLAGS: &[(&str, char)] = &[
    ("allexport", 'a'),
    ("errexit", 'e'),
    ("nounset", 'u'),
    ("verbose", 'v'),
    ("xtrace", 'x'),
    ("noclobber", 'C'),
];

/// zsh's name for each option this crate knows.
const ZSH_OPTIONS: &[(&str, &str)] = &[
    ("allexport", "all_export"),
    ("errexit", "err_exit"),
    ("nounset", "no_unset"),
    ("pipefail", "pipe_fail"),
    ("verbose", "verbose"),
    ("xtrace", "xtrace"),
];

/// The default when the host established nothing (`languages/shells/shell:V51`).
///
/// bash and strict, because the alternative is worse in one direction
/// only: a script extracted without `errexit` keeps running after a
/// failed command, and the failure surfaces somewhere else entirely.
const DEFAULT_STRICT: &str = "set -euo pipefail";

/// shfmt as a check, with NO parser or printer flag
/// (`languages/shells/shell:B5`).
///
/// Any such flag -- `--language-dialect` included -- makes shfmt ignore
/// `.editorconfig`, and a repo's indent then loses to shfmt's tabs. The
/// dialect needs no flag: shfmt reads it from the shebang, which every
/// extract carries (`languages/shells/shell:V51`).
const SHFMT_DIFF: &[&str] = &["shfmt", "--diff"];

/// shfmt as a fixer, flag-free for the same reason as [`SHFMT_DIFF`].
const SHFMT_WRITE: &[&str] = &["shfmt", "--write"];

impl ShellGuest {
    /// How to run a file of shell in a KNOWN dialect.
    ///
    /// Beside the trait's `invoke`, which has no env to read: an
    /// extracted `sh` script invoked with `bash` is a script running
    /// under a different shell than the one its host used.
    #[must_use]
    pub fn invoke_in(&self, path: &Path, env: &GuestEnv) -> Invoke {
        Invoke {
            argv: vec![dialect_name(env).to_owned(), path.display().to_string()],
        }
    }

    /// The why of the `Judgment` a body is owed INSTEAD of a verdict, or
    /// `None` when `trivial` and `constructs` decide it
    /// (`languages/shells/shell:V138`): zsh the bash grammar cannot read is
    /// ours to support, not the author's to fix.
    ///
    /// The body of [`Guest::unsupported`], kept here so its doc sits with
    /// the other shell-specific answers.
    fn zsh_unsupported(body: &str, env: &GuestEnv) -> Option<&'static str> {
        classify_in(body, env)
            .is_ok_and(|found| found.unsupported)
            .then_some(ZSH_UNSUPPORTED)
    }
}

impl Guest for ShellGuest {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    fn unsupported(&self, body: &str, env: &GuestEnv) -> Option<&'static str> {
        Self::zsh_unsupported(body, env)
    }

    fn extension(&self, env: &GuestEnv) -> &'static str {
        match family(env) {
            Family::Zsh => "zsh",
            Family::Posix | Family::Bash => "sh",
        }
    }

    fn invoke(&self, path: &Path) -> Invoke {
        self.invoke_in(path, &GuestEnv::default())
    }

    fn prelude(&self, env: &GuestEnv) -> Prelude {
        // No dialect means the host knew nothing about the site, which is
        // not the same as a site with no options: the first takes the
        // safe default, the second is a statement to be reproduced.
        if env.dialect.is_none() {
            return Prelude {
                shebang: Some(Shebang::env("bash")),
                strict: Some(DEFAULT_STRICT.to_owned()),
            };
        }
        Prelude {
            shebang: Some(Shebang::env(dialect_name(env))),
            strict: strict_line(env),
        }
    }

    fn executable(&self) -> bool {
        true
    }

    fn trivial(&self, body: &str) -> Result<bool> {
        // The classifier, not a second opinion (`languages/shells/shell:V3`).
        // "May this stay inline?" and "is this a script?" are one
        // question, and two implementations of it would eventually
        // answer differently.
        classify(body).map(|found| found.simple)
    }

    /// The classifier's construct names (`languages/shells/shell:V3`), the
    /// vocabulary `[threshold.shell] allow` is validated against
    /// (`src/config` §I), so the engine can relax per construct.
    fn constructs(&self, body: &str) -> Result<Vec<&'static str>> {
        classify(body).map(|found| {
            found
                .constructs
                .iter()
                .map(|construct| construct.as_str())
                .collect()
        })
    }

    fn checks(&self, env: &GuestEnv) -> Vec<LintCmd> {
        match family(env) {
            Family::Bash => vec![shellcheck("bash"), raw(SHFMT_DIFF)],
            Family::Posix => vec![
                shellcheck("sh"),
                // Bashisms in a file whose shebang says sh are the
                // failure that only appears on the machine where /bin/sh
                // is dash.
                raw(&["checkbashisms"]),
                raw(SHFMT_DIFF),
            ],
            // shellcheck does not read zsh at all, so offering it would
            // report syntax errors about a language it is not parsing.
            // `zsh -n` is a syntax check and the only one that reads zsh
            // whole (`languages/shells/shell:V310`).
            Family::Zsh => vec![raw(&["zsh", "-n"])],
        }
    }

    fn fixers(&self, env: &GuestEnv) -> Vec<LintCmd> {
        match family(env) {
            Family::Posix | Family::Bash => vec![raw(SHFMT_WRITE)],
            // No formatter reads zsh safely: shfmt's zsh dialect rejects
            // valid zsh and rewrites some of what it accepts into files
            // `zsh -n` refuses (`languages/shells/shell:V310`). An empty
            // list here is a statement, not a gap: `xnl lint --fix` has
            // nothing to run, and says so rather than pretending it fixed
            // something.
            Family::Zsh => Vec::new(),
        }
    }

    /// Holes as params (`languages/api/src/holes:V40`), all three parts
    /// from the one AST reading in `params`.
    fn vars(&self, body: &str) -> Result<Vec<String>> {
        params::vars(body)
    }

    fn params(&self, body: &str, params: &[Param]) -> Result<Bound> {
        params::params(body, params)
    }

    fn param_refs(&self, body: &str, names: &[String]) -> Result<Vec<(Span, String)>> {
        params::param_refs(body, names)
    }
}

/// A shellcheck invocation for one dialect.
///
/// JSON, because a parsed finding carries a line, a column and a code
/// (`src/lint:V92`), and the alternative is regexing human prose that
/// changes between releases.
fn shellcheck(dialect: &str) -> LintCmd {
    LintCmd {
        argv: vec![
            "shellcheck".to_owned(),
            format!("--shell={dialect}"),
            "--format=json".to_owned(),
        ],
        file_arg: FileArg::Append,
        format: Format::Json("shellcheck"),
    }
}

/// A command whose output has no machine-readable form.
fn raw(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}

/// The dialect name to put in a shebang and an invocation.
fn dialect_name(env: &GuestEnv) -> &str {
    env.dialect.as_deref().unwrap_or("bash")
}

/// Which family the site's dialect belongs to.
///
/// An unknown name is treated as sh rather than bash, which is the
/// conservative direction: a POSIX-only prelude runs under bash, while a
/// bash prelude does not run under dash.
pub(crate) fn family(env: &GuestEnv) -> Family {
    match dialect_name(env) {
        "bash" => Family::Bash,
        "zsh" => Family::Zsh,
        _ => Family::Posix,
    }
}

/// The `set` or `setopt` line reproducing the site's options, or `None`
/// when the site had none.
fn strict_line(env: &GuestEnv) -> Option<String> {
    if env.options.is_empty() {
        return None;
    }
    match family(env) {
        Family::Zsh => Some(zsh_setopt(&env.options)),
        Family::Bash => Some(set_line(&env.options, true)),
        // `pipefail` is dropped for sh, so a site whose only option it
        // was has nothing left to set. `set -` is not an empty line:
        // bash reads it as turning `-v` and `-x` off
        // (`languages/shells/shell:B3`).
        Family::Posix if env.options.iter().all(|option| option == "pipefail") => None,
        Family::Posix => Some(set_line(&env.options, false)),
    }
}

/// `setopt err_exit no_unset pipe_fail`.
fn zsh_setopt(options: &[String]) -> String {
    let mut names: Vec<&str> = options
        .iter()
        .map(|option| {
            ZSH_OPTIONS
                .iter()
                .find(|(posix, _)| *posix == option.as_str())
                .map_or(option.as_str(), |(_, zsh)| *zsh)
        })
        .collect();
    names.sort_unstable();
    names.dedup();
    format!("setopt {}", names.join(" "))
}

/// `set -euo pipefail`, and the sparser forms.
///
/// `pipefail` is rendered as the trailing `-o pipefail` that merges into
/// the letter run, because `set -euo pipefail` is the line people
/// recognise. It is DROPPED for sh, where it is not POSIX: writing it
/// into a `#!/usr/bin/env sh` file breaks the file under dash, which is
/// what `/bin/sh` is on Debian.
fn set_line(options: &[String], pipefail_supported: bool) -> String {
    let mut letters = String::new();
    for (name, flag) in SET_FLAGS {
        if options.iter().any(|option| option == name) {
            letters.push(*flag);
        }
    }

    let pipefail = pipefail_supported && options.iter().any(|option| option == "pipefail");

    // Anything this crate does not recognise is kept as `-o <name>`
    // rather than dropped. A dropped option changes the program
    // silently; an unknown one that reaches the shell fails loudly, and
    // loud is actionable.
    let mut unknown: Vec<&str> = options
        .iter()
        .map(String::as_str)
        .filter(|option| *option != "pipefail" && !SET_FLAGS.iter().any(|(name, _)| name == option))
        .collect();
    unknown.sort_unstable();
    unknown.dedup();

    let mut line = String::from("set -");
    line.push_str(&letters);
    if pipefail {
        line.push_str("o pipefail");
    }
    for option in unknown {
        line.push_str(" -o ");
        line.push_str(option);
    }
    // `set -` alone would enable nothing and look like a mistake; it can
    // only happen when every option the host reported is unknown.
    if letters.is_empty() && !pipefail {
        line = line.replacen("set - ", "set ", 1);
    }
    line
}
