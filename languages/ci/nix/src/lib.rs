//! Nix support for xenolith.
//!
//! Nix is where this tool's subject is densest: NixOS modules, dev shells
//! and derivations all hand bash a string, and a multi-line `''…''` of
//! shell is the most common embed in a flake-shaped repo. This crate is
//! the HOST side of that -- which strings are sinks, where their
//! delimiters sit, what bash options nix wraps them in
//! (`languages/ci/nix:T12`). The shell itself belongs to the shell crate,
//! named here only as [`LangId::Shell`] (`languages:C24`).
//!
//! Parsing is `rnix` (`languages:C4`): a lossless tree, so delimiter and
//! hole spans are grammar ranges rather than quote counting
//! (`languages/api/src/site:V38`), and `''${` -- nix's escape for a
//! literal `${` -- is a string token, never a hole.

#![forbid(unsafe_code)]

mod bound;
mod escape;
mod loads;
mod placement;
mod rewrite;
mod scope;
mod sinks;
mod unescape;

#[cfg(test)]
mod tests;

use std::path::Path;

use rnix::{Root, SyntaxKind, SyntaxNode, TextRange};
use xenolith_lang_api::holes::Param;
use xenolith_lang_api::lens::Rewrite;
use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Placement,
    Result, Site, Span,
};

/// Nix as a host.
///
/// A unit struct: everything it answers comes from the source it is
/// handed (`languages/api:V36`), so the registry can hold one
/// `&'static dyn Host` for nix (`src/registry:V41`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct NixHost;

impl Host for NixHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }

    /// `*.nix`, and nothing else.
    ///
    /// The head is not consulted: a `#!/usr/bin/env nix-shell` file is a
    /// SHELL script with a nix interpreter line, and claiming it here
    /// would parse bash as nix (`languages:V56`).
    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "nix")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let root = parse(src)?;
        let mut found: Vec<Site> = root
            .descendants()
            .filter(|node| node.kind() == SyntaxKind::NODE_STRING)
            .filter_map(|string| {
                let sink = sinks::classify(&string)?;
                site(&string, sink)
            })
            .collect();
        // Preorder already visits strings by start offset, and a nested
        // string (inside a hole) starts after its parent's opening quote.
        // Sorting anyway states the contract (`languages/api:V36`) where
        // it is kept, rather than leaning on a traversal detail.
        found.sort_by_key(|site| site.delim.open);
        Ok(found)
    }

    /// `builtins.readFile ./x.sh` and
    /// `nix-shebang.lib.readWithoutStrict ./x.sh`, anywhere in the file
    /// (`languages/ci/nix:V53`), spanning the `replaceStrings` call around
    /// either when it is one `rewrite_bound` writes (`languages/ci/nix:V174`);
    /// a broken file is refused as in [`NixHost::sites`].
    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        Ok(loads::loads(&parse(src)?))
    }

    /// The string becomes `nix-shebang.lib.readWithoutStrict ./<path>`
    /// where nix-shebang is provably in scope, `builtins.readFile
    /// ./<path>` elsewhere (`languages/ci/nix:V53`, `languages/ci/nix:V170`),
    /// parenthesised where an argument goes; what cannot round-trip is
    /// refused by name (`rewrite`).
    ///
    /// `invoke` is not read: nix runs the text the load reads, in the
    /// sink that ran the string, so the guest's command line has nowhere
    /// to go (`languages/api:V35` -- the load IS nix's wrapping).
    fn rewrite(&self, src: &str, site: &Site, _invoke: &Invoke, path: &Path) -> Result<String> {
        rewrite::rewrite(src, site, path)
    }

    /// With params, the V53 load goes inside `builtins.replaceStrings`,
    /// which puts each hole back where the extract holds its `__NAME__`
    /// (`languages/ci/nix:V174`, `bound`); the body is nix's own, and bind's
    /// `body` is not read. Without, it is [`NixHost::rewrite`].
    fn rewrite_bound(
        &self,
        src: &str,
        site: &Site,
        _invoke: &Invoke,
        path: &Path,
        body: &str,
        params: &[Param],
    ) -> Result<Rewrite> {
        rewrite::rewrite_bound(src, site, path, body, params)
    }

    /// Either load becomes a string holding `body`: `''…''` indented under
    /// the load's line when the body has a line break, `"…"` when not; a
    /// `replaceStrings` load's holes go back where its patterns are.
    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String> {
        rewrite::inline(src, load, body)
    }

    /// What bash runs: a `''` body dedented and its `''` escapes decoded,
    /// a `"` body's backslash escapes decoded (`languages/ci/nix:T158`).
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        unescape::unescape(&delim.kind, raw)
    }

    /// The inverse of [`NixHost::unescape`]: `''` pairs, `${` and the
    /// indent-sensitive spaces escaped in a `''` body, `\`, `"`, `${` and
    /// line breaks in a `"` one (`languages/ci/nix:V170`, `escape`).
    fn escape(&self, delim: &Delim, body: &str) -> Result<String> {
        escape::escape(&delim.kind, body)
    }

    /// `statix`, `deadnix`, `nixfmt --check` (`languages/api` §I).
    ///
    /// JSON where the tool has it, so a finding carries a line, a column
    /// and a code rather than prose that changes between releases
    /// (`src/lint:V92`). nixfmt has no report format: it either would
    /// change the file or would not, and its exit code says which.
    fn checks(&self) -> Vec<LintCmd> {
        vec![
            cmd(
                &["statix", "check", "--format", "json"],
                Format::Json("statix"),
            ),
            cmd(
                &["deadnix", "--output-format", "json"],
                Format::Json("deadnix"),
            ),
            cmd(&["nixfmt", "--check"], Format::Raw),
        ]
    }

    /// The same three tools, in their rewriting modes.
    fn fixers(&self) -> Vec<LintCmd> {
        vec![
            cmd(&["statix", "fix"], Format::Raw),
            cmd(&["deadnix", "--edit"], Format::Raw),
            cmd(&["nixfmt"], Format::Raw),
        ]
    }

    /// Whether the string at `site` is a site only by its shebang
    /// (`languages/ci/nix:T157`): the string starting at the site's opening
    /// quote, classified again. A named sink whose body happens to start
    /// with `#!` is the sink's, and says no.
    fn guest_by_shebang(&self, src: &str, site: &Site) -> bool {
        let Ok(root) = parse(src) else {
            return false;
        };
        let at = site.delim.open.start;
        root.descendants()
            .filter(|node| node.kind() == SyntaxKind::NODE_STRING)
            .find(|string| usize::from(string.text_range().start()) == at)
            .and_then(|string| sinks::classify(&string))
            .is_some_and(|sink| matches!(sink, sinks::Sink::Shebang { .. }))
    }

    /// Named after the attribute path, beside the host file
    /// (`languages/ci/nix:V53`, `placement::name`).
    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(placement::placement(site))
    }

    /// `replaceVars`, then argv, then env (`languages/ci/nix:V54`,
    /// `placement::hole_advice`).
    fn hole_advice(&self, _site: &Site) -> Result<Vec<String>> {
        Ok(placement::hole_advice())
    }
}

/// A command taking its file as the last argument.
fn cmd(argv: &[&str], format: Format) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format,
    }
}

/// The tree for `src`, or the parser's first complaint.
///
/// ANY error fails the whole file. rnix recovers and keeps building a
/// tree, but spans inside and after an error node are not trustworthy
/// enough to cut a file on (`languages:V78`); reporting only the sites
/// outside error regions is `languages:T78`, and until then a broken file
/// is reported as broken rather than as clean.
fn parse(src: &str) -> Result<SyntaxNode> {
    let parsed = Root::parse(src);
    if let Some(first) = parsed.errors().first() {
        return Err(Error::parse(LangId::Nix, first.to_string()));
    }
    Ok(parsed.syntax())
}

/// The [`Site`] for a string node already known to be in `sink`.
///
/// `None` only for a string without both quote tokens, which the parse
/// check above has already ruled out -- kept as `None` rather than a
/// panic, so a grammar surprise costs one site and not the scan.
fn site(string: &SyntaxNode, sink: sinks::Sink) -> Option<Site> {
    let (delim, holes) = delim(string)?;
    Some(Site {
        sink: sinks::sink_path(string),
        guest: sink.guest(),
        env: sink.env(),
        delim,
        holes,
    })
}

/// The delimiter of a string node and the spans of its holes, or `None`
/// for a string without both quote tokens -- what a [`Site`] is made of
/// before its sink says what it is, and what `inline` reads a string it
/// wrote back with (`languages/ci/nix:V174`).
fn delim(string: &SyntaxNode) -> Option<(Delim, Vec<Span>)> {
    let tokens = || {
        string
            .children_with_tokens()
            .filter_map(rnix::NodeOrToken::into_token)
    };
    let open = tokens().find(|t| t.kind() == SyntaxKind::TOKEN_STRING_START)?;
    let close = tokens()
        .filter(|t| t.kind() == SyntaxKind::TOKEN_STRING_END)
        .last()?;
    let kind = if open.text() == "''" {
        DelimKind::NixIndented
    } else {
        DelimKind::NixString
    };
    let open = span(open.text_range());
    let close = span(close.text_range());
    let holes = string
        .children()
        .filter(|c| c.kind() == SyntaxKind::NODE_INTERPOL)
        .map(|c| span(c.text_range()))
        .collect();
    let delim = Delim {
        kind,
        open,
        body: Span::new(open.end, close.start),
        close,
    };
    Some((delim, holes))
}

/// A rowan range as the api's byte span.
fn span(range: TextRange) -> Span {
    Span::new(range.start().into(), range.end().into())
}
