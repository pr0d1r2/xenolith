//! The shell a recipe line runs under (`languages/ci/just:V179`).
//!
//! just hands every line of a recipe without a shebang to `set shell`'s
//! argv with the line appended, `sh -cu` when the file sets none. A
//! site's env is that argv's dialect and `set` state, plus `errexit`:
//! just stops at the first line that fails, which is what `set -e` does
//! to the lines of one script (`languages/ci/just:V180`). The default
//! is therefore `sh` with `errexit` and `nounset`, the prelude `set -eu`.
//!
//! READABLE means statically: a list of plain string literals naming
//! `sh`, `bash` or `zsh` with flags this module knows. Anything else --
//! an escape in a string, a flag it does not know, another interpreter,
//! `set windows-shell` with no `set shell` beside it, an `import` that
//! could set it -- is unreadable, and unreadable is a judgement
//! (`Judgment`), never a guess.

use xenolith_lang_api::GuestEnv;

use crate::recipe::Settings;

#[cfg(test)]
mod tests;

/// just's shell when the file sets none.
pub(crate) const JUST_DEFAULT: &[&str] = &["sh", "-cu"];

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

/// zsh's letters differ (`-f` is `NO_RCS`); only these mean what the sh
/// letters mean.
const ZSH_LETTERS: &[(char, &str)] = &[
    ('e', "errexit"),
    ('u', "nounset"),
    ('v', "verbose"),
    ('x', "xtrace"),
];

/// `-o` names read as they are; zsh's spellings (`err_exit`,
/// `ERR_EXIT`) fold to these.
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

/// The shell a file's recipe lines run under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LineShell {
    /// The env of every line site: dialect, the argv's options, and
    /// `errexit` for just's stop at the first failing line.
    pub env: GuestEnv,
    /// Whether the argv itself sets `errexit`, so one line's `a; b`
    /// already stops at `a` exactly as a merged script would.
    pub errexit: bool,
}

/// The line shell of a file with `settings`, or `None` when it cannot be
/// read statically (`languages/ci/just:V179`).
pub(crate) fn line_shell(settings: &Settings) -> Option<LineShell> {
    let argv: Vec<String> = match &settings.shell {
        Some(strings) => strings
            .iter()
            .map(|s| literal(s))
            .collect::<Option<Vec<_>>>()?,
        None if settings.windows_shell || settings.imports => return None,
        None => JUST_DEFAULT.iter().map(|&w| w.to_owned()).collect(),
    };
    let (dialect, options) = read(&argv)?;
    let errexit = options.contains(&"errexit");
    let mut all = vec!["errexit"];
    all.extend(options);
    let mut seen = Vec::new();
    all.retain(|option| {
        let fresh = !seen.contains(option);
        seen.push(*option);
        fresh
    });
    Some(LineShell {
        env: GuestEnv {
            dialect: Some(dialect.to_owned()),
            options: all.into_iter().map(str::to_owned).collect(),
        },
        errexit,
    })
}

/// The value of a string literal as written, when it holds no escape
/// and is not indented: `"bash"` or `'bash'`.
pub(crate) fn literal(written: &str) -> Option<String> {
    let inner = written
        .strip_prefix('"')
        .and_then(|rest| rest.strip_suffix('"'))
        .filter(|inner| !inner.contains(['\\', '"']))
        .or_else(|| {
            written
                .strip_prefix('\'')
                .and_then(|rest| rest.strip_suffix('\''))
                .filter(|inner| !inner.contains('\''))
        })?;
    Some(inner.to_owned())
}

/// The dialect and options of a shell argv: `<sh|bash|zsh>
/// [-<letters>|-o <name>]...`, one letter bundle holding the `c` that
/// takes the line. `None` for anything else.
pub(crate) fn read(argv: &[String]) -> Option<(&'static str, Vec<&'static str>)> {
    let (interpreter, flags) = argv.split_first()?;
    let base = interpreter.rsplit('/').next().unwrap_or(interpreter);
    let dialect = DIALECTS.iter().copied().find(|d| *d == base)?;
    let letters = if dialect == "zsh" {
        ZSH_LETTERS
    } else {
        SET_LETTERS
    };
    let mut options = Vec::new();
    let mut command = false;
    let mut words = flags.iter();
    while let Some(word) = words.next() {
        let bundle = word.strip_prefix('-').filter(|b| !b.starts_with('-'))?;
        if bundle.is_empty() {
            return None;
        }
        for letter in bundle.chars() {
            match letter {
                'c' if !command => command = true,
                'o' => options.push(option_name(words.next()?, dialect)?),
                _ => options.push(
                    letters
                        .iter()
                        .find(|(known, _)| *known == letter)
                        .map(|(_, name)| *name)?,
                ),
            }
        }
    }
    command.then_some((dialect, options))
}

/// A `-o` name as [`GuestEnv`] spells it; zsh's case and underscores fold
/// away.
fn option_name(name: &str, dialect: &str) -> Option<&'static str> {
    let folded = if dialect == "zsh" {
        name.to_ascii_lowercase().replace('_', "")
    } else {
        name.to_owned()
    };
    OPTION_NAMES.iter().copied().find(|known| *known == folded)
}
