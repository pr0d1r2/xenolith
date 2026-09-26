//! The registry: which languages this build carries (`src:V41`).
//!
//! RED stub (`src:T46`): the shape the tests in `registry/tests.rs` are
//! written against, with every answer empty or refused. The GREEN commit
//! fills the lists behind their `lang-*` features.

use std::fmt;
use std::path::PathBuf;

use xenolith_lang_api::{Guest, Host, LangId};

use crate::cli::EXIT_USAGE;

#[cfg(test)]
mod tests;

/// The hosts this build carries, sorted by [`LangId`] (`src:V41`).
#[must_use]
pub fn hosts() -> &'static [&'static dyn Host] {
    &[]
}

/// The guests this build carries, sorted by [`LangId`] (`src:V41`).
#[must_use]
pub fn guests() -> &'static [&'static dyn Guest] {
    &[]
}

/// The host for `id`, when this build carries one.
#[must_use]
pub fn host(_id: LangId) -> Option<&'static dyn Host> {
    None
}

/// The guest for `id`, when this build carries one.
#[must_use]
pub fn guest(_id: LangId) -> Option<&'static dyn Guest> {
    None
}

/// Whether this build carries the crate for `id`.
#[must_use]
pub fn compiled_in(_id: LangId) -> bool {
    false
}

/// The Cargo feature that carries `id`.
#[must_use]
pub fn feature(_id: LangId) -> String {
    String::new()
}

/// A site's guest this build does not carry (`src:V42`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingGuest {
    /// The guest the host named.
    pub guest: LangId,
    /// The host file holding the site, when the caller knows it.
    pub file: Option<PathBuf>,
}

impl MissingGuest {
    /// The process exit code.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for MissingGuest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} is missing", self.guest)
    }
}

impl std::error::Error for MissingGuest {}

/// The guest for `id`, or a [`MissingGuest`].
///
/// # Errors
///
/// [`MissingGuest`], always, in this stub.
pub fn require_guest(id: LangId) -> Result<&'static dyn Guest, MissingGuest> {
    Err(MissingGuest {
        guest: id,
        file: None,
    })
}
