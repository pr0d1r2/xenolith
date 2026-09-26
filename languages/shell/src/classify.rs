//! Is this sink one simple command, or a script?
//!
//! `languages/shell:V3` draws the line: an argv, optionally preceded by
//! `NAME=value` assignments, with one whitelisted command substitution --
//! and nothing else. A pipe, a list, a redirect, a loop, a heredoc or a
//! function makes it a script, and a script belongs in a file.
//!
//! From the AST, not from substrings, and the reason is one example:
//! `echo "a | b"` holds a pipe character and is a single command, while
//! `a|b` is two. A substring rule either flags every quoted pipe or
//! misses every real one, and the grammar already knows the difference.

use tree_sitter::{Node, Parser};
use xenolith_lang_api::{Error, LangId, Result};

#[cfg(test)]
mod tests;

/// A shell construct that makes a body more than one simple command.
///
/// The names are the strings `[threshold.shell] allow` matches
/// (`src/config` §I), so they are a contract: renaming one turns every
/// allow entry into a silent no-op. Declaration order is name order, so
/// a sorted list of constructs is sorted by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Construct {
    /// `a && b`, `a || b`.
    AndOr,
    /// `case … esac`.
    Case,
    /// `$(…)` or a backtick pair, other than the whitelisted
    /// `$(dirname "${BASH_SOURCE[0]}")`.
    CommandSubstitution,
    /// `for … done`, in either syntax.
    For,
    /// `f() { … }`.
    FunctionDefinition,
    /// `<<TAG`, in any of its forms.
    Heredoc,
    /// `if … fi`.
    If,
    /// `a | b`.
    Pipeline,
    /// Any redirection: `>`, `<`, `2>&1`, `<<<`.
    Redirect,
    /// Two or more commands, separated by `;` or a newline.
    Sequence,
    /// `( … )`.
    Subshell,
    /// `while … done`, and `until`.
    While,
}

impl Construct {
    /// Every construct, in name order.
    pub const ALL: &'static [Construct] = &[
        Construct::AndOr,
        Construct::Case,
        Construct::CommandSubstitution,
        Construct::For,
        Construct::FunctionDefinition,
        Construct::Heredoc,
        Construct::If,
        Construct::Pipeline,
        Construct::Redirect,
        Construct::Sequence,
        Construct::Subshell,
        Construct::While,
    ];

    /// The config-facing name.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Construct::AndOr => "and-or",
            Construct::Case => "case",
            Construct::CommandSubstitution => "command-substitution",
            Construct::For => "for",
            Construct::FunctionDefinition => "function-definition",
            Construct::Heredoc => "heredoc",
            Construct::If => "if",
            Construct::Pipeline => "pipeline",
            Construct::Redirect => "redirect",
            Construct::Sequence => "sequence",
            Construct::Subshell => "subshell",
            Construct::While => "while",
        }
    }

    /// The inverse, exact-match only, so a misspelled allow entry is an
    /// error rather than a rule that matches nothing.
    #[must_use]
    pub fn from_name(name: &str) -> Option<Construct> {
        Construct::ALL
            .iter()
            .copied()
            .find(|construct| construct.as_str() == name)
    }
}

/// What a body turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    /// Whether this is one simple command, and so may stay inline.
    pub simple: bool,
    /// Every construct found, sorted by name and deduplicated: a body
    /// with three pipes reports `pipeline` once, and the same body always
    /// reports the same list (`src:V11`).
    pub constructs: Vec<Construct>,
}

/// The one command substitution `languages/shell:V3` permits.
///
/// It is what this tool's OWN generated load looks like: a script next to
/// the host file, addressed relative to the host rather than to the
/// caller's working directory. Without the exemption `xnl extract` would
/// emit a load that `xnl check` then flags, which is a tool arguing with
/// itself. Matched EXACTLY, so `$(anything-else)` is still a construct.
const ALLOWED_SUBSTITUTION: &str = "$(dirname \"${BASH_SOURCE[0]}\")";

/// Classify a shell body.
///
/// # Errors
///
/// [`Error::Parse`] when the grammar cannot parse `body`, or when the
/// parser itself cannot be built. An unparseable body is its own finding
/// (`languages:V77`): calling it simple would leave broken shell inline,
/// and naming a construct in it would name something nobody wrote.
pub fn classify(body: &str) -> Result<Classification> {
    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_bash::LANGUAGE.into())
        .map_err(|e| Error::parse(LangId::Shell, format!("cannot load the bash grammar: {e}")))?;
    let tree = parser
        .parse(body, None)
        .ok_or_else(|| Error::parse(LangId::Shell, "the bash parser returned no tree"))?;

    let root = tree.root_node();
    if root.has_error() {
        return Err(Error::parse(
            LangId::Shell,
            "this body is not valid shell, so it was neither classified nor extracted",
        ));
    }

    let mut constructs = Vec::new();
    // Two or more statements at the TOP level are a sequence. Counted
    // here rather than in the walk, because statements nested inside an
    // `if` are not a sequence of their own -- the `if` is the finding,
    // and reporting both would be one mistake read as two.
    if top_level_statements(root) > 1 {
        constructs.push(Construct::Sequence);
    }
    collect(root, body, &mut constructs);

    constructs.sort_unstable();
    constructs.dedup();

    Ok(Classification {
        simple: constructs.is_empty(),
        constructs,
    })
}

/// Named children of the program node, ignoring comments.
fn top_level_statements(root: Node<'_>) -> usize {
    let mut cursor = root.walk();
    root.named_children(&mut cursor)
        .filter(|child| child.kind() != "comment")
        .count()
}

/// Walk the tree, recording every construct.
fn collect(node: Node<'_>, src: &str, found: &mut Vec<Construct>) {
    if let Some(construct) = construct_of(node, src) {
        found.push(construct);
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        collect(child, src, found);
    }
}

/// The construct a single node represents, if any.
fn construct_of(node: Node<'_>, src: &str) -> Option<Construct> {
    match node.kind() {
        "pipeline" => Some(Construct::Pipeline),
        // The grammar calls `a && b` a list, which is the one node name
        // that does not read like what it matches.
        "list" => Some(Construct::AndOr),
        "subshell" => Some(Construct::Subshell),
        "if_statement" => Some(Construct::If),
        "for_statement" | "c_style_for_statement" => Some(Construct::For),
        "while_statement" => Some(Construct::While),
        "case_statement" => Some(Construct::Case),
        "function_definition" => Some(Construct::FunctionDefinition),
        "heredoc_redirect" => Some(Construct::Heredoc),
        // `<<<` is a redirection like `<` is, but the grammar gives it a
        // node of its own; without this row `cat <<< x` read as simple
        // (`languages/shell:B2`).
        "file_redirect" | "herestring_redirect" => Some(Construct::Redirect),
        // Process substitution is a subshell plus a redirect wearing one
        // piece of syntax. Reported as a substitution because that is the
        // part a reader sees.
        "command_substitution" | "process_substitution" => {
            if node
                .utf8_text(src.as_bytes())
                .is_ok_and(|text| text == ALLOWED_SUBSTITUTION)
            {
                None
            } else {
                Some(Construct::CommandSubstitution)
            }
        }
        _ => None,
    }
}
