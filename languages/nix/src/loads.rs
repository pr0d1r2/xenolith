//! The loads an extraction leaves in a nix file (`languages/nix:V53`):
//! `builtins.readFile ./x.sh`, or `nix-shebang.lib.readWithoutStrict
//! ./x.sh` when the extract carries a prelude.
//!
//! Read off the tree, never the text (`languages:V2`): a call in a
//! comment is no call, and a path is the grammar's path node. Anywhere
//! in the file counts -- a sink, a builder's argument, a `let` -- because
//! the graph asks what a file loads, not where; and only a RELATIVE path
//! literal with a shell extension, because that is what an extract is:
//! `readFile ./app.conf` reads data, `readFile /etc/x.sh` a file outside
//! the repo, and `./${x}.sh` a path nix alone can name.

use std::path::PathBuf;

use rnix::{SyntaxKind, SyntaxNode};
use xenolith_lang_api::{LangId, LoadRef};

use crate::span;

#[cfg(test)]
mod tests;

/// The load without a prelude, matched whole (`languages/nix:V53`): a
/// bare `readFile` is whatever its scope made it.
const READ_FILE: &[&str] = &["builtins", "readFile"];

/// The load with one, matched as the callee's TAIL: the input may be
/// reached through whatever binds it (`inputs.nix-shebang.lib…`).
const READ_WITHOUT_STRICT: &[&str] = &["nix-shebang", "lib", "readWithoutStrict"];

/// Whether a callee's dotted name is one of the two load calls.
fn is_load_call(name: &[String]) -> bool {
    let ends_with = |tail: &[&str]| {
        name.len() >= tail.len()
            && name
                .iter()
                .rev()
                .zip(tail.iter().rev())
                .all(|(a, b)| a.as_str() == *b)
    };
    (name.len() == READ_FILE.len() && ends_with(READ_FILE)) || ends_with(READ_WITHOUT_STRICT)
}

/// The extensions a load of a shell extract carries. Recognised, never
/// emitted: the extension is the guest's (`languages/api:V35`).
const SHELL_EXTENSIONS: &[&str] = &["bash", "sh", "zsh"];

/// Every load in the tree under `root`, sorted by span.
pub(crate) fn loads(root: &SyntaxNode) -> Vec<LoadRef> {
    let mut found: Vec<LoadRef> = root
        .descendants()
        .filter(|node| node.kind() == SyntaxKind::NODE_APPLY)
        .filter_map(|apply| load(&apply))
        .collect();
    found.sort_by_key(|load| load.span);
    found
}

/// The load `apply` is, when its function is a load call
/// ([`is_load_call`]) and its argument a relative path literal to a
/// shell file.
fn load(apply: &SyntaxNode) -> Option<LoadRef> {
    let mut children = apply.children();
    let (function, argument) = (children.next()?, children.next()?);
    if !is_load_call(&dotted(&function)?) {
        return None;
    }
    let path = relative_path(&argument)?;
    let extension = path.extension().and_then(|ext| ext.to_str())?;
    SHELL_EXTENSIONS.contains(&extension).then(|| LoadRef {
        span: span(apply.text_range()),
        path,
        guest: LangId::Shell,
    })
}

/// The segments of a function expression that is a plain name or a
/// select of one: `builtins.readFile` → `[builtins, readFile]`. `None`
/// for anything computed (`(f x).y`, `a.${b}`, `a."b"`), which names no
/// call this module can know.
fn dotted(function: &SyntaxNode) -> Option<Vec<String>> {
    match function.kind() {
        SyntaxKind::NODE_IDENT => Some(vec![function.text().to_string()]),
        SyntaxKind::NODE_SELECT => {
            let mut children = function.children();
            let base = children.next()?;
            if base.kind() != SyntaxKind::NODE_IDENT {
                return None;
            }
            let path = children.find(|c| c.kind() == SyntaxKind::NODE_ATTRPATH)?;
            let mut segments = vec![base.text().to_string()];
            for attr in path.children() {
                if attr.kind() != SyntaxKind::NODE_IDENT {
                    return None;
                }
                segments.push(attr.text().to_string());
            }
            Some(segments)
        }
        _ => None,
    }
}

/// The path a relative path literal names, as written: relative to the
/// host file's dir, the runtime base nix reads it from
/// (`languages/api/src/lens:V66`). `None` for an absolute, home or
/// search path, and for a path with a `${…}` in it.
fn relative_path(argument: &SyntaxNode) -> Option<PathBuf> {
    if argument.kind() != SyntaxKind::NODE_PATH_REL
        || argument
            .children()
            .any(|c| c.kind() == SyntaxKind::NODE_INTERPOL)
    {
        return None;
    }
    Some(PathBuf::from(argument.text().to_string()))
}
