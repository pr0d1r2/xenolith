//! Python as a guest (`languages/data/python:V59`): what an extract of it
//! is called, how it runs, what goes above its body, and which bodies may
//! stay inline.
//!
//! Trivial is ONE expression statement and nothing else: `print(x)` in a
//! `python3 -c` argument stays where it is. An `import`, a `def` or
//! `class`, an assignment, a compound statement or a second statement of
//! any kind is more than that. Assignment is excluded although the grammar
//! files it under `expression_statement`: in Python it is a statement, and
//! a body that binds a name is building state, not evaluating once.
//!
//! The guest names no constructs (`languages/api:V37`), so a body that is
//! not trivial is judged by `[threshold.python] max_lines` / `max_bytes`.

use std::path::Path;

use tree_sitter::{Node, Parser, Tree};
use xenolith_lang_api::{
    Error, FileArg, Format, Guest, GuestEnv, Invoke, LangId, LintCmd, Prelude, Result, Shebang,
};

#[cfg(test)]
mod tests;

/// Python as a guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PythonGuest;

/// The interpreter an extract runs under.
const PYTHON: &str = "python3";

/// `src` parsed as python, error nodes and all.
///
/// Neither failure is reachable with the grammar this crate pins -- its ABI
/// is one the runtime loads, and a parse with no timeout or cancel flag
/// always returns a tree -- so both share one message rather than two
/// branches no test can reach.
fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    let tree = parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .ok()
        .and_then(|()| parser.parse(src, None));
    tree.ok_or(Error::Parse {
        lang: LangId::Python,
        message: "the python grammar did not load, or returned no tree".to_owned(),
    })
}

/// Whether `statement` is an expression evaluated for its effect, and not
/// an assignment the grammar happens to file as one.
fn is_bare_expression(statement: Node<'_>) -> bool {
    if statement.kind() != "expression_statement" {
        return false;
    }
    let mut cursor = statement.walk();
    let mut children = statement.named_children(&mut cursor);
    matches!(
        (children.next(), children.next()),
        (Some(only), None) if !matches!(only.kind(), "assignment" | "augmented_assignment")
    )
}

impl Guest for PythonGuest {
    fn id(&self) -> LangId {
        LangId::Python
    }

    /// Always `py`: python has one dialect here.
    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "py"
    }

    /// `python3 {path}`.
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec![PYTHON.to_owned(), path.display().to_string()],
        }
    }

    /// True for exactly one bare expression statement, comments aside
    /// (`languages/data/python:V59`); a body the grammar rejects is never
    /// trivial (`languages:V77`).
    fn trivial(&self, body: &str) -> Result<bool> {
        let tree = parse(body)?;
        let root = tree.root_node();
        if root.has_error() {
            return Err(Error::parse(
                LangId::Python,
                "the body is not valid python to this grammar",
            ));
        }
        let mut cursor = root.walk();
        let mut statements = root
            .named_children(&mut cursor)
            .filter(|node| node.kind() != "comment");
        Ok(matches!(
            (statements.next(), statements.next()),
            (Some(only), None) if is_bare_expression(only)
        ))
    }

    /// `#!/usr/bin/env python3`, and no strict line: an exception already
    /// stops a python script (`languages/data/python:V59`).
    fn prelude(&self, _env: &GuestEnv) -> Prelude {
        Prelude {
            shebang: Some(Shebang::env(PYTHON)),
            strict: None,
        }
    }

    /// An extract carries a shebang, so it runs as `./x.py`.
    fn executable(&self) -> bool {
        true
    }

    /// The `src/lint` §I python defaults: `ruff check` (read as ruff's
    /// json), `ruff format --check`, `mypy` and `bandit`.
    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![
            LintCmd {
                argv: vec![
                    "ruff".to_owned(),
                    "check".to_owned(),
                    "--output-format=json".to_owned(),
                ],
                file_arg: FileArg::Append,
                format: Format::Json("ruff"),
            },
            raw(&["ruff", "format", "--check"]),
            raw(&["mypy"]),
            raw(&["bandit", "-q"]),
        ]
    }

    /// `ruff check --fix`, then `ruff format` (`src/lint` §I).
    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![raw(&["ruff", "check", "--fix"]), raw(&["ruff", "format"])]
    }
}

/// A command whose output has no machine-readable form.
fn raw(argv: &[&str]) -> LintCmd {
    LintCmd {
        argv: argv.iter().map(|a| (*a).to_owned()).collect(),
        file_arg: FileArg::Append,
        format: Format::Raw,
    }
}
