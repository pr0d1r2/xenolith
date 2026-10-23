//! What a justfile says, as far as its sites need it: its recipes and the
//! settings that decide how their bodies run.
//!
//! Plain data with byte spans into the source, built by [`crate::parse`]
//! (`languages:V2`, `languages/api/src/site:V38`): nothing here reads
//! text the parser did not bound.

use xenolith_lang_api::Span;

#[cfg(test)]
mod tests;

/// A parsed justfile.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Justfile {
    /// Every recipe, in source order.
    pub recipes: Vec<Recipe>,
    /// The settings that decide how a recipe line runs.
    pub settings: Settings,
}

/// One recipe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Recipe {
    /// The recipe's name, as written in its header.
    pub name: String,
    /// The header: `@name params: deps`, without a trailing comment.
    pub header: Span,
    /// The `#!` line opening the body, if any.
    pub shebang: Option<Line>,
    /// The body's lines after any `#!` line, in order; blank lines are
    /// not lines.
    pub lines: Vec<Line>,
    /// Whether the recipe takes parameters.
    pub params: bool,
    /// Whether a `[script]` attribute makes the body one script.
    pub script: bool,
    /// Whether a `[positional-arguments]` attribute passes its args to
    /// the shell.
    pub positional: bool,
}

/// One body line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Line {
    /// The line's text: after the body's indent, before the line break
    /// (a CRLF's `\r` is the host's line break, not the command's).
    pub span: Span,
    /// The `@`/`-` prefix at the start of the text; empty without one.
    pub prefix: Span,
    /// The `{{...}}` interpolations just evaluates, in order.
    pub holes: Vec<Span>,
}

impl Recipe {
    /// The body's bytes: from the start of the first line's LINE, its
    /// indent included, to the end of the last line. `None` for a recipe
    /// with no body.
    pub(crate) fn body(&self, src: &str) -> Option<Span> {
        let first = self.shebang.as_ref().or(self.lines.first())?;
        let last = self.lines.last().or(self.shebang.as_ref())?;
        let start = src
            .get(..first.span.start)
            .and_then(|before| before.rfind('\n'))
            .map_or(0, |newline| newline + 1);
        Some(Span::new(start, last.span.end))
    }
}

/// The settings that decide how a recipe line runs
/// (`languages/ci/just:V179`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Settings {
    /// `set shell := [...]`: each element as written, quotes included;
    /// `None` when the file has no such setting.
    pub shell: Option<Vec<String>>,
    /// `set windows-shell` or `set windows-powershell` is present.
    pub windows_shell: bool,
    /// `set positional-arguments`, unless set to `false`.
    pub positional: bool,
    /// The file imports another, whose settings it cannot see.
    pub imports: bool,
}
