//! Which commands hold a program, and where (`languages/shells/shell` §I,
//! `languages/shells/shell:V139`).
//!
//! Every answer here is about an ARGV, never about text: the host hands
//! over the command's name and its arguments as the grammar split them,
//! and this module says whether the interpreter was given a program in a
//! `-c`/`-e` argument, reads one from its heredoc, or neither.
//!
//! Each interpreter spells this differently -- bash bundles `-ec`, perl
//! `-ne`, node says `--eval`, psql reads SQL from stdin even with a
//! database argument, expect takes `-c` and tclsh takes none -- so each
//! gets its own reading, and anything a
//! reading does not recognise ends it with "no program here". A wrong
//! "yes" is a finding about text nothing runs; a wrong "no" is a site
//! missed, which a later task can widen deliberately.
//!
//! A shell's argv also says what the program runs UNDER
//! (`languages/shells/shell:V82`): the dialect is the interpreter's name, and
//! the options are the ones its flags set -- only those, since a child
//! shell does not inherit the enclosing script's `set`
//! (`languages/shells/shell:V139`).

use xenolith_lang_api::{GuestEnv, LangId};

#[cfg(test)]
mod tests;

/// An interpreter family: how its argv says where the program is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Kind {
    /// sh, bash, zsh, dash, ksh, ash: `-c` in a flag bundle.
    Shell,
    /// python: `-c`, exactly.
    Python,
    /// ruby: `-e`, alone or ending a bundle of switches (`-ne`).
    Ruby,
    /// perl: `-e` or `-E`, alone or ending a bundle (`-lne`).
    Perl,
    /// node: `-e` or `--eval`.
    Node,
    /// psql: no program argument this host reads; stdin is SQL.
    Psql,
    /// tclsh, wish: no program argument at all -- tclsh has no `-c`.
    Tcl,
    /// expect: `-c`, exactly; tcl in the expect dialect.
    Expect,
}

/// The dialect expect gives its tcl (`languages/shells/tcl:V197`).
const EXPECT: &str = "expect";

/// Script operands tcl reads its program from stdin for
/// (`languages/shells/shell:V139`).
const TCL_STDIN: &[&str] = &["-", "/dev/stdin"];

/// Interpreters by name, version suffix stripped (`python3.12` is
/// `python`), and the guest each runs.
const INTERPRETERS: &[(&str, Kind, LangId)] = &[
    ("ash", Kind::Shell, LangId::Shell),
    ("bash", Kind::Shell, LangId::Shell),
    ("dash", Kind::Shell, LangId::Shell),
    ("expect", Kind::Expect, LangId::Tcl),
    ("ksh", Kind::Shell, LangId::Shell),
    ("node", Kind::Node, LangId::Js),
    ("nodejs", Kind::Node, LangId::Js),
    ("perl", Kind::Perl, LangId::Perl),
    ("psql", Kind::Psql, LangId::Sql),
    ("python", Kind::Python, LangId::Python),
    ("ruby", Kind::Ruby, LangId::Ruby),
    ("sh", Kind::Shell, LangId::Shell),
    ("tclsh", Kind::Tcl, LangId::Tcl),
    ("wish", Kind::Tcl, LangId::Tcl),
    ("zsh", Kind::Shell, LangId::Shell),
];

/// Shell invocation letters that are not `set` options: `-c` (command
/// string), `-s` (stdin), `-l` (login), `-i` (interactive).
const SHELL_INVOCATION: &[char] = &['c', 's', 'l', 'i'];

/// `set` letters bash and POSIX sh share, which a flag bundle may carry,
/// and the option each sets, in the names [`GuestEnv`] uses.
const SET_LETTERS: &[(char, &str)] = &[
    ('a', "allexport"),
    ('e', "errexit"),
    ('f', "noglob"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
    ('C', "noclobber"),
];

/// zsh's letters differ: `-f` is `NO_RCS`, an invocation letter, and
/// only these four mean what the bash letters mean.
const ZSH_LETTERS: &[(char, &str)] = &[
    ('e', "errexit"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
];

/// The portable names zsh's `-o` spellings fold to. zsh reads option
/// names ignoring case and underscores, so `ERR_EXIT`, `err_exit` and
/// `errexit` are one option; the guest renders it back as zsh's own
/// (`languages/shells/shell:V51`).
const ZSH_PORTABLE: &[&str] = &[
    "allexport",
    "errexit",
    "nounset",
    "pipefail",
    "verbose",
    "xtrace",
];

/// Long options a shell takes WITHOUT a value; any other `--word` ends
/// the reading, since it may consume the next argument.
const SHELL_LONG: &[&str] = &["--login", "--noediting", "--noprofile", "--norc", "--posix"];

/// Ruby switches that may lead an `-e` in one bundle (`-ne`, `-lane`).
const RUBY_BUNDLE: &str = "nplaw";

/// Perl switches that may lead an `-e` in one bundle (`-lne`, `-wle`).
const PERL_BUNDLE: &str = "nplawsTtU";

/// The command is an interpreter this host reads: its family, its guest
/// and the name it was invoked as (basename, version kept).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Interpreter<'s> {
    /// How its argv reads.
    pub kind: Kind,
    /// The language it runs.
    pub guest: LangId,
    /// `python3`, `bash`: the basename as written.
    pub name: &'s str,
    /// The table's name for it, version stripped: `python`, `bash`. For
    /// a shell, this is the dialect.
    pub dialect: &'static str,
}

/// The interpreter a PLAIN command word names, or `None`.
///
/// A path is read by its basename (`/usr/bin/python3`); `env python3`
/// and `sudo bash` name `env` and `sudo`, and are not read through
/// (`languages/shells/shell:V139`).
pub(crate) fn interpreter(word: &str) -> Option<Interpreter<'_>> {
    let name = word.rsplit('/').next().unwrap_or(word);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    INTERPRETERS
        .iter()
        .find(|(known, _, _)| *known == bare)
        .map(|&(dialect, kind, guest)| Interpreter {
            kind,
            guest,
            name,
            dialect,
        })
}

impl Interpreter<'_> {
    fn is_zsh(&self) -> bool {
        self.dialect == "zsh"
    }
}

/// Where a shell's argv leaves off.
#[derive(Debug, Default, PartialEq, Eq)]
struct ShellArgv {
    /// `-c` was given: the first operand is the program.
    command: bool,
    /// `-s` was given: stdin is the program, operands are its arguments.
    stdin: bool,
    /// Index of the first operand, if any.
    operand: Option<usize>,
    /// Options the flags left on, sorted, deduplicated.
    options: Vec<String>,
}

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
                    toggle(&mut out.options, option_name(zsh, name), on);
                } else if on && letter == 'c' {
                    out.command = true;
                } else if on && letter == 's' {
                    out.stdin = true;
                } else if let Letter::Sets(name) = letter_option(zsh, letter)? {
                    toggle(&mut out.options, name.to_owned(), on);
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

/// Turn `name` on or off in `options`, keeping each name once.
fn toggle(options: &mut Vec<String>, name: String, on: bool) {
    options.retain(|option| *option != name);
    if on {
        options.push(name);
    }
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
    // zsh's `-f` is `NO_RCS`: skip the startup files, set nothing.
    if SHELL_INVOCATION.contains(&letter) || (zsh && letter == 'f') {
        return Some(Letter::Invocation);
    }
    let letters = if zsh { ZSH_LETTERS } else { SET_LETTERS };
    letters
        .iter()
        .find(|(known, _)| *known == letter)
        .map(|(_, name)| Letter::Sets(name))
}

/// An `-o` value as [`GuestEnv`] names it. bash and sh names are taken
/// as written; a zsh name folds case and underscores to the portable
/// name when it has one, and is otherwise kept as written, so an option
/// this crate does not know still reaches the prelude
/// (`languages/shells/shell:V82`).
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

/// Whether `word` is the flag that makes the NEXT argument the program.
fn is_eval_flag(kind: Kind, word: &str) -> bool {
    let bundle = |switches: &str, evals: &[char]| {
        let Some(rest) = word.strip_prefix('-') else {
            return false;
        };
        let mut letters = rest.chars().rev();
        letters.next().is_some_and(|last| evals.contains(&last))
            && letters.all(|letter| switches.contains(letter))
    };
    match kind {
        Kind::Python | Kind::Expect => word == "-c",
        Kind::Node => word == "-e" || word == "--eval",
        Kind::Ruby => bundle(RUBY_BUNDLE, &['e']),
        Kind::Perl => bundle(PERL_BUNDLE, &['e', 'E']),
        Kind::Shell | Kind::Psql | Kind::Tcl => false,
    }
}

/// The index of the argument holding the program, when the interpreter
/// was given one in a `-c`/`-e` flag.
///
/// `args` are the command's arguments, `Some(text)` for a plain word and
/// `None` for anything quoted or expanded, which is never a flag.
pub(crate) fn program_arg(interpreter: &Interpreter<'_>, args: &[Option<&str>]) -> Option<usize> {
    match interpreter.kind {
        Kind::Shell => {
            let read = shell_argv(interpreter.is_zsh(), args)?;
            if read.command { read.operand } else { None }
        }
        Kind::Psql => None,
        kind => {
            // Every argument before the flag is itself a flag: an operand
            // first is a script path, and the flag after it is the
            // script's own. A second eval flag means a program in pieces,
            // which one site cannot hold.
            let at = args
                .iter()
                .position(|arg| !arg.is_some_and(|w| w.starts_with('-')))?;
            let flag = args.get(at.checked_sub(1)?).copied().flatten()?;
            let before = args.get(..at.checked_sub(1)?)?;
            let flags_before = before
                .iter()
                .all(|arg| arg.is_some_and(|w| !is_eval_flag(kind, w)));
            let more = args
                .get(at + 1..)
                .unwrap_or_default()
                .iter()
                .any(|arg| arg.is_some_and(|w| is_eval_flag(kind, w)));
            (is_eval_flag(kind, flag) && flags_before && !more).then_some(at)
        }
    }
}

/// Whether the interpreter reads its PROGRAM from stdin -- a heredoc fed
/// to it is then code, not data (`languages/shells/shell:V139`).
pub(crate) fn stdin_is_program(interpreter: &Interpreter<'_>, args: &[Option<&str>]) -> bool {
    match interpreter.kind {
        Kind::Shell => shell_argv(interpreter.is_zsh(), args)
            .is_some_and(|argv| !argv.command && (argv.operand.is_none() || argv.stdin)),
        // A database name is an operand and changes nothing; a command,
        // a file or a listing means stdin is not read for SQL.
        Kind::Psql => !args.iter().flatten().any(|word| {
            word.starts_with("--command")
                || word.starts_with("--file")
                || word.starts_with("--list")
                || (!word.starts_with("--")
                    && word.starts_with('-')
                    && word.contains(['c', 'f', 'l']))
        }),
        // No argument, or a script operand that IS stdin. A flag first
        // (`expect -c`, `-f`, `tclsh -encoding`) or any other operand
        // ends the reading with "not stdin".
        Kind::Tcl | Kind::Expect => match args.first() {
            None => true,
            Some(Some(word)) => TCL_STDIN.contains(word),
            Some(None) => false,
        },
        kind => {
            for arg in args {
                match *arg {
                    // `-` is "the program is stdin"; what follows is its argv.
                    Some("-") => return true,
                    Some(word) if word.starts_with('-') && !is_eval_flag(kind, word) => {
                        // `python -m mod` runs a module, not stdin.
                        if kind == Kind::Python && word.starts_with("-m") {
                            return false;
                        }
                    }
                    _ => return false,
                }
            }
            true
        }
    }
}

/// What the program runs under (`languages/shells/shell:V82`).
///
/// For a shell: its dialect, and the options its own flags set. Not the
/// enclosing script's `set`, which a child process never sees
/// (`languages/shells/shell:V139`) -- `set -e; bash -c '...'` runs the child
/// WITHOUT errexit, and an extract given it would stop where the inline
/// body carried on. expect states its dialect and nothing else
/// (`languages/shells/tcl:V197`); any other interpreter states no env
/// here, and its guest applies its own defaults.
pub(crate) fn env(interpreter: &Interpreter<'_>, args: &[Option<&str>]) -> GuestEnv {
    match interpreter.kind {
        Kind::Shell => {}
        Kind::Expect => {
            return GuestEnv {
                dialect: Some(EXPECT.to_owned()),
                options: Vec::new(),
            };
        }
        _ => return GuestEnv::default(),
    }
    GuestEnv {
        dialect: Some(interpreter.dialect.to_owned()),
        options: shell_argv(interpreter.is_zsh(), args)
            .map(|read| read.options)
            .unwrap_or_default(),
    }
}
