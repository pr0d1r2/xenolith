//! What a host FINDS: the site, its delimiter, and the guest environment
//! in force there.
//!
//! A site is a delimiter AND a sink context (`languages/api/src/site:V38`).
//! The same `''…''` under a nix `description` attribute is inert data, not
//! a site -- which is why none of this is discoverable by scanning for
//! quotes, and every span here comes from a grammar node.

use crate::{LangId, Span};

#[cfg(test)]
mod tests;

/// One place in a host file that holds guest code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Site {
    /// The host-syntax context that makes this a site rather than a
    /// string: a nix attribute path, a hk step name, a GitHub `run` key.
    /// Named in the host's own vocabulary, because that is what the
    /// report has to point a reader back to.
    pub sink: String,
    /// The language of the enclosed code.
    pub guest: LangId,
    /// Interpreter dialect and options the host establishes here.
    pub env: GuestEnv,
    /// The delimiter bounding the guest code.
    pub delim: Delim,
    /// Host interpolations INSIDE the body: nix `${…}`, pkl `\(…)`, yaml
    /// `${{ }}`, just `{{…}}`. They are the reason extraction is not
    /// always mechanical -- an extracted file cannot carry host syntax, so
    /// each hole becomes a named parameter or the extraction becomes a
    /// judgement call (`languages/api/src/holes:V40`).
    pub holes: Vec<Span>,
}

/// Where a host would put the extract of a site: layer D of extract
/// resolution, the lowest precedence (`src/extract:V45`).
///
/// Both fields are `src/extract:V46` templates the engine renders,
/// because a [`Site`] does not carry its host's path: nix places beside
/// the host file (`{host_dir}/{host_stem}`), pkl in a fixed `scripts/hk`,
/// and only the engine knows which file it is scanning.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Placement {
    /// The extract's name without extension, deterministic and semantic
    /// (`languages/api/src/site:V43`): an attribute path tail, a step
    /// name, a job id -- never a line number or a hash.
    pub name: String,
    /// The directory, repo-root relative once rendered
    /// (`languages/api/src/lens:V66`).
    pub dir: String,
}

/// The braces around guest code, taken from a grammar node.
///
/// Never from brace counting over raw bytes
/// (`languages/api/src/site:V38`): escapes, nesting, heredoc terminators
/// and indent rules are exactly what a parser already resolved, and
/// re-deriving them from text is how a scanner ends up wrong about the one
/// file that matters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delim {
    /// Which delimiter form this is.
    pub kind: DelimKind,
    /// The opening delimiter itself.
    pub open: Span,
    /// The guest code between the delimiters, still in host escaping.
    pub body: Span,
    /// The closing delimiter.
    pub close: Span,
}

/// The delimiter forms xenolith understands, across every host.
///
/// One enum rather than a per-host type, because the engines report the
/// delimiter kind in a violation (`src:V1`) and a fixture exists per kind
/// (`languages/api/src/lens:V39`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelimKind {
    /// Nix indented string, `''…''`.
    NixIndented,
    /// Nix double-quoted string, `"…"`.
    NixString,
    /// Shell heredoc, `<<TAG`, `<<-TAG` or `<<~TAG`.
    Heredoc {
        /// The terminator word.
        tag: String,
        /// Whether the tag was quoted, which stops interpolation and so
        /// decides whether the body can contain holes at all.
        quoted: bool,
        /// Whether leading tabs or indent are stripped (`<<-`, `<<~`).
        strip_indent: bool,
    },
    /// Pkl multi-line string, `"""…"""`, optionally with `#` guards.
    PklMultiline {
        /// Number of `#` characters guarding the quotes, zero for plain
        /// `"""`.
        pounds: usize,
    },
    /// YAML block scalar, `|` or `>`, with its chomping indicator.
    YamlBlock {
        /// `true` for `|` (literal), `false` for `>` (folded).
        literal: bool,
        /// The chomping indicator as written: `-`, `+` or none.
        chomp: Option<char>,
    },
    /// An HTML `<script>` or `<style>` element body.
    HtmlElement {
        /// The tag name, lowercased.
        tag: String,
    },
    /// Rust raw string, `r"…"` or `r#"…"#`.
    RustRawString {
        /// Number of `#` characters, zero for `r"…"`.
        pounds: usize,
    },
    /// Ruby heredoc, `<<~TAG` and friends.
    RubyHeredoc {
        /// The terminator word.
        tag: String,
        /// Whether indentation is squiggly-stripped (`<<~`).
        squiggly: bool,
    },
    /// A command string passed as one argv element: `bash -c '…'`,
    /// `perl -e '…'`.
    ArgvString,
    /// A just recipe body, bounded by indentation.
    JustRecipe,
    /// A just recipe whose first line is a shebang, which makes the whole
    /// body one script rather than a line-per-command sequence.
    JustShebangRecipe,
}

impl DelimKind {
    /// Whether the host runs each line of such a body as a program of its
    /// own: a just recipe without a shebang hands every line to a fresh
    /// shell (`languages/ci/just:V180`). The engine may then judge the
    /// lines one at a time, up to the host's `[threshold.<host>]
    /// max_lines` (`src/config:V240`). Every other kind is one program.
    #[must_use]
    pub const fn runs_line_by_line(&self) -> bool {
        matches!(self, DelimKind::JustRecipe)
    }
}

/// The interpreter dialect and options in force at a site.
///
/// Derived by the HOST from its own context -- a nix `runtimeInputs` list,
/// a `#!/bin/sh` shebang on a just recipe, `set -euo pipefail` already
/// present in the body -- and consumed by the guest, which is what lets
/// `shellcheck -s sh` be chosen over `-s bash` without either side knowing
/// the other's syntax.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GuestEnv {
    /// The dialect, in the guest's own naming: `sh`, `bash`, `zsh`.
    /// `None` means the host established nothing and the guest's default
    /// applies.
    pub dialect: Option<String>,
    /// Effective options at the site: `errexit`, `nounset`, `pipefail`.
    pub options: Vec<String>,
}
