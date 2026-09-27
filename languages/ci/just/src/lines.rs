//! A recipe body as text: the lines just runs, and the way back.
//!
//! A body without a shebang is LINES (`languages/ci/just:V180`): just
//! strips the recipe's indent, reads an `@` / `-` prefix off each line,
//! joins a line ending in `\` with the next (the `\` dropped, the next
//! line's indent too), turns `{{{{` into a literal `{{`, and skips blank
//! lines. [`unescape`] does exactly that, so the guest judges what just
//! would run (`languages/api/src/lens:V39`). A shebang body is one
//! SCRIPT: indent and `{{{{` only, blank lines kept.
//!
//! [`escape`] is the inverse for a body [`unescape`] could have produced;
//! [`merged`] and [`written`] are the extract direction's pair: lines
//! merged into one script and read back into lines.

use xenolith_lang_api::{Delim, DelimKind, Error, LangId, Result};

#[cfg(test)]
mod tests;

/// The indent [`escape`] writes: just's own formatter's.
pub(crate) const INDENT: &str = "    ";

/// What a `-` line becomes in a merged script: its failure ignored, as
/// just ignores it (`languages/ci/just:V180`).
pub(crate) const INFALLIBLE: &str = " || true";

/// The prefixes just reads off a line, longest first.
const PREFIXES: &[&str] = &["@-", "-@", "@", "-"];

/// One line as just runs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Logical {
    /// The prefix as written: `""`, `@`, `-`, `@-` or `-@`.
    pub prefix: String,
    /// The command, prefix and indent gone, continuations joined,
    /// `{{{{` read as `{{`.
    pub text: String,
    /// Whether it was continued over more than one line with `\`.
    pub continued: bool,
}

impl Logical {
    /// `@`: just does not echo it.
    pub(crate) fn quiet(&self) -> bool {
        self.prefix.contains('@')
    }

    /// `-`: just ignores its failure.
    pub(crate) fn infallible(&self) -> bool {
        self.prefix.contains('-')
    }
}

/// The body as its guest reads it (`languages/api/src/lens:V39`).
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter that is not a just recipe's,
/// and [`Error::Parse`] for a line indented less than the first.
pub fn unescape(delim: &Delim, raw: &str) -> Result<String> {
    match delim.kind {
        DelimKind::JustRecipe => Ok(logical(raw)?
            .into_iter()
            .map(|line| line.text)
            .collect::<Vec<_>>()
            .join("\n")),
        DelimKind::JustShebangRecipe => script(raw),
        _ => Err(Error::unsupported(LangId::Just, "unescape")),
    }
}

/// The inverse of [`unescape`] for a body it could have produced: each
/// line indented by [`INDENT`], `{{` written `{{{{`.
///
/// # Errors
///
/// [`Error::Unsupported`] for a delimiter that is not a just recipe's,
/// and [`Error::Parse`] for a line just would read differently: the first
/// one indented, and in a line body one starting with `@` or `-` (read as
/// a prefix) or ending in `\` (read as a continuation).
pub fn escape(delim: &Delim, body: &str) -> Result<String> {
    let linewise = match delim.kind {
        DelimKind::JustRecipe => true,
        DelimKind::JustShebangRecipe => false,
        _ => return Err(Error::unsupported(LangId::Just, "escape")),
    };
    let mut out = Vec::new();
    for (i, line) in body.split('\n').enumerate() {
        if i == 0 && line.starts_with([' ', '\t']) {
            return Err(unwritable(line, "the first line is indented"));
        }
        if linewise {
            if line.starts_with(['@', '-']) {
                return Err(unwritable(
                    line,
                    "just reads its first character as a prefix",
                ));
            }
            if line.ends_with('\\') {
                return Err(unwritable(line, "just reads it as continued"));
            }
        }
        out.push(indented(line));
    }
    Ok(out.join("\n"))
}

fn unwritable(line: &str, why: &str) -> Error {
    Error::parse(
        LangId::Just,
        format!("cannot write {line:?} into a recipe: {why}"),
    )
}

/// `line` under [`INDENT`] with `{{` escaped; an empty line stays empty.
fn indented(line: &str) -> String {
    if line.is_empty() {
        String::new()
    } else {
        format!("{INDENT}{}", line.replace("{{", "{{{{"))
    }
}

/// The physical lines of `raw` with the recipe's indent -- the first
/// non-blank line's -- stripped; `None` for a blank line.
fn dedented(raw: &str) -> Result<Vec<Option<&str>>> {
    let lines: Vec<&str> = raw
        .split('\n')
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect();
    let indent = lines
        .iter()
        .find(|line| !line.trim().is_empty())
        .map_or("", |line| {
            let body = line.trim_start_matches([' ', '\t']);
            line.get(..line.len() - body.len()).unwrap_or_default()
        });
    lines
        .into_iter()
        .map(|line| {
            if line.trim().is_empty() {
                Ok(None)
            } else {
                line.strip_prefix(indent).map(Some).ok_or_else(|| {
                    Error::parse(
                        LangId::Just,
                        format!("{line:?} is indented less than the recipe's first line"),
                    )
                })
            }
        })
        .collect()
}

/// A shebang body: dedented, blank lines kept empty, `{{{{` read.
fn script(raw: &str) -> Result<String> {
    Ok(dedented(raw)?
        .into_iter()
        .map(|line| line.unwrap_or_default().replace("{{{{", "{{"))
        .collect::<Vec<_>>()
        .join("\n"))
}

/// A line body's lines as just runs them.
///
/// # Errors
///
/// [`Error::Parse`] for a line indented less than the first.
pub(crate) fn logical(raw: &str) -> Result<Vec<Logical>> {
    let mut out: Vec<Logical> = Vec::new();
    let mut continuing = false;
    for line in dedented(raw)? {
        let text = line.unwrap_or_default();
        let (prefix, command, joined) = match out.last_mut() {
            Some(last) if continuing => {
                last.continued = true;
                ("", text.trim_start(), Some(last))
            }
            _ if line.is_none() => continue,
            _ => {
                let prefix = PREFIXES
                    .iter()
                    .copied()
                    .find(|p| text.starts_with(p))
                    .unwrap_or("");
                (prefix, text.get(prefix.len()..).unwrap_or_default(), None)
            }
        };
        continuing = command.ends_with('\\');
        let command = if continuing {
            command.get(..command.len() - 1).unwrap_or_default()
        } else {
            command
        };
        match joined {
            Some(last) => last.text.push_str(command),
            None => out.push(Logical {
                prefix: prefix.to_owned(),
                text: command.to_owned(),
                continued: false,
            }),
        }
    }
    for line in &mut out {
        line.text = line.text.replace("{{{{", "{{");
    }
    Ok(out)
}

/// Why a line body cannot be merged into one script mechanically; the
/// `&'static str` is the refused operation the engine prints
/// (`languages/api:V37`).
pub(crate) type Refusal = &'static str;

/// The lines merged into the script an extract holds
/// (`languages/ci/just:V180`): one per line, a `-` line with
/// [`INFALLIBLE`] appended; and whether every line was `@`, which the
/// load then carries. A body [`written`] could not give back is refused.
pub(crate) fn merged(lines: &[Logical]) -> std::result::Result<(String, bool), Refusal> {
    let quiet = lines.iter().filter(|line| line.quiet()).count();
    if quiet != 0 && quiet != lines.len() {
        return Err("rewrite of a recipe whose lines are @ only in part");
    }
    let mut out = Vec::new();
    for line in lines {
        if line.continued {
            return Err("rewrite of a line continued with a backslash");
        }
        if line.prefix == "-@" {
            return Err("rewrite of a -@ line (write it @-)");
        }
        if line.infallible() {
            if line.text.trim().is_empty() || line.text.contains('#') {
                return Err("rewrite of a - line that is empty or holds a #");
            }
            out.push(format!("{}{INFALLIBLE}", line.text));
        } else if line.text.trim_end().ends_with("|| true") {
            return Err("rewrite of a line ending in || true without -");
        } else {
            out.push(line.text.clone());
        }
    }
    Ok((out.join("\n"), quiet != 0))
}

/// The inverse of [`merged`]: `body`'s lines as recipe lines, each `@`
/// when `quiet`, a line ending in [`INFALLIBLE`] written `-` without it,
/// blank lines dropped. Every line after the first starts a new line
/// (`newline`) under `indent`; the first takes the place of the load
/// line's own text, so it carries no indent.
///
/// # Errors
///
/// [`Error::Parse`] for a line just would read differently: one starting
/// with `@` or `-`, or ending in `\`.
pub(crate) fn written(body: &str, indent: &str, quiet: bool, newline: &str) -> Result<String> {
    let mut out = Vec::new();
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (text, infallible) = match line.strip_suffix(INFALLIBLE) {
            Some(text) => (text, true),
            None => (line, false),
        };
        if text.starts_with(['@', '-']) {
            return Err(unwritable(
                text,
                "just reads its first character as a prefix",
            ));
        }
        if text.ends_with('\\') {
            return Err(unwritable(text, "just reads it as continued"));
        }
        let prefix = match (quiet, infallible) {
            (true, true) => "@-",
            (true, false) => "@",
            (false, true) => "-",
            (false, false) => "",
        };
        out.push(format!("{prefix}{}", text.replace("{{", "{{{{")));
    }
    if out.is_empty() {
        return Err(unwritable(body, "it has no line to run"));
    }
    Ok(out.join(&format!("{newline}{indent}")))
}
