//! The one place this crate touches FFI.
//!
//! The grammar is vendored C (`languages:V121`), so reaching it is an
//! `extern` call and a raw-pointer conversion. Both are here and nowhere
//! else, which is why this module -- and only this module -- is allowed
//! `unsafe_code` (see the `[lints]` note in `Cargo.toml`).

#![allow(unsafe_code)]

use tree_sitter::Language;
use tree_sitter::ffi::TSLanguage;

unsafe extern "C" {
    /// Defined by the vendored `parser.c`.
    fn tree_sitter_pkl() -> *const TSLanguage;
}

/// The pkl grammar, ready for `Parser::set_language`.
#[must_use]
pub fn language() -> Language {
    // SAFETY: `tree_sitter_pkl` is the generated entry point of the
    // vendored grammar. It takes no arguments and returns a pointer to a
    // static, never-null `TSLanguage`, which is the whole contract
    // `from_raw` asks for.
    unsafe { Language::from_raw(tree_sitter_pkl()) }
}
