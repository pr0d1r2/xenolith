//! Whether a site can name nix-shebang, and by what (`languages/ci/nix:V53`,
//! `languages/ci/nix:V170`).
//!
//! `rewrite` writes `nix-shebang.lib.readWithoutStrict` only where that
//! call evaluates, and nothing but the tree says where: a name in scope
//! is a lexical binding, found by walking up from the site. Scope, never
//! value -- `let nix-shebang = 1;` binds the name as surely as the flake
//! input does -- and the proof is conservative: a form this module cannot
//! read proves nothing, and nothing means `builtins.readFile`, which
//! always evaluates.
//!
//! Proofs, in the order they are tried:
//!
//! - a lexical binding of `nix-shebang` itself: a lambda's argument,
//!   formal or `@` name, a `let`, a `rec` set;
//! - failing one, a `with` whose namespace is an attribute set literal
//!   binding it. A `with` never shadows a lexical binding, but an inner
//!   one does shadow an outer, so a namespace this module cannot read
//!   ends the search;
//! - failing both, a path the file already uses, `<x>.....nix-shebang.lib....`,
//!   whose base `<x>` has the SAME lexical binder at the use and at the
//!   site: then `<x>.....nix-shebang` is the one the file already reaches.

use rnix::ast::{self, AstNode, HasEntry};
use rnix::{SyntaxKind, SyntaxNode};

#[cfg(test)]
mod tests;

/// The name the flake input goes by.
const NAME: &str = "nix-shebang";

/// The expression naming nix-shebang at `at` -- `nix-shebang` or a path
/// ending in it -- when this module can prove one is in scope there.
pub(crate) fn nix_shebang(at: &SyntaxNode) -> Option<String> {
    if binder(at, NAME).is_some() || with_supplies(at, NAME) {
        return Some(NAME.to_owned());
    }
    let root = at.ancestors().last()?;
    prefix_uses(&root).into_iter().find_map(|(base, prefix)| {
        let name = base.text().to_string();
        let used = binder(&base, &name)?;
        (binder(at, &name) == Some(used)).then_some(prefix)
    })
}

/// The nearest scope enclosing `at` that binds `name` lexically.
fn binder(at: &SyntaxNode, name: &str) -> Option<SyntaxNode> {
    at.ancestors().find(|scope| binds(scope, name))
}

/// Whether `scope` binds `name` lexically for everything inside it: a
/// lambda by its argument, a formal or its `@` name; a `let` or a `rec`
/// set by a binding or an `inherit`. A plain set, a `with` and a legacy
/// `let { }` bind nothing this module reads.
fn binds(scope: &SyntaxNode, name: &str) -> bool {
    match scope.kind() {
        SyntaxKind::NODE_LAMBDA => cast::<ast::Lambda>(scope.clone())
            .and_then(|lambda| lambda.param())
            .is_some_and(|param| param_binds(&param, name)),
        SyntaxKind::NODE_LET_IN => {
            cast::<ast::LetIn>(scope.clone()).is_some_and(|set| entries_bind(&set, name))
        }
        SyntaxKind::NODE_ATTR_SET => cast::<ast::AttrSet>(scope.clone())
            .is_some_and(|set| set.rec_token().is_some() && entries_bind(&set, name)),
        _ => false,
    }
}

/// Whether a lambda's parameter names `name`.
fn param_binds(param: &ast::Param, name: &str) -> bool {
    let is = |ident: Option<ast::Ident>| ident.is_some_and(|i| text(&i) == name);
    match param {
        ast::Param::IdentParam(ident) => is(ident.ident()),
        ast::Param::Pattern(pattern) => {
            pattern.pat_entries().any(|entry| is(entry.ident()))
                || is(pattern.pat_bind().and_then(|bind| bind.ident()))
        }
    }
}

/// Whether a `let` or a set binds `name`: `name = ...`, `name.a = ...`, or
/// `inherit name` with or without a source.
fn entries_bind(set: &impl HasEntry, name: &str) -> bool {
    let is = |attr: Option<ast::Attr>| match attr {
        Some(ast::Attr::Ident(ident)) => text(&ident) == name,
        _ => false,
    };
    set.attrpath_values()
        .any(|value| is(value.attrpath().and_then(|path| path.attrs().next())))
        || set
            .inherits()
            .any(|inherit| inherit.attrs().any(|attr| is(Some(attr))))
}

/// Whether the innermost `with` around `at` whose namespace says
/// anything about `name` binds it: namespaces are read outward from
/// `at`, a literal set without `name` is passed over, and any other
/// namespace -- `with pkgs;` -- may hold `name` and ends the search.
fn with_supplies(at: &SyntaxNode, name: &str) -> bool {
    let mut child = at.clone();
    for scope in at.ancestors().skip(1) {
        if let Some(with) = cast::<ast::With>(scope.clone())
            && with.body().is_some_and(|body| node(&body) == &child)
        {
            match with.namespace() {
                Some(ast::Expr::AttrSet(set)) if entries_bind(&set, name) => return true,
                Some(ast::Expr::AttrSet(_)) => {}
                _ => return false,
            }
        }
        child = scope;
    }
    false
}

/// `node` as the typed node `T`, when it is one. The bound reaches
/// rowan's `AstNode` through rnix's own trait, so rowan stays a
/// dependency of rnix alone (`Cargo.toml`).
fn cast<T: AstNode>(node: SyntaxNode) -> Option<T> {
    T::cast(node)
}

/// The untyped node under a typed one.
fn node<T: AstNode>(typed: &T) -> &SyntaxNode {
    typed.syntax()
}

/// A typed node's source text.
fn text<T: AstNode>(typed: &T) -> String {
    typed.syntax().text().to_string()
}

/// Every `<x>.....nix-shebang.lib.<more>` in the tree under `root`, in
/// source order: the base identifier `<x>` and the path up to and
/// including `nix-shebang`. Plain identifier segments only.
fn prefix_uses(root: &SyntaxNode) -> Vec<(SyntaxNode, String)> {
    root.descendants()
        .filter_map(cast::<ast::Select>)
        .filter_map(|select| {
            let ast::Expr::Ident(base) = select.expr()? else {
                return None;
            };
            let mut segments = vec![text(&base)];
            for attr in select.attrpath()?.attrs() {
                let ast::Attr::Ident(ident) = attr else {
                    return None;
                };
                segments.push(text(&ident));
            }
            let at = segments.iter().skip(1).position(|s| s == NAME)? + 1;
            let lib = segments.get(at + 1).is_some_and(|s| s == "lib");
            let prefix = segments.get(..=at)?.join(".");
            (lib && segments.len() > at + 2).then(|| (node(&base).clone(), prefix))
        })
        .collect()
}
