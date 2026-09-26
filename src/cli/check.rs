//! `xnl check`: read `xenolith.toml`, run the engine, render the report
//! (`src:V152`, `src/cli` §I).
//!
//! Everything here is input and output. The engine
//! ([`crate::check`]) decides what is a violation; this module decides
//! only which stream it goes to, in which format, and which exit code
//! that makes (`src/cli:V24`): 0 clean, 1 anything found, 2 refused --
//! a config that does not load, a path that does not exist, a guest
//! compiled out.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::Path;

use super::args::{OutputFormat, Scan};
use super::{EXIT_OK, refuse};
use crate::check::{CONFIG_FILE, Options};
use crate::config::{self, Config};
use crate::model::{Report, Warning};

#[cfg(test)]
mod tests;

/// Run `xnl check` from `root` over `scan`, writing to `out` and `err`;
/// `strict_hosts` is `--strict-hosts` (`src:V13`).
pub fn run(
    root: &Path,
    scan: &Scan,
    verbose: bool,
    strict_hosts: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let config = match load(root) {
        Ok(config) => config,
        Err(message) => return refuse(err, &message),
    };
    let options = Options {
        paths: scan.paths.clone(),
        strict_hosts,
    };
    match crate::check::check(root, &config, &options) {
        Ok(report) => render(&report, scan.format, verbose, out, err),
        Err(e) => refuse(err, &format!("xnl: {e}")),
    }
}

/// The config at `root`, or the defaults when there is none
/// (`src/config:V88`: convention over configuration).
///
/// # Errors
///
/// The message to refuse with: the file exists but cannot be read, or
/// does not parse (`src/config` §V, exit 2).
pub fn load(root: &Path) -> Result<Config, String> {
    let path = root.join(CONFIG_FILE);
    match fs::read_to_string(&path) {
        Ok(text) => config::parse(&text).map_err(|e| format!("xnl: {e}")),
        Err(e) if e.kind() == ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("xnl: {}: {e}", path.display())),
    }
}

/// Write `report` in `format` and return its exit code.
///
/// Human: one line per violation on stdout, warnings on stderr, and with
/// `--verbose` a count line last on stderr; silent when clean. JSON: the
/// envelope on stdout, warnings inside it (`src/cli` §I). Warnings never
/// change the exit code.
pub fn render(
    report: &Report,
    format: OutputFormat,
    verbose: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    // As everywhere in the CLI: a failed write has nowhere to be
    // reported, and the exit code still says what was found.
    match format {
        OutputFormat::Json => {
            let _ = out.write_all(report.to_json().as_bytes());
        }
        OutputFormat::Human | OutputFormat::Sarif => {
            for violation in report.violations() {
                let _ = writeln!(out, "{}", violation.to_human());
            }
            for warning in report.warnings() {
                let _ = writeln!(err, "{}", human_warning(warning));
            }
            if verbose {
                let _ = writeln!(
                    err,
                    "{} violations, {} warnings",
                    report.violations().len(),
                    report.warnings().len()
                );
            }
        }
    }
    if report.violations().is_empty() {
        EXIT_OK
    } else {
        1
    }
}

/// `warning: <code>: <message>`, with the file first when there is one.
fn human_warning(warning: &Warning) -> String {
    match &warning.file {
        Some(file) => format!(
            "{}: warning: {}: {}",
            file.display(),
            warning.code,
            warning.message
        ),
        None => format!("warning: {}: {}", warning.code, warning.message),
    }
}
