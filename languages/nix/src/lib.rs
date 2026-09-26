//! Nix support for xenolith.
//!
//! Nix is where this tool's subject is densest: NixOS modules, dev shells
//! and derivations all hand bash a string, and a multi-line `''…''` of
//! shell is the most common embed in a flake-shaped repo. This crate is
//! the HOST side of that -- which strings are sinks, where their
//! delimiters sit, what bash options nix wraps them in
//! (`languages/nix:T12`). The shell itself belongs to the shell crate,
//! named here only as [`LangId::Shell`] (`languages:C24`).
//!
//! Parsing is `rnix` (`languages:C4`): a lossless tree, so delimiter and
//! hole spans are grammar ranges rather than quote counting
//! (`languages/api/src/site:V38`), and `''${` -- nix's escape for a
//! literal `${` -- is a string token, never a hole.

#![forbid(unsafe_code)]

mod sinks;

#[cfg(test)]
mod tests;

use std::path::Path;

use rnix::{Root, SyntaxKind, SyntaxNode, TextRange};
use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Host, Invoke, LangId, LintCmd, LoadRef, Result, Site,
    Span,
};

/// Nix as a host.
///
/// A unit struct: everything it answers comes from the source it is
/// handed (`languages/api:V36`), so the registry can hold one
/// `&'static dyn Host` for nix (`src:V41`).
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

    /// Not offered yet: this crate reports sites (`languages/nix:T12`),
    /// and the load idiom it will recognise -- `builtins.readFile` or
    /// `nix-shebang.lib.readWithoutStrict` -- is `languages/nix:V53`'s.
    /// Refused loudly rather than answered with an empty list, which
    /// would read as "no loads here" (`languages/api:V37`).
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Nix, "loads"))
    }

    /// Not offered yet; see [`NixHost::loads`].
    fn rewrite(&self, _src: &str, _site: &Site, _invoke: &Invoke, _path: &Path) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "rewrite"))
    }

    /// Not offered yet; see [`NixHost::loads`].
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Err(Error::unsupported(LangId::Nix, "inline"))
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
    Some(Site {
        sink: sinks::sink_path(string),
        guest: LangId::Shell,
        env: sink.env(),
        delim: Delim {
            kind,
            open,
            body: Span::new(open.end, close.start),
            close,
        },
        holes,
    })
}

/// A rowan range as the api's byte span.
fn span(range: TextRange) -> Span {
    Span::new(range.start().into(), range.end().into())
}
