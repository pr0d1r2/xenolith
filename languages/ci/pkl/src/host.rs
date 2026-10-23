//! Pkl as a host: hk steps holding shell (`languages/ci/pkl:T13`).
//!
//! A site is a delimiter AND a sink context (`languages/api/src/site:V38`),
//! and in pkl the context has three layers, all read from the object tree
//! [`crate::syntax`] builds:
//!
//! 1. the module `amends` hk's `Config.pkl` -- it is a hk config, so the
//!    property names below mean what hk says they mean;
//! 2. the property sits directly in a step ENTRY, `["name"] { ... }`,
//!    whose key is a plain string -- that name is the step, and the
//!    report points back to it;
//! 3. the property is one of hk's command properties, [`SINKS`]
//!    (`languages/ci/pkl` §I).
//!
//! Drop any layer and a `"""..."""` is inert data (`languages:V2`): the
//! same shell in a `message`, in a top-level `local`, in a module that is
//! not a hk config, or in a comment -- which the tree never holds as a
//! string at all.
//!
//! The delimiter is the pkl multi-line string, `"""..."""` or `#"""..."""#`
//! (`languages/api/src/site` §I). A single-line `"..."` has no delimiter
//! kind in the api, so it is never a site here; it is where a LOAD lives,
//! `sh ./scripts/hk/<name>.sh {{files}}` -- run by the interpreter the
//! extract's prelude names, `sh` under hk's default step shell
//! (`languages/ci/pkl:V52`, `languages/ci/pkl:V172`).

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, Host, Invoke, LangId, LintCmd, LoadRef, Placement, Result, Site, Span,
};

use crate::syntax::{self, Body, Member, Module, Str, Value};
use crate::{placement, shell, string};

#[cfg(test)]
mod tests;

/// hk step properties that hold a command (`languages/ci/pkl` §I), in name
/// order.
pub const SINKS: &[&str] = &["check", "check_diff", "check_list_files", "fix", "shell"];

/// hk's template for the files a step runs on. A load forwards it, so an
/// extracted script receives the same file list the inline body did
/// (`languages/ci/pkl:V52`).
const FILES: &str = "{{files}}";

/// Interpreters a load may name, and the extensions their scripts carry.
/// Recognised, never emitted: the argv of a load is the guest's
/// `invoke` (`languages/api:V35`), and this list only lets `loads` read
/// one back.
const INTERPRETERS: &[&str] = &["bash", "sh", "zsh"];
const SCRIPT_EXTENSIONS: &[&str] = &["bash", "sh", "zsh"];

/// The pkl host.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PklHost;

/// One hk command property: which step, which property, and its value.
pub(crate) struct Sink<'m> {
    pub(crate) step: String,
    pub(crate) property: String,
    pub(crate) value: &'m Value,
    /// The step's body, where hk's shell for it is read, and the body of
    /// the `Group` or hook whose `steps` hold it (`languages/ci/pkl:V172`).
    pub(crate) body: &'m Body,
    pub(crate) group: Option<&'m Body>,
}

/// Layer 1: whether the module amends hk's `Config.pkl` -- a package URI
/// (`.../hk@1.2.0#/Config.pkl`) or a vendored copy (`pkl/Config.pkl`).
fn is_hk_config(module: &Module) -> bool {
    module
        .amends
        .as_deref()
        .is_some_and(|uri| uri.rsplit(['/', '#', ':']).next() == Some("Config.pkl"))
}

/// Layers 2 and 3: every hk command property in the module, in source
/// order -- a non-`local` property named in [`SINKS`] directly in the body
/// of a `["name"] { ... }` entry. A `local check` inside a step is a
/// private helper, not the step's command.
pub(crate) fn sinks(module: &Module) -> Vec<Sink<'_>> {
    /// `body`, the step `step` when it is a step's body, inside the
    /// `group` whose `steps` hold it.
    fn walk<'m>(
        body: &'m Body,
        step: Option<&str>,
        group: Option<&'m Body>,
        out: &mut Vec<Sink<'m>>,
    ) {
        for member in &body.members {
            match member {
                Member::Property(property) => {
                    if let Some(step) = step
                        && !property.local
                        && SINKS.contains(&property.name.as_str())
                    {
                        out.push(Sink {
                            step: step.to_owned(),
                            property: property.name.clone(),
                            value: &property.value,
                            body,
                            group,
                        });
                    }
                    // A `steps` object's entries are steps of THIS body.
                    let holder = (property.name == "steps").then_some(body);
                    match &property.value {
                        Value::Object(inner) | Value::New(inner) => walk(inner, None, holder, out),
                        Value::Other(bodies) => {
                            for inner in bodies {
                                walk(inner, None, None, out);
                            }
                        }
                        Value::Str(_) => {}
                    }
                }
                Member::Entry(entry) => {
                    for inner in &entry.inside_key {
                        walk(inner, None, None, out);
                    }
                    match &entry.value {
                        Value::Object(inner) => walk(inner, entry.key.as_deref(), group, out),
                        Value::New(inner) => walk(inner, None, None, out),
                        Value::Other(bodies) => {
                            for inner in bodies {
                                walk(inner, None, None, out);
                            }
                        }
                        Value::Str(_) => {}
                    }
                }
                Member::Other(bodies) => {
                    for inner in bodies {
                        walk(inner, None, None, out);
                    }
                }
            }
        }
    }

    let mut out = Vec::new();
    if is_hk_config(module) {
        walk(&module.body, None, None, &mut out);
    }
    out
}

/// The string a sink's value is, alone.
fn literal<'m>(sink: &Sink<'m>) -> Option<&'m Str> {
    match sink.value {
        Value::Str(literal) => Some(literal),
        _ => None,
    }
}

/// The site a multi-line sink value makes, or `None` for any other value.
fn site(sink: &Sink<'_>) -> Option<Site> {
    let literal = literal(sink).filter(|l| l.multiline)?;
    Some(Site {
        sink: format!("{}.{}", sink.step, sink.property),
        guest: LangId::Shell,
        // The shell hk runs this step under, never the guest's default,
        // whose strict line the inline step did not have
        // (`languages/ci/pkl:V172`).
        env: shell::env(sink.body, sink.group),
        delim: Delim {
            kind: DelimKind::PklMultiline {
                pounds: literal.pounds,
            },
            open: literal.open,
            body: literal.body(),
            close: literal.close,
        },
        holes: literal.holes.clone(),
    })
}

/// The load a single-line sink value holds: `<interpreter> <script>`,
/// optionally followed by `{{files}}`, and nothing else.
fn load(sink: &Sink<'_>) -> Option<LoadRef> {
    let literal = literal(sink)?;
    let command = literal.plain.as_deref()?;
    let words: Vec<&str> = command.split_whitespace().collect();
    let [interpreter, script, rest @ ..] = words.as_slice() else {
        return None;
    };
    if !INTERPRETERS.contains(interpreter) || !matches!(rest, [] | [FILES]) {
        return None;
    }
    let path = PathBuf::from(script);
    let extension = path.extension().and_then(|ext| ext.to_str())?;
    SCRIPT_EXTENSIONS.contains(&extension).then_some(LoadRef {
        span: literal.span,
        path,
        guest: LangId::Shell,
    })
}

/// Whether an argv word can go into a load as it is: no shell quoting
/// and no pkl escaping needed, so `loads` reads back exactly what
/// `rewrite` wrote.
fn plain_word(word: &str) -> bool {
    !word.is_empty()
        && word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "/._-+,:@=%".contains(ch))
}

/// The line break the host uses where `at` sits: the one ending its
/// line, else the one before it, else `\n` for a one-line file
/// (`languages/ci/pkl:B2`). The literal `inline` writes breaks its lines the
/// same way, so a CRLF config stays CRLF -- and evaluates the same, a
/// CRLF in a pkl string being one `\n` (`languages/ci/pkl:B1`).
fn line_break(src: &str, at: Span) -> &'static str {
    let crlf = |newline: usize| newline > 0 && src.as_bytes().get(newline - 1) == Some(&b'\r');
    let after = src.get(at.end..).and_then(|rest| rest.find('\n'));
    let before = src.get(..at.start).and_then(|head| head.rfind('\n'));
    match (after.map(|offset| at.end + offset), before) {
        (Some(newline), _) | (None, Some(newline)) if crlf(newline) => "\r\n",
        _ => "\n",
    }
}

/// `src` with `[at.start, at.end)` replaced by `with`.
fn splice(src: &str, at: Span, with: &str) -> Result<String> {
    let before = src.get(..at.start);
    let after = src.get(at.end..);
    let (Some(before), Some(after)) = (before, after) else {
        return Err(Error::parse(LangId::Pkl, "span outside the source"));
    };
    Ok(format!("{before}{with}{after}"))
}

impl Host for PklHost {
    fn id(&self) -> LangId {
        LangId::Pkl
    }

    /// `*.pkl`, and `PklProject`, which is pkl without the extension.
    /// No shebang: pkl files are evaluated, not executed.
    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "pkl")
            || path.file_name().is_some_and(|name| name == "PklProject")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let module = syntax::parse(src)?;
        Ok(sinks(&module).iter().filter_map(site).collect())
    }

    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        let module = syntax::parse(src)?;
        Ok(sinks(&module).iter().filter_map(load).collect())
    }

    /// The multi-line string becomes `"<invoke argv> {{files}}"`
    /// (`languages/ci/pkl:V52`): hk passes the step's files through, so the
    /// script sees what the inline body saw.
    fn rewrite(&self, src: &str, site: &Site, invoke: &Invoke, _path: &Path) -> Result<String> {
        if !self.sites(src)?.contains(site) {
            return Err(Error::parse(
                LangId::Pkl,
                format!("no site `{}` at bytes {:?}", site.sink, site.delim.open),
            ));
        }
        // A `\(...)` is pkl: in the script it would be text. Holes go through
        // `rewrite_bound`, and this host has no load that passes a param
        // yet, so the api's default refuses them (`languages/ci/pkl:V171`,
        // `languages/api/src/holes:V40`).
        if !site.holes.is_empty() {
            return Err(Error::unsupported(
                LangId::Pkl,
                "rewrite of a string with holes",
            ));
        }
        // A word needing quotes would need shell quoting inside pkl
        // escaping, and `loads` could no longer read the result back:
        // refusing is better than a load `xnl graph` cannot follow.
        if invoke.argv.is_empty() || !invoke.argv.iter().all(|word| plain_word(word)) {
            return Err(Error::unsupported(LangId::Pkl, "rewrite"));
        }
        let command = format!("\"{} {FILES}\"", invoke.argv.join(" "));
        splice(
            src,
            Span::new(site.delim.open.start, site.delim.close.end),
            &command,
        )
    }

    /// The load's string becomes a multi-line literal holding `body`,
    /// indented one step past the property's line, its lines broken the
    /// way the host's are (`languages/ci/pkl:B2`).
    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String> {
        if !self.loads(src)?.contains(load) {
            return Err(Error::parse(
                LangId::Pkl,
                format!("no load of {} at {:?}", load.path.display(), load.span),
            ));
        }
        let line_start = src
            .get(..load.span.start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        let line = src.get(line_start..load.span.start).unwrap_or_default();
        let indent: String = line
            .chars()
            .take_while(|&ch| ch == ' ' || ch == '\t')
            .collect();
        let literal = string::multiline(body, &format!("{indent}  "));
        splice(
            src,
            load.span,
            &literal.replace('\n', line_break(src, load.span)),
        )
    }

    /// [`string::unescape`]: the closing line's indent stripped and the
    /// escapes decoded, as pkl evaluates the literal
    /// (`languages/api/src/lens:V39`).
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        string::unescape(delim, raw)
    }

    /// [`string::escape`]: the inverse, under the delimiter's own `#`
    /// count (`languages/ci/pkl:V171`).
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        string::escape(delim, body)
    }

    /// None yet, as a statement rather than a gap: the api lists
    /// `pkl format --diff` for pkl hosts with a `?` (`languages/api` §I),
    /// and a check this crate is unsure of would be a finding nobody can
    /// trust.
    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    /// None yet, for the same reason as [`PklHost::checks`].
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }

    /// `scripts/hk/<step>`, the step key kebab-cased
    /// (`languages/ci/pkl:V52`); `rewrite` then forwards `{{files}}` to it.
    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(placement::placement(site))
    }
}
