//! Tcl as a guest (`languages/shells/tcl:V197`): what an extract of it is
//! called, how it runs, and what goes above its body.
//!
//! The dialect decides the file: an expect body lands in `*.exp` under
//! `#!/usr/bin/env expect` and runs as `expect x.exp`, anything else in
//! `*.tcl` under `tclsh`. Nothing is strict: Tcl has no `set -e` to
//! reproduce, an error already stops a script.
//!
//! Triviality is not decided (`languages/shells/tcl:V197` `?`), so this
//! guest names no constructs and the engine judges a body by
//! `[threshold.tcl] max_lines` / `max_bytes` (`languages/api:V37`). What
//! [`TclGuest::trivial`] does decide is that a body the grammar rejects is
//! never trivial (`languages:V77`).

use std::path::Path;

use xenolith_lang_api::{
    Error, Guest, GuestEnv, Invoke, LangId, LintCmd, Prelude, Result, Shebang,
};

use crate::grammar;
use crate::host::EXPECT;
use crate::syntax;

#[cfg(test)]
mod tests;

/// Tcl as a guest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TclGuest;

/// The interpreter of plain tcl.
const TCLSH: &str = "tclsh";

/// Whether the site's dialect is expect.
fn is_expect(env: &GuestEnv) -> bool {
    env.dialect.as_deref() == Some(EXPECT)
}

impl Guest for TclGuest {
    fn id(&self) -> LangId {
        LangId::Tcl
    }

    /// `exp` for the expect dialect, else `tcl`.
    fn extension(&self, env: &GuestEnv) -> &'static str {
        if is_expect(env) { "exp" } else { "tcl" }
    }

    /// `expect {path}` for an `*.exp` file, else `tclsh {path}`: the path
    /// carries the dialect, since [`TclGuest::extension`] put it there.
    fn invoke(&self, path: &Path) -> Invoke {
        let expect = path.extension().is_some_and(|ext| ext == "exp");
        let interpreter = if expect { EXPECT } else { TCLSH };
        Invoke {
            argv: vec![interpreter.to_owned(), path.display().to_string()],
        }
    }

    /// Never trivial when the grammar rejects the body (`languages:V77`);
    /// otherwise `false`, leaving the verdict to the size threshold.
    fn trivial(&self, body: &str) -> Result<bool> {
        let tree = grammar::parse(body)?;
        if tree.root_node().has_error() {
            return Err(Error::parse(
                LangId::Tcl,
                "the body is not valid tcl to this grammar",
            ));
        }
        Ok(false)
    }

    /// `#!/usr/bin/env tclsh` or `#!/usr/bin/env expect`, and no strict
    /// line (`languages/shells/tcl:V197`).
    fn prelude(&self, env: &GuestEnv) -> Prelude {
        let interpreter = if is_expect(env) { EXPECT } else { TCLSH };
        Prelude {
            shebang: Some(Shebang::env(interpreter)),
            strict: None,
        }
    }

    /// An extract carries a shebang, so it runs as `./x.tcl`.
    fn executable(&self) -> bool {
        true
    }

    /// The same syntax check as a tcl host file's, in either dialect: an
    /// extract is tcl text like any other (`languages/shells/tcl:V198`).
    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        vec![syntax::lint_cmd()]
    }

    /// None: a syntax error has no mechanical fix
    /// (`languages/shells/tcl:V198`).
    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
}
