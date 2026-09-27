//! `xnl graph`: read `xenolith.toml`, run the graph engine, render the
//! graph (`src/graph:V7`, `src/cli` §I).
//!
//! Input and output only, as `xnl check`'s module is: the engine
//! ([`crate::graph`]) decides what is an edge, a dangling load or an
//! orphan; this module decides the stream, the format and the exit code
//! -- 0 clean, 1 a violation, 2 refused (`src/cli:V24`).

use std::io::Write;
use std::path::Path;

use super::args::{OutputFormat, Scan};
use super::{EXIT_OK, refuse};
use crate::graph::{Graph, Options};

#[cfg(test)]
mod tests;

/// Run `xnl graph` from `root` over `scan`, writing to `out` and `err`;
/// `strict_hosts` is `--strict-hosts` (`src/check:V13`).
pub fn run(
    root: &Path,
    scan: &Scan,
    verbose: bool,
    strict_hosts: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    let config = match super::check::load(root) {
        Ok(config) => config,
        Err(message) => return refuse(err, &message),
    };
    let options = Options {
        paths: scan.paths.clone(),
        strict_hosts,
    };
    match crate::graph::graph(root, &config, &options) {
        Ok(graph) => render(&graph, scan.format, verbose, out, err),
        Err(e) => refuse(err, &format!("xnl: {e}")),
    }
}

/// Write `graph` in `format` and return its exit code.
///
/// Human (`src/cli` §I): one `file:line:col <rule>: <why>` line per
/// violation, then `N edges, N violations` last -- on stdout, and only
/// when something was found or with `--verbose`. Warnings go to stderr.
/// JSON: the envelope with `edges` on stdout (`src/graph` §I).
pub fn render(
    graph: &Graph,
    format: OutputFormat,
    verbose: bool,
    out: &mut impl Write,
    err: &mut impl Write,
) -> u8 {
    // As everywhere in the CLI: a failed write has nowhere to be
    // reported, and the exit code still says what was found.
    let violations = graph.report.violations();
    match format {
        OutputFormat::Json => {
            let _ = out.write_all(graph.to_json().as_bytes());
        }
        OutputFormat::Human | OutputFormat::Sarif => {
            for v in violations {
                let _ = writeln!(
                    out,
                    "{}:{}:{} {}: {}",
                    v.file.display(),
                    v.line,
                    v.col,
                    v.rule.as_str(),
                    v.why
                );
            }
            for warning in graph.report.warnings() {
                let _ = writeln!(err, "{}", warning.to_human());
            }
            if verbose || !violations.is_empty() {
                let _ = writeln!(
                    out,
                    "{} edges, {} violations",
                    graph.edges.len(),
                    violations.len()
                );
            }
        }
    }
    if violations.is_empty() { EXIT_OK } else { 1 }
}
