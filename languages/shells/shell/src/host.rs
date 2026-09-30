//! Shell as a host: the files it claims (`languages/shells/shell:T135`) and the
//! programs they hand other interpreters (`languages/shells/shell:T15`).
//!
//! A shell file encloses other languages the way a nix string encloses
//! bash -- a heredoc fed to `python`, a `-c` string handed to `bash`
//! (`languages/shells/shell` §I). Before any of that, the host has to say which
//! files are its to parse, and the answer has one exception that matters
//! more than the rule: a `*.bats` file is never shell's
//! (`languages/shells/shell:V137`).
//!
//! A site is a delimiter AND a sink context
//! (`languages/api/src/site:V38`): the same `'…'` is a site after
//! `python3 -c` and inert data after `echo`. Both halves come from the
//! tree -- the command node and its arguments, the heredoc node and its
//! terminator -- and which argv holds a program is `sinks`'s, as is the
//! dialect and options a shell site runs under (`languages/shells/shell:T83`).

use std::path::Path;

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site,
    Span, shebang, shebang::Shebang,
};

use crate::sinks::{self, Interpreter, Kind};

#[cfg(test)]
mod tests;

/// Shell as a host.
///
/// A unit struct, as [`crate::ShellGuest`] is: everything it answers
/// comes from the text it is handed (`languages/api:V36`), so the
/// registry can hold one `&'static dyn Host` for it (`src/registry:V41`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ShellHost;

/// Extensions that make a file shell whatever its first line says.
const EXTENSIONS: &[&str] = &["sh", "bash"];

/// Filenames that are shell with no extension at all.
const FILENAMES: &[&str] = &[".envrc"];

/// The extension of bats, the language based on shell that shell must
/// never claim (`languages:V130`).
const BATS: &str = "bats";

/// The one shell dialect whose files this host refuses
/// (`languages/shells/shell:V310`).
const ZSH: &str = "zsh";

impl Host for ShellHost {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    /// `*.sh`, `*.bash`, `.envrc`, or a shebang resolving to a shell
    /// dialect (`languages/shells/shell` §I) -- and never `*.bats`, which is
    /// checked FIRST so no shebang can talk the host into it, nor a file
    /// whose shebang runs zsh, whatever its extension.
    ///
    /// The bash grammar reads a `@test` block as a command and a brace
    /// group, so a claimed bats file would be offered for extraction as a
    /// script: confident and wrong (`languages/shells/shell:V137`). Refusing by
    /// extension is the only place that mistake can be stopped, because
    /// nothing downstream would ever see an error.
    ///
    /// zsh is refused for the opposite reason: everything downstream
    /// errors (`languages/shells/shell:V310`). The grammar rejects
    /// zsh-only syntax, and [`ShellHost::checks`] is not told which file
    /// it is checking, so shellcheck -- which refuses zsh outright -- would
    /// fail every one. Unclaimed, the file is still linted as an extract,
    /// in its zsh dialect (`src/lint` §I).
    fn claims(&self, path: &Path, head: &str) -> bool {
        if path.extension().is_some_and(|ext| ext == BATS) {
            return false;
        }
        let bang = shebang::parse(head);
        if bang.as_ref().is_some_and(is_zsh) {
            return false;
        }
        let by_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| EXTENSIONS.contains(&ext));
        let by_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| FILENAMES.contains(&name));
        by_extension
            || by_name
            || bang.is_some_and(|line| shebang::resolves_to(&line, LangId::Shell))
    }

    /// Every heredoc fed to an interpreter as its program, and every
    /// `-c`/`-e` argument, sorted by span (`languages/shells/shell:V139`).
    ///
    /// ANY parse error fails the whole file, as nix's host does: spans
    /// inside and after an `ERROR` node are the parser's guess, and a site
    /// built on a guess rewrites the wrong bytes (`languages:V78`).
    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let tree = parse(src)?;
        let root = tree.root_node();
        if root.has_error() && !bash_accepts(src) {
            return Err(Error::parse(
                LangId::Shell,
                "the file is not valid shell, so none of its sites were read",
            ));
        }
        let mut found = Vec::new();
        walk(root, src, &mut found);
        found.sort_by_key(|site| site.delim.open);
        Ok(found)
    }

    /// Not offered: the load idiom (`python scripts/x.py`, `jq -f x.jq`)
    /// arrives with extraction, and an empty list would read as "nothing
    /// loaded here" (`languages/api:V37`).
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Shell, "loads"))
    }

    /// Not offered yet; see [`ShellHost::loads`].
    fn rewrite(&self, _src: &str, _site: &Site, _invoke: &Invoke, _path: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "rewrite"))
    }

    /// Not offered yet; see [`ShellHost::loads`].
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Shell, "inline"))
    }

    /// The body as the interpreter receives it
    /// (`languages/api/src/lens:V39`).
    ///
    /// An argv string is verbatim: this host reports only the forms
    /// whose bytes ARE the argument -- single-quoted, or double-quoted
    /// with no backslash (`languages/shells/shell:V139`). A heredoc loses its
    /// leading tabs under `<<-`, and, when its tag is unquoted, the
    /// backslash before `$`, a backtick, a backslash or a newline.
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        match &delim.kind {
            DelimKind::ArgvString => Ok(raw.to_owned()),
            DelimKind::Heredoc {
                quoted,
                strip_indent,
                ..
            } => {
                let text = if *strip_indent {
                    strip_tabs(raw)
                } else {
                    raw.to_owned()
                };
                Ok(if *quoted { text } else { unbackslash(&text) })
            }
            _ => Err(Error::unsupported(LangId::Shell, "unescape")),
        }
    }

    /// shellcheck and shfmt, the default linters (`languages/shells/shell` §G).
    ///
    /// Neither is told a dialect. A host file names its own -- a
    /// shebang, a `# shellcheck shell=` directive, an extension -- and
    /// both tools read it; forcing `bash` would check every `#!/bin/sh`
    /// script against the wrong shell. Both read the sh family only,
    /// which is why [`ShellHost::claims`] never hands them a zsh file
    /// (`languages/shells/shell:V310`).
    fn checks(&self) -> Vec<LintCmd> {
        vec![
            LintCmd {
                argv: vec!["shellcheck".to_owned(), "--format=json".to_owned()],
                file_arg: FileArg::Append,
                format: Format::Json("shellcheck"),
            },
            raw(&["shfmt", "--diff"]),
        ]
    }

    /// shfmt in its rewriting mode. shellcheck fixes nothing itself.
    fn fixers(&self) -> Vec<LintCmd> {
        vec![raw(&["shfmt", "--write"])]
    }
}

/// tree-sitter-bash 0.25.1 rejects several valid bash parameter expansions
/// and read-write redirects. Bash remains the authority for whether the host
/// itself is syntactically valid; the tree is still used for site discovery.
fn bash_accepts(src: &str) -> bool {
    let mut command = std::process::Command::new("bash");
    command.arg("-n").stdin(std::process::Stdio::piped());
    let Ok(mut child) = command.spawn() else {
        return false;
    };
    let Some(mut stdin) = child.stdin.take() else {
        return false;
    };
    use std::io::Write;
    if stdin.write_all(src.as_bytes()).is_err() {
        return false;
    }
    drop(stdin);
    child.wait().map(|status| status.success()).unwrap_or(false)
}

/// Whether a shebang runs zsh: its resolved interpreter's basename, the
/// dialect `src/lint` §I gives the file as an extract, so what this host
/// refuses is exactly what the zsh guest checks.
fn is_zsh(line: &Shebang) -> bool {
    let interpreter = line.resolved_interpreter();
    interpreter.rsplit('/').next().unwrap_or(interpreter) == ZSH
}

/// A command whose output has no machine-readable form.
fn raw(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}

fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .map_err(|e| Error::parse(LangId::Shell, format!("cannot load the bash grammar: {e}")))?;
    parser
        .parse(src, None)
        .ok_or_else(|| Error::parse(LangId::Shell, "the bash parser returned no tree"))
}

fn text<'s>(node: Node<'_>, src: &'s str) -> &'s str {
    src.get(node.byte_range()).unwrap_or_default()
}

fn span(node: Node<'_>) -> Span {
    Span::new(node.start_byte(), node.end_byte())
}

/// A hole's span: its node, minus leading whitespace.
///
/// tree-sitter-bash starts a `$(`, backtick or `$((` node that follows
/// another expansion in a `"…"` at the SPACE before it (measured: in
/// `"${a} $(b)"` the substitution spans ` $(b)`). The space is the
/// guest's text, not the host's interpolation.
fn hole(node: Node<'_>, src: &str) -> Span {
    let whole = text(node, src);
    let lead = whole.len() - whole.trim_start().len();
    Span::new(node.start_byte() + lead, node.end_byte())
}

/// Every site at or under `node`.
fn walk(node: Node<'_>, src: &str, out: &mut Vec<Site>) {
    match node.kind() {
        "command" => out.extend(argv_site(node, src)),
        "redirected_statement" => out.extend(heredoc_site(node, src)),
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(child, src, out);
    }
}

/// The interpreter a command node runs, and its arguments: `Some(text)`
/// for a plain word, `None` for anything quoted or expanded, which can
/// never be a flag.
fn command<'t, 's>(node: Node<'t>, src: &'s str) -> Option<(Interpreter<'s>, Vec<Node<'t>>)> {
    let name = node.child_by_field_name("name")?;
    let word = name.named_child(0).filter(|word| word.kind() == "word")?;
    let interpreter = sinks::interpreter(text(word, src))?;
    let mut cursor = node.walk();
    let args = node
        .children_by_field_name("argument", &mut cursor)
        .collect();
    Some((interpreter, args))
}

fn words<'s>(args: &[Node<'_>], src: &'s str) -> Vec<Option<&'s str>> {
    args.iter()
        .map(|arg| (arg.kind() == "word").then(|| text(*arg, src)))
        .collect()
}

/// The `-c`/`-e` site of a command node, if it has one.
fn argv_site(node: Node<'_>, src: &str) -> Option<Site> {
    let (interpreter, args) = command(node, src)?;
    let words = words(&args, src);
    let at = sinks::program_arg(&interpreter, &words)?;
    let program = *args.get(at)?;
    // The sink is named by the flag as written (`perl -ne`), except for
    // a shell, whose `-c` may sit in any bundle before the program
    // (`bash -c -e '…'`, `sh -ec '…'`): `-c` is the one stable name.
    let flag = match interpreter.kind {
        Kind::Shell => "-c",
        _ => words.get(at.checked_sub(1)?).copied().flatten()?,
    };
    // Only a string whose bytes ARE the argument: `'…'`, or `"…"` with
    // no backslash (`languages/shells/shell:V139`). `ArgvString` does not say
    // which quote it came from, so `\"` could not be decoded for one
    // without corrupting the other.
    let holes = match program.kind() {
        "raw_string" => Vec::new(),
        "string" if !text(program, src).contains('\\') => {
            let mut cursor = program.walk();
            program
                .named_children(&mut cursor)
                .filter(|part| part.kind() != "string_content")
                .map(|part| hole(part, src))
                .collect()
        }
        _ => return None,
    };
    let whole = span(program);
    let open = Span::new(whole.start, whole.start + 1);
    let close = Span::new(whole.end.checked_sub(1)?, whole.end);
    Some(Site {
        sink: format!("{} {flag}", interpreter.name),
        guest: interpreter.guest,
        env: sinks::env(&interpreter, &words),
        delim: Delim {
            kind: DelimKind::ArgvString,
            open,
            body: Span::new(open.end, close.start),
            close,
        },
        holes,
    })
}

/// The heredoc site of a redirected command, when the heredoc is the
/// interpreter's program rather than its input.
fn heredoc_site(node: Node<'_>, src: &str) -> Option<Site> {
    let body = node.child_by_field_name("body")?;
    let (interpreter, args) = command(body, src)?;
    let words = words(&args, src);
    if !sinks::stdin_is_program(&interpreter, &words) {
        return None;
    }
    let mut cursor = node.walk();
    let redirects: Vec<Node<'_>> = node
        .children_by_field_name("redirect", &mut cursor)
        .collect();
    // One redirect, the heredoc: a second stdin (`< file`, `<<<`) would
    // leave it unclear which one the interpreter reads.
    let [heredoc] = redirects.as_slice() else {
        return None;
    };
    if heredoc.kind() != "heredoc_redirect" {
        return None;
    }
    let operator = heredoc.child(0)?;
    let start = heredoc
        .named_child(0)
        .filter(|n| n.kind() == "heredoc_start")?;
    let mut cursor = heredoc.walk();
    let children: Vec<Node<'_>> = heredoc.named_children(&mut cursor).collect();
    let end = children.iter().find(|n| n.kind() == "heredoc_end")?;
    // The grammar nests redirects written AFTER the tag inside the
    // heredoc node: `python3 <<PY <in` would read `in`, not the heredoc.
    let second_stdin = children.iter().any(|n| {
        n.kind() == "herestring_redirect"
            || (n.kind() == "file_redirect" && text(*n, src).starts_with('<'))
    });
    if second_stdin {
        return None;
    }
    let content = children.iter().find(|n| n.kind() == "heredoc_body");
    let tag_text = text(start, src);
    let quoted = tag_text.contains(['\'', '"', '\\']);
    let tag: String = tag_text
        .chars()
        .filter(|c| !matches!(c, '\'' | '"' | '\\'))
        .collect();
    let body_end = line_start(src, end.start_byte());
    let body_start = content.map_or(body_end, |content| line_start(src, content.start_byte()));
    let holes = match content {
        Some(content) if !quoted => {
            let mut cursor = content.walk();
            content
                .named_children(&mut cursor)
                .filter(|part| part.kind() != "heredoc_content")
                .map(|part| hole(part, src))
                .collect()
        }
        _ => Vec::new(),
    };
    Some(Site {
        sink: format!("{} <<{tag}", interpreter.name),
        guest: interpreter.guest,
        env: sinks::env(&interpreter, &words),
        delim: Delim {
            kind: DelimKind::Heredoc {
                tag,
                quoted,
                strip_indent: text(operator, src) == "<<-",
            },
            open: Span::new(operator.start_byte(), start.end_byte()),
            body: Span::new(body_start, body_end),
            close: span(*end),
        },
        holes,
    })
}

/// The start of the line holding byte `at`.
///
/// The grammar starts a `<<-` body AFTER the first line's leading tabs
/// and ends it before the terminator's; the delimiter's body is whole
/// lines, so the tabs are the host's to strip in [`Host::unescape`].
fn line_start(src: &str, at: usize) -> usize {
    src.get(..at)
        .and_then(|before| before.rfind('\n'))
        .map_or(0, |newline| newline + 1)
}

/// Every line with its leading tabs removed, as `<<-` does.
fn strip_tabs(raw: &str) -> String {
    raw.split_inclusive('\n')
        .map(|line| line.trim_start_matches('\t'))
        .collect()
}

/// An unquoted heredoc's escapes resolved: `\$`, `` \` `` and `\\` lose
/// their backslash, `\` + newline joins the lines, and every other
/// backslash is kept as written.
fn unbackslash(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '\\' {
            out.push(ch);
            continue;
        }
        match chars.peek() {
            Some('$' | '`' | '\\') => out.extend(chars.next()),
            Some('\n') => {
                chars.next();
            }
            _ => out.push(ch),
        }
    }
    out
}
