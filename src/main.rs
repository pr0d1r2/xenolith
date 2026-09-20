//! `xnl`, the xenolith command line.
//!
//! The verbs (`check`, `extract`, `graph`, `lint`), their flags and their
//! exit codes belong to `src/cli` and arrive with `src/cli:T9`. What this
//! file does today is answer for the binary's identity and REFUSE
//! everything else -- loudly, with exit 2.
//!
//! Refusing matters more than it looks. A binary that accepts `xnl check`
//! and exits 0 having scanned nothing is indistinguishable, in a gate,
//! from one that scanned the tree and found it clean.

use std::io::Write;
use std::process::ExitCode;

/// Exit code for "the request was understood and not carried out"
/// (`src` §I): usage errors and unimplemented verbs alike. Distinct from
/// 1, which is reserved for findings.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    match args.first().map(String::as_str) {
        Some("--version" | "-V") => {
            println!("xnl {}", xenolith::VERSION);
            ExitCode::SUCCESS
        }
        Some(verb) => refuse(&format!(
            "xnl: `{verb}` is not implemented yet. The verbs check, extract, \
             graph and lint arrive with src/cli:T9; this build knows only \
             --version."
        )),
        None => refuse(
            "usage: xnl --version\n\nNo verb is implemented yet \
             (src/cli:T9). This build reports its version and refuses \
             everything else rather than exiting 0 having done nothing.",
        ),
    }
}

/// Write a refusal to stderr and return the usage exit code.
///
/// stderr, not stdout: the machine-readable output of every verb is
/// stdout (`src/cli` §I), and a refusal that lands there corrupts a caller
/// parsing JSON.
fn refuse(message: &str) -> ExitCode {
    let mut err = std::io::stderr();
    // A failed write to stderr cannot itself be reported anywhere, so the
    // result is dropped deliberately rather than unwrapped: panicking here
    // would replace a clean exit 2 with a panic message on the same
    // stream.
    let _ = writeln!(err, "{message}");
    ExitCode::from(EXIT_USAGE)
}
