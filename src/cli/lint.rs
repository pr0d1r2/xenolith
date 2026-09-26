//! `xnl lint`: read `xenolith.toml`, run the lint engine, render its
//! results (`src/lint:V8`, `src/cli` §I).
//!
//! Input and output only, as `xnl check`'s module is: the engine
//! ([`crate::lint`]) decides what ran and how it ended; this module
//! decides the stream, the format and the exit code -- 0 every check
//! passed, 1 one failed, 2 one could not run or the run was refused
//! (`src/lint:V92`, `src/cli:V24`).

use std::io::Write;
use std::path::Path;

use super::args::{OutputFormat, Scan};
use super::{not_yet, refuse};
use crate::lint::{LintReport, Options, Status};

#[cfg(test)]
mod tests;

/// The flags `xnl lint` takes beyond the scan (`src/cli` §I).
///
/// Four switches, each a flag the user typed; an enum would only rename
/// them.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Flags {
    /// `--fix`.
    pub fix: bool,
    /// `--trust-config`.
    pub trust_config: bool,
    /// `--verbose`.
    pub verbose: bool,
    /// `--strict-hosts`.
    pub strict_hosts: bool,
}

/// Run `xnl lint` from `root` over `scan`, writing to `out` and `err`.
pub fn run(
    root: &Path,
    scan: &Scan,
    flags: Flags,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    if flags.trust_config {
        return not_yet(err, "lint --trust-config", "src/lint:T92");
    }
    let config = match super::check::load(root) {
        Ok(config) => config,
        Err(message) => return refuse(err, &message),
    };
    let options = Options {
        paths: scan.paths.clone(),
        strict_hosts: flags.strict_hosts,
        fix: flags.fix,
    };
    match crate::lint::lint(root, &config, &options) {
        Ok(report) => render(&report, scan.format, flags.verbose, out, err),
        Err(e) => refuse(err, &format!("xnl: {e}")),
    }
}

/// Write `report` in `format` and return its exit code.
///
/// Human (`src/lint` §I): each result that did not pass as
/// `file: <check>: <why>`, the tool's tail indented under it, then
/// `N checks, N failed` last -- on stdout, and only when something
/// failed or with `--verbose`. Warnings go to stderr. JSON: the envelope
/// on stdout.
pub fn render(
    report: &LintReport,
    format: OutputFormat,
    verbose: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    match format {
        OutputFormat::Json => {
            let _ = out.write_all(report.to_json().as_bytes());
        }
        OutputFormat::Human | OutputFormat::Sarif => {
            let mut ran = 0;
            let mut failed = 0;
            for outcome in report.outcomes() {
                if outcome.status == Status::Skipped {
                    continue;
                }
                ran += 1;
                if outcome.status == Status::Pass {
                    continue;
                }
                failed += 1;
                let role = if outcome.fixer { "fixer " } else { "" };
                let mut tail = outcome.raw_tail.as_deref().unwrap_or_default().lines();
                let why = match (outcome.status, outcome.exit) {
                    (Status::Fail, Some(code)) => format!("failed (exit {code})"),
                    _ => tail.next().unwrap_or("error").to_owned(),
                };
                let _ = writeln!(
                    out,
                    "{}: {role}{}: {why}",
                    outcome.file.display(),
                    outcome.check
                );
                for line in tail {
                    let _ = writeln!(out, "    {line}");
                }
            }
            for warning in report.warnings() {
                let _ = writeln!(err, "{}", warning.to_human());
            }
            if failed > 0 || verbose {
                let _ = writeln!(out, "{ran} checks, {failed} failed");
            }
        }
    }
    report.exit_code()
}
