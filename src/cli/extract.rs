//! `xnl extract`: read `xenolith.toml`, run the extract engine, print
//! the diff or write it (`src/extract` §G, `src/cli` §I).
//!
//! Input and output only, as the other verbs' modules are: the engine
//! ([`crate::extract`]) decides what moves where and what it refuses;
//! this module decides the streams and the exit code. The diff is the
//! only thing on stdout, so hk's `check_diff` can show it as it is;
//! refusals, warnings and `--verbose` go to stderr.
//!
//! Exit (`src/cli:V24`): 2 when anything was refused, else 1 when the
//! diff is not empty, else 0. Under `--write` a change made is success:
//! 0, or 2 when something was refused -- the other files are written
//! all the same (`src/extract:V64`).

use std::io::Write;
use std::path::Path;

use super::args::Target;
use super::{EXIT_OK, EXIT_USAGE, refuse};
use crate::extract::{self, Edit, Options};

#[cfg(test)]
mod tests;

/// The flags `xnl extract` runs under.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--write`: apply rather than print (`src/extract:C15`).
    pub write: bool,
    /// `--verbose`: explain every placement and skip.
    pub verbose: bool,
    /// `--strict-hosts` (`src:V13`).
    pub strict_hosts: bool,
}

/// Run `xnl extract` from `root` over `targets`, writing to `out` and
/// `err`.
pub fn run(
    root: &Path,
    targets: &[Target],
    flags: Flags,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let config = match super::check::load(root) {
        Ok(config) => config,
        Err(message) => return refuse(err, &message),
    };
    let options = Options {
        targets: targets
            .iter()
            .map(|t| extract::Target {
                path: t.path.clone(),
                line: t.line,
            })
            .collect(),
        strict_hosts: flags.strict_hosts,
    };
    let edit = match extract::extract(root, &config, &options) {
        Ok(edit) => edit,
        Err(e) => return refuse(err, &format!("xnl: {e}")),
    };
    if flags.write {
        write(root, &edit, flags.verbose, err)
    } else {
        render(&edit, flags.verbose, out, err)
    }
}

/// Print `edit` as a diff and return its exit code.
pub fn render(edit: &Edit, verbose: bool, out: &mut impl Write, err: &mut impl Write) -> u8 {
    // As everywhere in the CLI: a failed write has nowhere to be
    // reported, and the exit code still says what was found.
    notes(edit, verbose, err);
    let _ = out.write_all(edit.diff().as_bytes());
    edit.exit_code()
}

/// Carry `edit` out and return the exit code: 0, or 2 when anything
/// was refused or a write failed.
pub fn write(root: &Path, edit: &Edit, verbose: bool, err: &mut impl Write) -> u8 {
    notes(edit, verbose, err);
    match extract::write::apply(root, edit) {
        Ok(written) => {
            if verbose {
                for path in written {
                    let _ = writeln!(err, "wrote {path}");
                }
            }
            if edit.refusals.is_empty() {
                EXIT_OK
            } else {
                EXIT_USAGE
            }
        }
        Err(e) => refuse(err, &format!("xnl: extract --write: {e}")),
    }
}

/// Warnings, `--verbose` explanations and refusals, on stderr.
fn notes(edit: &Edit, verbose: bool, err: &mut impl Write) {
    for warning in &edit.warnings {
        let _ = writeln!(err, "{}", warning.to_human());
    }
    if verbose {
        for line in &edit.explain {
            let _ = writeln!(err, "{line}");
        }
    }
    for refusal in &edit.refusals {
        let _ = writeln!(err, "xnl: {refusal}");
    }
}
