//! The shell hk runs a step's command under (`languages/ci/pkl:V172`).
//!
//! hk hands a step's command to a shell as one `-c` argument: the step's
//! own `shell`, else the one its `Group` sets for its children, else hk's
//! `sh -o errexit -c` (`pkl/Config.pkl`, `Step.shell`; hk 1.58.1 runs
//! exactly that argv). A site's `env` is that shell's dialect and `set`
//! state, so an extract's prelude reproduces what the inline step ran
//! under (`languages/shells/shell:V82`) -- errexit, and not the `nounset` and
//! `pipefail` the guest's own default would add (`languages/ci/pkl:B3`).
//!
//! A `shell` this crate cannot read as one such command line -- a per-OS
//! `Script`, an interpolated string, a flag it does not know -- leaves
//! hk's default standing: V172 says so, and the alternative, the guest
//! default, is the stricter program B3 is about.

use tree_sitter::Node;
use xenolith_lang_api::GuestEnv;

use crate::host::{plain_string, text};

#[cfg(test)]
mod tests;

/// hk's shell for a step that names none (`pkl/Config.pkl`,
/// `Step.shell`).
pub(crate) const HK_DEFAULT: &str = "sh -o errexit -c";

/// The dialects a site env may name (`languages/shells/shell:V82`).
const DIALECTS: &[&str] = &["sh", "bash", "zsh"];

/// `set` letters sh and bash share, and the option each sets, in the
/// names [`GuestEnv`] uses.
const SET_LETTERS: &[(char, &str)] = &[
    ('a', "allexport"),
    ('e', "errexit"),
    ('f', "noglob"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
    ('C', "noclobber"),
];

/// zsh's letters differ (`-f` is `NO_RCS`); only these mean what the
/// sh letters mean.
const ZSH_LETTERS: &[(char, &str)] = &[
    ('e', "errexit"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
];

/// `-o` names read as they are. zsh spells them its own way too
/// (`err_exit`, `ERR_EXIT`), which folds to these.
const OPTION_NAMES: &[&str] = &[
    "allexport",
    "errexit",
    "noclobber",
    "noglob",
    "nounset",
    "pipefail",
    "verbose",
    "xtrace",
];

/// hk's default as an env: `sh` with `errexit`, read off [`HK_DEFAULT`]
/// so the two cannot drift. It always reads; the fallback is there only
/// because `read` answers `Option`, and the tests pin the result.
pub(crate) fn hk_default() -> GuestEnv {
    read(HK_DEFAULT).unwrap_or_default()
}

/// The env of a hk `shell` command line: `<sh|bash|zsh> [-<letters>|-o
/// <name>]… -c`, the interpreter by its basename, or `None` for anything
/// else.
pub(crate) fn read(command: &str) -> Option<GuestEnv> {
    let words: Vec<&str> = command.split_whitespace().collect();
    let [interpreter, flags @ .., last] = words.as_slice() else {
        return None;
    };
    let dialect = interpreter.rsplit('/').next().unwrap_or(interpreter);
    if !DIALECTS.contains(&dialect) {
        return None;
    }
    // hk appends the command as the next argument, so the line ends in
    // `-c`, alone or closing a letter bundle (`-ec`).
    let tail = last.strip_prefix('-')?.strip_suffix('c')?;
    let letters = if dialect == "zsh" {
        ZSH_LETTERS
    } else {
        SET_LETTERS
    };
    let mut options = Vec::new();
    let mut words = flags.iter();
    while let Some(word) = words.next() {
        if *word == "-o" {
            options.push(option_name(words.next()?, dialect)?);
        } else {
            options.extend(bundle(word.strip_prefix('-')?, letters)?);
        }
    }
    options.extend(bundle(tail, letters)?);
    let mut seen = Vec::new();
    options.retain(|option| {
        let fresh = !seen.contains(option);
        seen.push(*option);
        fresh
    });
    Some(GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.into_iter().map(str::to_owned).collect(),
    })
}

/// The options a letter bundle sets, or `None` when one letter is not a
/// `set` option of the dialect. An empty bundle sets none.
fn bundle(letters: &str, table: &[(char, &'static str)]) -> Option<Vec<&'static str>> {
    letters
        .chars()
        .map(|letter| {
            table
                .iter()
                .find(|(known, _)| *known == letter)
                .map(|(_, name)| *name)
        })
        .collect()
}

/// A `-o` name as [`GuestEnv`] spells it; zsh's case and underscores
/// fold away.
fn option_name(name: &str, dialect: &str) -> Option<&'static str> {
    let folded = if dialect == "zsh" {
        name.to_ascii_lowercase().replace('_', "")
    } else {
        name.to_owned()
    };
    OPTION_NAMES.iter().copied().find(|known| *known == folded)
}

/// The value of the `shell` property directly in an object body.
fn shell_value<'t>(body: Node<'t>, src: &str) -> Option<Node<'t>> {
    let mut cursor = body.walk();
    body.named_children(&mut cursor)
        .filter(|child| child.kind() == "objectProperty")
        .find_map(|property| {
            let mut cursor = property.walk();
            let named: Vec<Node<'t>> = property.named_children(&mut cursor).collect();
            match named.as_slice() {
                // A `local shell` leads with its modifier, not the name.
                [name, .., value] if name.kind() == "identifier" && text(*name, src) == "shell" => {
                    Some(*value)
                }
                _ => None,
            }
        })
}

/// The body of the object whose `steps` hold the step `entry`: a
/// `Group`, or a hook, which has no `shell` of its own.
fn owner<'t>(entry: Node<'t>, src: &str) -> Option<Node<'t>> {
    let mut up = entry
        .parent()
        .filter(|p| p.kind() == "objectBody")?
        .parent()?;
    if up.kind() == "newExpr" {
        up = up.parent()?;
    }
    let steps = Some(up).filter(|p| p.kind() == "objectProperty")?;
    let name = steps.named_child(0)?;
    if name.kind() != "identifier" || text(name, src) != "steps" {
        return None;
    }
    steps.parent().filter(|p| p.kind() == "objectBody")
}

/// The env of the step whose body is `body` and whose entry is `entry`
/// (`languages/ci/pkl:V172`).
pub(crate) fn env(entry: Node<'_>, body: Node<'_>, src: &str) -> GuestEnv {
    let value = shell_value(body, src)
        .or_else(|| owner(entry, src).and_then(|group| shell_value(group, src)));
    value
        .and_then(|value| plain_string(value, src))
        .and_then(read)
        .unwrap_or_else(hk_default)
}
