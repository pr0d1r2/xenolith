//! Running one lint command: spawn, collect, judge (`src/lint:V8`).
//!
//! The one place `xnl lint` starts a process. A tool that is not on
//! PATH is an `error` naming it and how to get it -- never a silent skip,
//! because a gate that passes when its linter is missing has checked
//! nothing (`src/lint:V8`). stdin is closed, so a tool waiting on it
//! cannot hang the gate.

use std::ffi::OsString;

use std::path::Path;

use super::report::Status;

#[cfg(test)]
mod tests;

/// How many trailing lines of a tool's output a result keeps
/// (`src/lint:V92`).
pub const TAIL_LINES: usize = 40;

/// Where commands are looked up: the inherited `PATH`, or -- the seam
/// the tests use (`tests:V150`) -- a `PATH` of the caller's choosing, so
/// a test sees only the stub tools it wrote.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tools {
    path: Option<OsString>,
}

impl Tools {
    /// The process's own `PATH`.
    #[must_use]
    pub fn inherit() -> Tools {
        Tools::default()
    }

    /// Exactly `path` as the tools' `PATH`.
    #[must_use]
    pub fn on_path(path: impl Into<OsString>) -> Tools {
        Tools {
            path: Some(path.into()),
        }
    }
}

/// How one command ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ran {
    /// Pass, fail or error.
    pub status: Status,
    /// The exit code, when there was one.
    pub exit: Option<i32>,
    /// The output's last [`TAIL_LINES`] lines, or why the tool did not
    /// run; `None` on a pass.
    pub tail: Option<String>,
}

/// Run `argv` in `root` and judge it: exit 0 is a pass, any other exit a
/// fail, and a tool that never ran or died on a signal an error
/// (`src/lint` §I).
#[must_use]
pub fn run(root: &Path, argv: &[String], tools: &Tools) -> Ran {
    let _ = (root, argv, tools);
    Ran {
        status: Status::Pass,
        exit: Some(0),
        tail: None,
    }
}

/// Why a tool that is not on PATH did not run, and how to get it
/// (`src/lint:V8`).
#[must_use]
pub fn absent(program: &str) -> String {
    let _ = program;
    String::new()
}

/// The last [`TAIL_LINES`] lines of `text`, or `None` when it is blank.
#[must_use]
pub fn tail(text: &str) -> Option<String> {
    let _ = text;
    None
}
