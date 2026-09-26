//! `xnl`, the xenolith command line: the dispatch the binary runs.
//!
//! Here in the library rather than in `main.rs` (`src:C139`): `main.rs`
//! is a shim with no mirror of its own, so logic kept there is logic no
//! unit test can reach. The binary calls [`main`]; [`run`] takes its
//! arguments and both streams as parameters, which is what lets
//! `src/cli/tests.rs` drive every branch without spawning a process.
//!
//! Three layers, one file each (`src/cli:T9`):
//!
//! * [`args`] -- what was ASKED: the verbs `check`, `extract`, `graph`,
//!   `lint`, `langs` and their flags, parsed in full.
//! * this module -- what to DO about it: one match arm per verb.
//! * [`langs`] -- the one verb with nothing to scan, answered here.
//! * [`check`] -- `xnl check`: config in, the engine's report out
//!   (`src:V152`: the engine decides, `src/cli` renders and maps exit
//!   codes).
//!
//! The scanning verbs whose engines have not landed parse every flag and
//! path and then REFUSE with exit 2, naming the task that brings their
//! engine. Refusing matters more than it looks: a binary that accepts
//! `xnl graph` and exits 0 having scanned nothing is indistinguishable,
//! in a gate, from one that scanned the tree and found it clean. Wiring
//! an engine in is replacing its arm's [`not_yet`] with one call and a
//! render, as `check`'s arm did.

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::Path;
use std::process::ExitCode;

use self::args::{Invocation, OutputFormat, Scan, Usage, Verb};

pub mod args;
pub mod check;
pub mod langs;

#[cfg(test)]
mod tests;

/// Exit code for "done, nothing found" (`src/cli:V24`).
pub const EXIT_OK: u8 = 0;

/// Exit code for "the request was understood and not carried out"
/// (`src/cli:V24`): usage errors and unimplemented verbs alike. Distinct
/// from 1, which is reserved for findings.
pub const EXIT_USAGE: u8 = 2;

/// The whole grammar, for a refusal to end with (`src/cli` §I).
const USAGE: &str = "\
usage: xnl <verb> [flags] [paths...]

  xnl check   [--format human|json|sarif] [paths...]
  xnl extract [--write] [--relocate] <path>[:line]...
  xnl graph   [--format human|json|sarif] [paths...]
  xnl lint    [--fix] [--trust-config] [--format human|json|sarif] [paths...]
  xnl langs   [--format human|json]
  xnl --version

every verb also takes --verbose and --strict-hosts; `--` ends the flags.
exit: 0 ok, 1 violation, 2 usage or refused.";

/// The binary's entry point: the process arguments, stdout and stderr.
///
/// `args_os`, not `args`: the latter panics on an argument that is not
/// UTF-8, and a filename from hk can be exactly that.
#[must_use]
pub fn main() -> ExitCode {
    let args: Vec<OsString> = std::env::args_os().skip(1).collect();
    ExitCode::from(run(&args, &mut std::io::stdout(), &mut std::io::stderr()))
}

/// Dispatch `args` (without the program name), writing to `out` and
/// `err`, and return the exit code. Paths are relative to the current
/// directory, which is the root a scanning verb works from.
pub fn run<A: AsRef<OsStr>>(args: &[A], out: &mut impl Write, err: &mut impl Write) -> u8 {
    // An empty root joins to relative paths and is `git -C ""`, which
    // git reads as "stay here": the current directory, even when it
    // cannot be named.
    let root = std::env::current_dir().unwrap_or_default();
    run_in(&root, args, out, err)
}

/// [`run`] from `root` rather than the current directory -- the seam the
/// tests drive a scanning verb through.
pub fn run_in<A: AsRef<OsStr>>(
    root: &Path,
    args: &[A],
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    if args.is_empty() {
        return refuse(err, USAGE);
    }
    match args::parse(args) {
        Ok(invocation) => dispatch(root, &invocation, out, err),
        // The problem first, so a terminal showing only the top of the
        // message shows the mistake.
        Err(Usage(problem)) => refuse(err, &format!("xnl: {problem}\n\n{USAGE}")),
    }
}

/// One arm per verb. An arm whose engine has landed calls it and
/// renders the result; the rest refuse.
fn dispatch(
    root: &Path,
    invocation: &Invocation,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    match &invocation.verb {
        Verb::Version => {
            // As in `refuse`: a failed write to stdout has nowhere to be
            // reported, and a panic would only add noise on stderr.
            let _ = writeln!(out, "xnl {}", crate::VERSION);
            EXIT_OK
        }
        Verb::Langs { format } => {
            let text = match format {
                OutputFormat::Json => langs::json(),
                // The parser refuses `langs --format sarif`, so
                // `Sarif` does not reach here; if it ever did, the
                // human list beats an empty stdout.
                OutputFormat::Human | OutputFormat::Sarif => langs::human(),
            };
            let _ = out.write_all(text.as_bytes());
            EXIT_OK
        }
        Verb::Check(scan) => sarif(err, "check", scan)
            .unwrap_or_else(|| check::run(root, scan, invocation.verbose, out, err)),
        Verb::Graph(scan) => scanning(err, "graph", scan, "src/graph:T21"),
        Verb::Lint { scan, .. } => scanning(err, "lint", scan, "src/lint:T24"),
        Verb::Extract { .. } => not_yet(err, "extract", "src/extract:T22"),
    }
}

/// `--format sarif`, refused on its own: its writer is its own task
/// (`src/cli:T103`), and it is missing whether or not the verb's engine
/// has landed.
fn sarif(err: &mut impl Write, verb: &str, scan: &Scan) -> Option<u8> {
    (scan.format == OutputFormat::Sarif).then(|| {
        refuse(
            err,
            &format!(
                "xnl: `{verb} --format sarif` is not implemented yet: the SARIF \
                 writer arrives with src/cli:T103."
            ),
        )
    })
}

/// A scanning verb whose engine has not landed; `--format sarif` is
/// refused first, by [`sarif`].
fn scanning(err: &mut impl Write, verb: &str, scan: &Scan, task: &str) -> u8 {
    sarif(err, verb, scan).unwrap_or_else(|| not_yet(err, verb, task))
}

/// Refuse `verb`, naming the task that brings it.
fn not_yet(err: &mut impl Write, verb: &str, task: &str) -> u8 {
    refuse(
        err,
        &format!(
            "xnl: `{verb}` is not implemented yet: it arrives with {task}. \
             This build refuses rather than exit 0 having done nothing."
        ),
    )
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
