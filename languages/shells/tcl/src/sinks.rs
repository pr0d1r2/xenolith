//! Which `exec`/`spawn` argvs hold a program, and where
//! (`languages/shells/tcl:V196`).
//!
//! Every answer here is about an ARGV, never about text: the host hands
//! over the words the grammar split, each `Some(text)` when it is a plain
//! word and `None` when it is braced, quoted or substituted -- which can
//! never be a flag. This module says whether the interpreter was handed a
//! program in a `-c` word, reads one from Tcl's `<<` redirection, or
//! neither. Anything a reading does not recognise ends it with "no
//! program here": a wrong "yes" is a finding about text nothing runs, a
//! wrong "no" is a site missed that a later task can widen on purpose.
//!
//! A shell's argv also says what its program runs UNDER: the dialect is
//! the interpreter's name and the options are the ones its own flags set
//! -- the argv form of `languages/shells/shell:V139`. The shell crate has
//! the same reading for bash hosts, but a language crate may not depend
//! on another (`languages/api:V32`), so the few rules it needs are
//! restated here.

use xenolith_lang_api::{GuestEnv, LangId};

#[cfg(test)]
mod tests;

/// The Tcl commands that run an argv: `exec` in plain Tcl, `spawn` in the
/// expect dialect (`languages/shells/tcl:V195`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Runner {
    /// `exec`: runs the argv, and alone reads Tcl's `<< value` redirection.
    Exec,
    /// `spawn`: starts the argv under expect's control.
    Spawn,
}

impl Runner {
    /// The runner a command word names, or `None`.
    pub(crate) fn of(word: &str) -> Option<Runner> {
        match word {
            "exec" => Some(Runner::Exec),
            "spawn" => Some(Runner::Spawn),
            _ => None,
        }
    }

    /// The command as written, for the site's sink.
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Runner::Exec => "exec",
            Runner::Spawn => "spawn",
        }
    }
}

/// The shells whose `-c` word is a site (`languages/shells/tcl:V196`).
const SHELLS: &[&str] = &["bash", "dash", "sh", "zsh"];

/// Interpreters whose stdin, fed by `<<`, is their PROGRAM, and the guest
/// each runs. `awk` and `jq` are absent on purpose: their stdin is data
/// (`languages/shells/shell:V139`).
const STDIN_PROGRAMS: &[(&str, LangId)] = &[
    ("bash", LangId::Shell),
    ("dash", LangId::Shell),
    ("expect", LangId::Tcl),
    ("node", LangId::Js),
    ("nodejs", LangId::Js),
    ("perl", LangId::Perl),
    ("psql", LangId::Sql),
    ("python", LangId::Python),
    ("ruby", LangId::Ruby),
    ("sh", LangId::Shell),
    ("tclsh", LangId::Tcl),
    ("wish", LangId::Tcl),
    ("zsh", LangId::Shell),
];

/// Tcl's redirection that feeds a VALUE to stdin.
pub(crate) const FEED: &str = "<<";

/// The interpreter a PLAIN word names: its basename as written and its
/// table name with any version suffix stripped (`python3` is `python`,
/// `tclsh8.6` is `tclsh`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Interpreter<'s> {
    /// `python3`, `bash`: the basename as written.
    pub name: &'s str,
    /// The bare name, version stripped.
    pub bare: &'s str,
}

/// The interpreter `word` names, read by its basename
/// (`/usr/bin/python3`). A word that is not plain never reaches here.
pub(crate) fn interpreter(word: &str) -> Interpreter<'_> {
    let name = word.rsplit('/').next().unwrap_or(word);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    Interpreter { name, bare }
}

/// Where the program of an `exec`/`spawn` argv is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Program {
    /// Index into the arguments AFTER the interpreter of the word that
    /// holds the program.
    pub at: usize,
    /// The guest it is written in.
    pub guest: LangId,
    /// What it runs under.
    pub env: GuestEnv,
    /// `-c` or `<<`, for the sink's name.
    pub how: &'static str,
}

/// The program of `runner interpreter args...`, if the argv holds one
/// (`languages/shells/tcl:V196`): a shell's `-c` word, or, for `exec`
/// only, the word after `<<` when `<<` comes straight after the
/// interpreter and no later word feeds stdin again.
pub(crate) fn program(
    runner: Runner,
    interpreter: Interpreter<'_>,
    args: &[Option<&str>],
) -> Option<Program> {
    if runner == Runner::Exec && args.first() == Some(&Some(FEED)) {
        return feed(interpreter, args);
    }
    if !SHELLS.contains(&interpreter.bare) {
        return None;
    }
    let read = shell_argv(interpreter.bare == "zsh", args)?;
    let at = read.operand.filter(|_| read.command)?;
    Some(Program {
        at,
        guest: LangId::Shell,
        env: GuestEnv {
            dialect: Some(interpreter.bare.to_owned()),
            options: read.options,
        },
        how: "-c",
    })
}

/// `exec <interp> << <word>`: the value is the interpreter's program.
fn feed(interpreter: Interpreter<'_>, args: &[Option<&str>]) -> Option<Program> {
    let guest = STDIN_PROGRAMS
        .iter()
        .find(|(name, _)| *name == interpreter.bare)
        .map(|(_, guest)| *guest)?;
    // A second stdin (`< file`, `<@ chan`, another `<<`) leaves it
    // unclear which one the interpreter reads.
    let again = args
        .get(2..)
        .unwrap_or_default()
        .iter()
        .any(|arg| arg.is_some_and(|word| word.starts_with('<')));
    if again || args.len() < 2 {
        return None;
    }
    let env = match (guest, interpreter.bare) {
        (LangId::Shell, dialect) => GuestEnv {
            dialect: Some(dialect.to_owned()),
            options: Vec::new(),
        },
        (LangId::Tcl, "expect") => GuestEnv {
            dialect: Some("expect".to_owned()),
            options: Vec::new(),
        },
        _ => GuestEnv::default(),
    };
    Some(Program {
        at: 1,
        guest,
        env,
        how: FEED,
    })
}

/// Where a shell's argv leaves off.
#[derive(Debug, Default, PartialEq, Eq)]
struct ShellArgv {
    /// `-c` was given: the first operand is the program.
    command: bool,
    /// Index of the first operand, if any.
    operand: Option<usize>,
    /// Options the flags left on, sorted, deduplicated.
    options: Vec<String>,
}

/// Invocation letters that set no option: `-c` (command string), `-s`
/// (stdin), `-l` (login), `-i` (interactive).
const INVOCATION: &[char] = &['c', 's', 'l', 'i'];

/// `set` letters sh and bash share, and the option each sets.
const SET_LETTERS: &[(char, &str)] = &[
    ('a', "allexport"),
    ('e', "errexit"),
    ('f', "noglob"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
    ('C', "noclobber"),
];

/// zsh's letters: `-f` is `NO_RCS`, an invocation letter, and only these
/// four mean what the bash letters mean.
const ZSH_LETTERS: &[(char, &str)] = &[
    ('e', "errexit"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
];

/// Long options a shell takes WITHOUT a value; any other `--word` ends
/// the reading, since it may consume the next argument.
const SHELL_LONG: &[&str] = &["--login", "--noediting", "--noprofile", "--norc", "--posix"];

/// The argv of a shell, read up to its first operand; `None` for any
/// option this reading does not know (it may take a value).
fn shell_argv(zsh: bool, args: &[Option<&str>]) -> Option<ShellArgv> {
    let mut out = ShellArgv::default();
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let Some(word) = *arg else {
            out.operand = Some(i);
            break;
        };
        if word == "-" || word == "--" {
            out.operand = (i + 1 < args.len()).then_some(i + 1);
            break;
        }
        if word.starts_with("--") {
            if !SHELL_LONG.contains(&word) {
                return None;
            }
        } else if word.len() > 1 && (word.starts_with('-') || word.starts_with('+')) {
            let on = word.starts_with('-');
            for letter in word.chars().skip(1) {
                if letter == 'o' {
                    // `-o NAME` / `+o NAME`: the value is the next word.
                    i += 1;
                    let name = args.get(i).copied().flatten()?;
                    toggle(&mut out.options, &option_name(zsh, name), on);
                } else if on && letter == 'c' {
                    out.command = true;
                } else if let Letter::Sets(name) = letter_option(zsh, letter)? {
                    toggle(&mut out.options, name, on);
                }
            }
        } else {
            out.operand = Some(i);
            break;
        }
        i += 1;
    }
    out.options.sort_unstable();
    Some(out)
}

/// A letter this reading knows in a shell's flag bundle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Letter {
    /// A `set` letter of the dialect, and the option it names.
    Sets(&'static str),
    /// An invocation letter, which sets no option.
    Invocation,
}

/// What a letter in a shell's flag bundle means, or `None` for one this
/// reading does not know -- it may take a value, so the reading stops.
fn letter_option(zsh: bool, letter: char) -> Option<Letter> {
    if INVOCATION.contains(&letter) || (zsh && letter == 'f') {
        return Some(Letter::Invocation);
    }
    let letters = if zsh { ZSH_LETTERS } else { SET_LETTERS };
    letters
        .iter()
        .find(|(known, _)| *known == letter)
        .map(|(_, name)| Letter::Sets(name))
}

/// The portable names zsh's `-o` spellings fold to: zsh reads option
/// names ignoring case and underscores, so `ERR_EXIT` is `errexit`.
const ZSH_PORTABLE: &[&str] = &[
    "allexport",
    "errexit",
    "nounset",
    "pipefail",
    "verbose",
    "xtrace",
];

/// An `-o` value as [`GuestEnv`] names it: as written, except a zsh name
/// that folds to a portable one, as the shell crate names it.
fn option_name(zsh: bool, name: &str) -> String {
    if !zsh {
        return name.to_owned();
    }
    let folded: String = name
        .chars()
        .filter(|c| *c != '_')
        .map(|c| c.to_ascii_lowercase())
        .collect();
    if ZSH_PORTABLE.contains(&folded.as_str()) {
        folded
    } else {
        name.to_owned()
    }
}

/// Turn `name` on or off in `options`, keeping each name once.
fn toggle(options: &mut Vec<String>, name: &str, on: bool) {
    options.retain(|option| option != name);
    if on {
        options.push(name.to_owned());
    }
}
