//! `xnl graph` in the CLI: the mirror of `src/cli/graph.rs`
//! (`src:C139`).
//!
//! What is pinned: which stream each part of a graph lands on, the exit
//! code it makes (`src/cli:V24`), the human line and summary shape
//! (`src/cli` §I human), and that a config which does not load or a path
//! that does not exist is a refusal. The engine's rules are
//! `src/graph/tests.rs`'s.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{DelimKind, LangId};

use super::{render, run};
use crate::cli::args::{OutputFormat, Scan};
use crate::discover::{Sandbox, write};
use crate::graph::{Edge, Graph};
use crate::model::{Direction, Fix, Report, Rule, Violation, Warning};

fn edge() -> Edge {
    Edge {
        host: PathBuf::from("hk.pkl"),
        sink: String::new(),
        line: 7,
        col: 17,
        extract: PathBuf::from("scripts/a.sh"),
        guest: LangId::Shell,
        params: Vec::new(),
    }
}

fn graph(dangling: usize, warn: bool) -> Graph {
    let mut report = Report::new();
    for n in 0..dangling {
        report.push(Violation {
            rule: Rule::DanglingLoad,
            file: PathBuf::from("hk.pkl"),
            line: n + 1,
            col: 3,
            host: LangId::Pkl,
            guest: LangId::Shell,
            sink: String::new(),
            site: DelimKind::ArgvString,
            why: "the load of `x.sh` does not resolve: x.sh does not exist".to_owned(),
            directions: vec![Direction {
                kind: Fix::Judgment,
                action: "restore it".to_owned(),
            }],
        });
    }
    if warn {
        report.warn(Warning {
            code: "loads-unsupported".to_owned(),
            file: None,
            message: "nix cannot report its loads".to_owned(),
        });
    }
    Graph {
        edges: vec![edge()],
        report,
    }
}

fn rendered(graph: &Graph, format: OutputFormat, verbose: bool) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = render(graph, format, verbose, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn ran(root: &Path, paths: &[&str], format: OutputFormat) -> (u8, String, String) {
    let scan = Scan {
        format,
        paths: paths.iter().map(PathBuf::from).collect(),
    };
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(root, &scan, false, false, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn human_puts_violations_and_the_summary_on_stdout() {
    let (code, out, err) = rendered(&graph(2, true), OutputFormat::Human, false);
    assert_eq!(code, 1);
    assert_eq!(
        out,
        "hk.pkl:1:3 dangling-load: the load of `x.sh` does not resolve: x.sh does not exist\n\
         hk.pkl:2:3 dangling-load: the load of `x.sh` does not resolve: x.sh does not exist\n\
         1 edges, 2 violations\n"
    );
    assert_eq!(
        err,
        "warning: loads-unsupported: nix cannot report its loads\n"
    );
}

#[test]
fn human_is_silent_on_success_unless_verbose() {
    let (code, out, err) = rendered(&graph(0, false), OutputFormat::Human, false);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
    let (code, out, _) = rendered(&graph(0, false), OutputFormat::Human, true);
    assert_eq!((code, out.as_str()), (0, "1 edges, 0 violations\n"));
}

#[test]
fn json_is_the_graph_envelope_on_stdout_only() {
    let g = graph(1, true);
    let (code, out, err) = rendered(&g, OutputFormat::Json, true);
    assert_eq!(code, 1);
    assert_eq!(out, g.to_json());
    assert!(err.is_empty(), "{err:?}");
}

#[test]
fn a_config_that_does_not_parse_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, "xenolith.toml", "version = 99\n");
    write(&root, "a.sh", "echo\n");
    let (code, out, err) = ran(&root, &["a.sh"], OutputFormat::Human);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.starts_with("xnl: "), "{err:?}");
}

#[test]
fn a_missing_path_is_the_engines_refusal() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let (code, out, err) = ran(&root, &["nope.pkl"], OutputFormat::Json);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("nope.pkl"), "{err:?}");
}

#[test]
fn a_named_extract_alone_is_clean_and_silent() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, "a.sh", "echo\n");
    let (code, out, err) = ran(&root, &["a.sh"], OutputFormat::Human);
    assert_eq!((code, out.as_str(), err.as_str()), (0, "", ""));
}
