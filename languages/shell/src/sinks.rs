//! Which commands hold a program, and where (`languages/shell` §I,
//! `languages/shell:V139`).
//!
//! Every answer here is about an ARGV, never about text: the host hands
//! over the command's name and its arguments as the grammar split them,
//! and this module says whether the interpreter was given a program in a
//! `-c`/`-e` argument, reads one from its heredoc, or neither.
//!
//! Each interpreter spells this differently -- bash bundles `-ec`, perl
//! `-ne`, node says `--eval`, psql reads SQL from stdin even with a
//! database argument -- so each gets its own reading, and anything a
//! reading does not recognise ends it with "no program here". A wrong
//! "yes" is a finding about text nothing runs; a wrong "no" is a site
//! missed, which a later task can widen deliberately.

use xenolith_lang_api::LangId;

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
}

/// Interpreters by name, version suffix stripped (`python3.12` is
/// `python`), and the guest each runs.
const INTERPRETERS: &[(&str, Kind, LangId)] = &[
    ("ash", Kind::Shell, LangId::Shell),
    ("bash", Kind::Shell, LangId::Shell),
    ("dash", Kind::Shell, LangId::Shell),
    ("ksh", Kind::Shell, LangId::Shell),
    ("node", Kind::Node, LangId::Js),
    ("nodejs", Kind::Node, LangId::Js),
    ("perl", Kind::Perl, LangId::Perl),
    ("psql", Kind::Psql, LangId::Sql),
    ("python", Kind::Python, LangId::Python),
    ("ruby", Kind::Ruby, LangId::Ruby),
    ("sh", Kind::Shell, LangId::Shell),
    ("zsh", Kind::Shell, LangId::Shell),
];

/// Shell invocation letters that are not `set` options: `-c` (command
/// string), `-s` (stdin), `-l` (login), `-i` (interactive).
const SHELL_INVOCATION: &[char] = &['c', 's', 'l', 'i'];

/// `set` letters bash and POSIX sh share, which a flag bundle may carry.
const SET_LETTERS: &[char] = &['a', 'e', 'f', 'u', 'v', 'x', 'C'];

/// zsh's letters differ: `-f` is `NO_RCS`, an invocation letter, and
/// only these four mean what the bash letters mean.
const ZSH_LETTERS: &[char] = &['e', 'u', 'v', 'x'];

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
}

/// The interpreter a PLAIN command word names, or `None`.
///
/// A path is read by its basename (`/usr/bin/python3`); `env python3`
/// and `sudo bash` name `env` and `sudo`, and are not read through
/// (`languages/shell:V139`).
pub(crate) fn interpreter(word: &str) -> Option<Interpreter<'_>> {
    let name = word.rsplit('/').next().unwrap_or(word);
    let bare = name.trim_end_matches(|c: char| c.is_ascii_digit() || c == '.');
    INTERPRETERS
        .iter()
        .find(|(known, _, _)| *known == bare)
        .map(|&(_, kind, guest)| Interpreter { kind, guest, name })
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
}

/// The argv of a shell, read up to its first operand; `None` for any
/// option this reading does not know (it may take a value).
fn shell_argv(zsh: bool, args: &[Option<&str>]) -> Option<ShellArgv> {
    let mut out = ShellArgv::default();
    let mut i = 0;
    while let Some(arg) = args.get(i) {
        let Some(word) = *arg else {
            out.operand = Some(i);
            return Some(out);
        };
        if word == "-" || word == "--" {
            out.operand = (i + 1 < args.len()).then_some(i + 1);
            return Some(out);
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
                    args.get(i).copied().flatten()?;
                } else if on && letter == 'c' {
                    out.command = true;
                } else if on && letter == 's' {
                    out.stdin = true;
                } else if !known_letter(zsh, letter) {
                    return None;
                }
            }
        } else {
            out.operand = Some(i);
            return Some(out);
        }
        i += 1;
    }
    Some(out)
}

/// Whether a letter in a shell's flag bundle is one this reading knows
/// takes no value: an invocation letter or a `set` letter of the dialect.
fn known_letter(zsh: bool, letter: char) -> bool {
    if zsh {
        // zsh's `-f` is `NO_RCS`: skip the startup files, set nothing.
        SHELL_INVOCATION.contains(&letter) || letter == 'f' || ZSH_LETTERS.contains(&letter)
    } else {
        SHELL_INVOCATION.contains(&letter) || SET_LETTERS.contains(&letter)
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
        Kind::Python => word == "-c",
        Kind::Node => word == "-e" || word == "--eval",
        Kind::Ruby => bundle(RUBY_BUNDLE, &['e']),
        Kind::Perl => bundle(PERL_BUNDLE, &['e', 'E']),
        Kind::Shell | Kind::Psql => false,
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
            let read = shell_argv(interpreter.name.starts_with("zsh"), args)?;
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
/// to it is then code, not data (`languages/shell:V139`).
pub(crate) fn stdin_is_program(interpreter: &Interpreter<'_>, args: &[Option<&str>]) -> bool {
    match interpreter.kind {
        Kind::Shell => shell_argv(interpreter.name.starts_with("zsh"), args)
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
