//! The registry: which languages this build carries (`src:V41`).
//!
//! ONE file, and the only one in the root crate that names a language
//! crate or a `lang-*` feature. Every engine iterates [`hosts`] and
//! [`guests`] and never asks "is shell compiled in?" any other way, so
//! the `src:V30` no-leak rule is a property of this file rather than a
//! convention every module has to remember -- and a test
//! (`registry/tests.rs`) greps the rest of `src/` to keep it that way.
//!
//! Each entry sits behind its own `#[cfg(feature = "lang-<lang>")]`, and
//! the lists are sorted by [`LangId`], so the order an engine meets
//! hosts in is the same in every subset build (`src:V11`).
//!
//! A guest a host names but this build lacks is never guessed about
//! (`src:V42`): [`require_guest`] says which feature would bring it, and
//! [`on_missing_guest`] applies `[langs] missing_guest` to that answer.

use std::fmt;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Guest, Host, LangId};

use crate::cli::EXIT_USAGE;
use crate::config::Policy;
use crate::model::Warning;

#[cfg(test)]
mod tests;

/// The warning code for a site whose guest is compiled out, under
/// `[langs] missing_guest = "warn"` (`src:V42`). Stable, matched like a
/// rule id.
pub const MISSING_GUEST: &str = "missing-guest";

/// Every compiled-in host, sorted by [`LangId`].
const HOSTS: &[&dyn Host] = &[
    #[cfg(feature = "lang-nix")]
    &xenolith_lang_nix::NixHost,
    #[cfg(feature = "lang-pkl")]
    &xenolith_lang_pkl::PklHost,
    #[cfg(feature = "lang-shell")]
    &xenolith_lang_shell::ShellHost,
];

/// Every compiled-in guest, sorted by [`LangId`].
const GUESTS: &[&dyn Guest] = &[
    #[cfg(feature = "lang-shell")]
    &xenolith_lang_shell::ShellGuest,
];

/// The hosts this build carries, sorted by [`LangId`] (`src:V41`).
#[must_use]
pub fn hosts() -> &'static [&'static dyn Host] {
    HOSTS
}

/// The guests this build carries, sorted by [`LangId`] (`src:V41`).
#[must_use]
pub fn guests() -> &'static [&'static dyn Guest] {
    GUESTS
}

/// The host for `id`, when this build carries one.
#[must_use]
pub fn host(id: LangId) -> Option<&'static dyn Host> {
    hosts().iter().copied().find(|h| h.id() == id)
}

/// The guest for `id`, when this build carries one.
#[must_use]
pub fn guest(id: LangId) -> Option<&'static dyn Guest> {
    guests().iter().copied().find(|g| g.id() == id)
}

/// Whether this build carries the crate for `id`, as a host, a guest or
/// both. A language with no crate yet has no feature and is never
/// compiled in.
#[must_use]
pub fn compiled_in(id: LangId) -> bool {
    host(id).is_some() || guest(id).is_some()
}

/// The Cargo feature that carries `id`: `lang-<id>` (`src:C1`), spelled
/// with `LangId::as_str` like every other name for a language.
#[must_use]
pub fn feature(id: LangId) -> String {
    format!("lang-{id}")
}

/// Every language with a crate, and so a `lang-<id>` feature, whether
/// or not this build turned it on. Names, not `cfg`s: the one list of
/// which features EXIST, kept equal to `Cargo.toml`'s `[features]` by a
/// test, so a message never sends anyone after a feature no crate
/// provides (`src:V42`, `src:B8`).
const FEATURED: &[LangId] = &[LangId::Nix, LangId::Pkl, LangId::Shell];

/// [`feature`] for `id` when that feature exists, else `None`: no crate
/// provides the language yet, and no build can have it.
#[must_use]
pub fn existing_feature(id: LangId) -> Option<String> {
    FEATURED.contains(&id).then(|| feature(id))
}

/// How a message says a build lacks `id`: the feature to rebuild with,
/// or that no build supports it yet.
fn lacking(id: LangId) -> String {
    match existing_feature(id) {
        Some(feature) => format!("this build has no {id} guest: rebuild with feature `{feature}`"),
        None => format!("xenolith has no support for {id} in this build: no crate provides it yet"),
    }
}

/// A site's guest this build does not carry (`src:V42`). Exit 2 under
/// the default `[langs] missing_guest = "error"`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissingGuest {
    /// The guest the host named.
    pub guest: LangId,
    /// The host file holding the site, when the caller knows it.
    pub file: Option<PathBuf>,
}

impl MissingGuest {
    /// The process exit code: always 2 (`src/cli:V24`).
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        EXIT_USAGE
    }
}

impl fmt::Display for MissingGuest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(file) = &self.file {
            write!(f, "{}: ", file.display())?;
        }
        write!(
            f,
            "a site holds {} code, but {}; or set `[langs] missing_guest` to `warn` or \
             `ignore` (src:V42)",
            self.guest,
            lacking(self.guest),
        )
    }
}

impl std::error::Error for MissingGuest {}

/// The guest for `id`, or a [`MissingGuest`] naming the feature that
/// would bring it -- never a guess at whether the body is trivial
/// (`src:V42`).
///
/// # Errors
///
/// [`MissingGuest`] when this build does not carry `id` as a guest.
pub fn require_guest(id: LangId) -> Result<&'static dyn Guest, MissingGuest> {
    guest(id).ok_or(MissingGuest {
        guest: id,
        file: None,
    })
}

/// The warning for a site whose guest a SHEBANG named -- `interpreter`,
/// when the body's `#!` line gave one -- and this build lacks: always a
/// warning, whatever `[langs] missing_guest` says (`src:V42`). The line
/// is the file saying what it is, and a language xenolith cannot check
/// yet must not stop the run or hide the rest of the report.
#[must_use]
pub fn missing_shebang_guest(id: LangId, file: &Path, interpreter: Option<&str>) -> Warning {
    let named = interpreter.map_or_else(String::new, |i| format!(" (`{i}`)"));
    Warning {
        code: MISSING_GUEST.to_owned(),
        file: Some(file.to_path_buf()),
        message: format!(
            "a shebang{named} names {id}, but {}; the site was not checked (src:V42)",
            lacking(id)
        ),
    }
}

/// `[langs] missing_guest` applied to a site in `file` whose guest `id`
/// is compiled out (`src:V42`, `src:T88`): `error` refuses, `warn`
/// returns the warning to report, `ignore` returns nothing. Either way
/// the site is not judged -- the body is neither flagged nor passed.
///
/// # Errors
///
/// [`MissingGuest`] under [`Policy::Error`].
pub fn on_missing_guest(
    policy: Policy,
    id: LangId,
    file: &Path,
) -> Result<Option<Warning>, MissingGuest> {
    let missing = MissingGuest {
        guest: id,
        file: Some(file.to_path_buf()),
    };
    match policy {
        Policy::Error => Err(missing),
        Policy::Warn => Ok(Some(Warning {
            code: MISSING_GUEST.to_owned(),
            file: Some(file.to_path_buf()),
            message: format!(
                "a site holds {id} code, but {}; the site was not checked (src:V42)",
                lacking(id)
            ),
        })),
        Policy::Ignore => Ok(None),
    }
}
