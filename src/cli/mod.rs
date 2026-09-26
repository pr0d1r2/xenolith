//! `xnl`, the xenolith command line: the dispatch the binary runs.
//!
//! Here in the library rather than in `main.rs` (`src:C139`): `main.rs`
//! is a shim with no mirror of its own, so logic kept there is logic no
//! unit test can reach. The binary calls [`main`]; [`run`] takes its
//! arguments and both streams as parameters, which is what lets
//! `src/cli/tests.rs` drive every branch without spawning a process.
//!
//! The verbs (`check`, `extract`, `graph`, `lint`), their flags and their
//! exit codes belong to `src/cli` and arrive with `src/cli:T9`. What this
//! module does today is answer for the binary's identity and REFUSE
//! everything else -- loudly, with exit 2.
//!
//! Refusing matters more than it looks. A binary that accepts `xnl check`
//! and exits 0 having scanned nothing is indistinguishable, in a gate,
//! from one that scanned the tree and found it clean.

use std::io::Write;
use std::process::ExitCode;

#[cfg(test)]
mod tests;

/// Exit code for "the request was understood and not carried out"
/// (`src/cli:V24`): usage errors and unimplemented verbs alike. Distinct
/// from 1, which is reserved for findings.
pub const EXIT_USAGE: u8 = 2;

/// The binary's entry point: the process arguments, stdout and stderr.
#[must_use]
pub fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    ExitCode::from(run(&args, &mut std::io::stdout(), &mut std::io::stderr()))
}

/// Dispatch `args` (without the program name), writing to `out` and
/// `err`, and return the exit code.
pub fn run(args: &[String], out: &mut impl Write, err: &mut impl Write) -> u8 {
    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            // As in `refuse`: a failed write to stdout has nowhere to be
            // reported, and a panic would only add noise on stderr.
            let _ = writeln!(out, "xnl {}", crate::VERSION);
            0
        }
        Some(verb) => refuse(
            err,
            &format!(
                "xnl: `{verb}` is not implemented yet. The verbs check, extract, \
                 graph and lint arrive with src/cli:T9; this build knows only \
                 --version."
            ),
        ),
        None => refuse(
            err,
            "usage: xnl --version\n\nNo verb is implemented yet \
             (src/cli:T9). This build reports its version and refuses \
             everything else rather than exiting 0 having done nothing.",
        ),
    }
}

/// Write a refusal to `err` and return the usage exit code.
///
/// stderr, not stdout: the machine-readable output of every verb is
/// stdout (`src/cli` §I), and a refusal that lands there corrupts a caller
/// parsing JSON.
fn refuse(err: &mut impl Write, message: &str) -> u8 {
    // A failed write to stderr cannot itself be reported anywhere, so the
    // result is dropped deliberately rather than unwrapped: panicking here
    // would replace a clean exit 2 with a panic message on the same
    // stream.
    let _ = writeln!(err, "{message}");
    EXIT_USAGE
}
