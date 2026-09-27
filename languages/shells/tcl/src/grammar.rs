//! The one place this crate touches FFI.
//!
//! The grammar is vendored C (`languages:V121`), so reaching it is an
//! `extern` call and a raw-pointer conversion. Both are here and nowhere
//! else, which is why this module -- and only this module -- is allowed
//! `unsafe_code` (see the `[lints]` note in `Cargo.toml`).

#![allow(unsafe_code)]

use tree_sitter::ffi::TSLanguage;
use tree_sitter::{Language, Parser, Tree};
use xenolith_lang_api::{Error, LangId, Result};

#[cfg(test)]
mod tests;

unsafe extern "C" {
    /// Defined by the vendored `parser.c`.
    fn tree_sitter_tcl() -> *const TSLanguage;
}

/// The tcl grammar, ready for `Parser::set_language`.
#[must_use]
pub fn language() -> Language {
    // SAFETY: `tree_sitter_tcl` is the generated entry point of the
    // vendored grammar. It takes no arguments and returns a pointer to a
    // static, never-null `TSLanguage`, which is the whole contract
    // `from_raw` asks for.
    unsafe { Language::from_raw(tree_sitter_tcl()) }
}

/// `src` parsed as tcl, error nodes and all: whether a tree with errors
/// is usable is the caller's question, not the parser's.
///
/// # Errors
///
/// [`Error::Parse`] when the grammar cannot be loaded or the parser
/// returns no tree at all.
pub fn parse(src: &str) -> Result<Tree> {
    let mut parser = Parser::new();
    parser
        .set_language(&language())
        .map_err(|e| Error::parse(LangId::Tcl, format!("cannot load the tcl grammar: {e}")))?;
    parser
        .parse(src, None)
        .ok_or_else(|| Error::parse(LangId::Tcl, "the tcl parser returned no tree"))
}
