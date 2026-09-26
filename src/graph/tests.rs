//! The graph engine: the mirror of `src/graph/mod.rs` (`src:C139`).
//!
//! As `src/check/tests.rs`: most cases drive [`graph_with`] with FAKE
//! hosts and a fake guest, so they hold in every feature subset
//! (`src:V30`) and pin the engine's own rules -- edges, dangling loads,
//! the orphan scan and when it is judged -- rather than any grammar's.
//! One case per real host goes through [`graph`] and the registry,
//! gated on the features it needs. Every tree lives in a [`Sandbox`], and
//! every git these tests run is the sandbox's (`tests:V150`).

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, Guest, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef, Placement,
    Prelude, Result, Site, Span,
};

use super::{Edge, Graph, GraphError, LOADS_UNSUPPORTED, Options, graph_with};
use crate::check::Langs;
use crate::config::{self, Config};
use crate::discover::{Sandbox, write};
use crate::model::Rule;

// ---------------------------------------------------------------------
// fakes
// ---------------------------------------------------------------------

/// A host that claims `*.fake`, one statement per line: `load <path>`
/// is a load of a shell extract, `site <dir>` a site whose placement
/// dir is `<dir>`, and `!` a parse error.
struct FakeHost;

/// A host that claims `*.noload` and cannot report its loads, as nix
/// cannot today.
struct NoLoads;

fn lines(src: &str) -> Result<Vec<(Span, &str)>> {
    let mut out = Vec::new();
    let mut offset = 0;
    for line in src.split_inclusive('\n') {
        let start = offset;
        offset += line.len();
        let text = line.trim_end_matches('\n');
        if text == "!" {
            return Err(Error::parse(LangId::Just, "a fake syntax error"));
        }
        out.push((Span::new(start, start + text.len()), text));
    }
    Ok(out)
}

macro_rules! fake_host_parts {
    ($id:expr) => {
        fn id(&self) -> LangId {
            $id
        }
        fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
            Err(Error::unsupported($id, "rewrite"))
        }
        fn inline(&self, _: &str, _: &LoadRef, _: &str) -> Result<String> {
            Err(Error::unsupported($id, "inline"))
        }
        fn unescape(&self, _: &Delim, raw: &str) -> Result<String> {
            Ok(raw.to_owned())
        }
        fn checks(&self) -> Vec<LintCmd> {
            Vec::new()
        }
        fn fixers(&self) -> Vec<LintCmd> {
            Vec::new()
        }
    };
}

impl Host for FakeHost {
    fake_host_parts!(LangId::Just);

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "fake")
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        Ok(lines(src)?
            .into_iter()
            .filter_map(|(span, text)| {
                let dir = text.strip_prefix("site ")?;
                Some(Site {
                    sink: dir.to_owned(),
                    guest: LangId::Shell,
                    env: GuestEnv::default(),
                    delim: Delim {
                        kind: DelimKind::JustRecipe,
                        open: Span::new(span.start, span.start),
                        body: span,
                        close: Span::new(span.end, span.end),
                    },
                    holes: Vec::new(),
                })
            })
            .collect())
    }

    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        Ok(lines(src)?
            .into_iter()
            .filter_map(|(span, text)| {
                Some(LoadRef {
                    span,
                    path: PathBuf::from(text.strip_prefix("load ")?),
                    guest: LangId::Shell,
                })
            })
            .collect())
    }

    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(Placement {
            name: "x".to_owned(),
            dir: site.sink.clone(),
        })
    }
}

impl Host for NoLoads {
    fake_host_parts!(LangId::Yaml);

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == "noload")
    }

    fn sites(&self, _src: &str) -> Result<Vec<Site>> {
        Ok(Vec::new())
    }

    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Err(Error::unsupported(LangId::Yaml, "loads"))
    }
}

/// Shell, as far as the graph cares: extracts end in `.sh`.
struct FakeShell;

impl Guest for FakeShell {
    fn id(&self) -> LangId {
        LangId::Shell
    }
    fn extension(&self, _: &GuestEnv) -> &'static str {
        "sh"
    }
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec!["bash".to_owned(), path.display().to_string()],
        }
    }
    fn trivial(&self, _: &str) -> Result<bool> {
        Ok(false)
    }
    fn prelude(&self, _: &GuestEnv) -> Prelude {
        Prelude {
            shebang: None,
            strict: None,
        }
    }
    fn executable(&self) -> bool {
        true
    }
    fn checks(&self, _: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self, _: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
}

const HOSTS: &[&dyn Host] = &[&FakeHost, &NoLoads];
const GUESTS: &[&dyn Guest] = &[&FakeShell];

fn fakes() -> Langs<'static> {
    Langs {
        hosts: HOSTS,
        guests: GUESTS,
    }
}

// ---------------------------------------------------------------------
// driving it
// ---------------------------------------------------------------------

fn config(toml: &str) -> Config {
    config::parse(toml).unwrap_or_else(|e| panic!("fixture config parses: {e}"))
}

/// A plain directory holding `files`: for runs that name their paths.
fn tree(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.plain("t");
    for (rel, text) in files {
        write(&root, rel, text);
    }
    root
}

/// A repository with `files` tracked: for whole-tree runs.
fn repo(sandbox: &Sandbox, files: &[(&str, &str)]) -> PathBuf {
    let root = sandbox.repo("r");
    for (rel, text) in files {
        write(&root, rel, text);
    }
    sandbox.run_git(&root, &["add", "."]);
    root
}

fn run(
    sandbox: &Sandbox,
    root: &Path,
    config: &Config,
    paths: &[&str],
) -> std::result::Result<Graph, GraphError> {
    let options = Options {
        paths: paths.iter().map(PathBuf::from).collect(),
        ..Options::default()
    };
    graph_with(root, config, &options, &fakes(), &|| sandbox.git())
}

fn ok(sandbox: &Sandbox, root: &Path, config: &Config, paths: &[&str]) -> Graph {
    run(sandbox, root, config, paths).unwrap_or_else(|e| panic!("expected a graph, got: {e}"))
}

fn rules(graph: &Graph) -> Vec<(String, usize, Rule)> {
    graph
        .report
        .violations()
        .iter()
        .map(|v| (v.file.display().to_string(), v.line, v.rule))
        .collect()
}

fn codes(graph: &Graph) -> Vec<&str> {
    graph
        .report
        .warnings()
        .iter()
        .map(|w| w.code.as_str())
        .collect()
}

// ---------------------------------------------------------------------
// edges (`src/graph:V7`, `src/graph` §I)
// ---------------------------------------------------------------------

#[test]
fn a_load_that_resolves_is_an_edge() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("a.fake", "# top\nload scripts/a.sh\n"),
            ("scripts/a.sh", ""),
        ],
    );
    let graph = ok(&sandbox, &root, &Config::default(), &["a.fake"]);
    assert_eq!(
        graph.edges,
        vec![Edge {
            host: PathBuf::from("a.fake"),
            sink: String::new(),
            line: 2,
            col: 1,
            extract: PathBuf::from("scripts/a.sh"),
            guest: LangId::Shell,
            params: Vec::new(),
        }]
    );
    assert!(graph.report.violations().is_empty());
    assert_eq!(graph.exit_code(), 0);
}

#[test]
fn edges_are_sorted_by_host_then_position() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("b.fake", "load x.sh\n"),
            ("a.fake", "load y.sh\nload x.sh\n"),
            ("x.sh", ""),
            ("y.sh", ""),
        ],
    );
    let graph = ok(&sandbox, &root, &Config::default(), &["b.fake", "a.fake"]);
    let got: Vec<(String, usize, String)> = graph
        .edges
        .iter()
        .map(|e| {
            (
                e.host.display().to_string(),
                e.line,
                e.extract.display().to_string(),
            )
        })
        .collect();
    assert_eq!(
        got,
        vec![
            ("a.fake".to_owned(), 1, "y.sh".to_owned()),
            ("a.fake".to_owned(), 2, "x.sh".to_owned()),
            ("b.fake".to_owned(), 1, "x.sh".to_owned()),
        ]
    );
}

// ---------------------------------------------------------------------
// dangling loads (`src/graph:V7`, `src/graph:V72`)
// ---------------------------------------------------------------------

#[test]
fn a_load_of_a_missing_file_is_dangling_with_every_field() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("sub/a.fake", "\n  \nload gone.sh\n")]);
    let graph = ok(&sandbox, &root, &Config::default(), &["sub/a.fake"]);
    assert!(graph.edges.is_empty(), "{:?}", graph.edges);
    let [v] = graph.report.violations() else {
        panic!("expected one violation: {:?}", graph.report);
    };
    assert_eq!(v.rule, Rule::DanglingLoad);
    assert_eq!(v.file, PathBuf::from("sub/a.fake"));
    assert_eq!((v.line, v.col), (3, 1));
    assert_eq!((v.host, v.guest), (LangId::Just, LangId::Shell));
    assert_eq!(v.sink, "");
    assert_eq!(v.site, DelimKind::ArgvString);
    assert!(v.why.contains("sub/gone.sh"), "{}", v.why);
    assert!(!v.directions.is_empty());
    assert_eq!(graph.exit_code(), 1);
}

#[cfg(unix)]
#[test]
fn a_load_through_a_symlink_is_dangling() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[("a.fake", "load linked/x.sh\n"), ("real/x.sh", "")],
    );
    symlink(root.join("real"), root.join("linked")).unwrap_or_else(|e| panic!("{e}"));
    let graph = ok(&sandbox, &root, &Config::default(), &["a.fake"]);
    assert_eq!(
        rules(&graph),
        vec![("a.fake".to_owned(), 1, Rule::DanglingLoad)]
    );
    let why = graph
        .report
        .violations()
        .first()
        .map(|v| v.why.clone())
        .unwrap_or_default();
    assert!(why.contains("symlink"), "{why}");
}

// ---------------------------------------------------------------------
// orphans (`src/graph:V7`, `src/graph:V50`)
// ---------------------------------------------------------------------

/// A host with a site placing under `scripts`, loading one script of
/// two there, beside files no guest reads and a script outside every
/// root.
fn orphan_tree<'a>() -> Vec<(&'a str, &'a str)> {
    vec![
        ("a.fake", "site scripts\nload scripts/used.sh\n"),
        ("scripts/used.sh", "echo used\n"),
        ("scripts/orphan.sh", "echo nobody\n"),
        ("scripts/SPEC.md", "# spec\n"),
        ("scripts/xenolith.toml", "version = 1\n"),
        ("src/elsewhere.sh", "echo outside every root\n"),
    ]
}

#[test]
fn an_extract_under_a_root_that_no_host_loads_is_an_orphan() {
    let sandbox = Sandbox::new();
    let root = repo(&sandbox, &orphan_tree());
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    assert_eq!(
        rules(&graph),
        vec![("scripts/orphan.sh".to_owned(), 1, Rule::OrphanExtract)]
    );
    let [v] = graph.report.violations() else {
        panic!("one violation");
    };
    assert_eq!(v.col, 1);
    assert_eq!((v.host, v.guest), (LangId::Shell, LangId::Shell));
    assert_eq!(v.site, DelimKind::ArgvString);
    assert!(v.why.contains("scripts/"), "{}", v.why);
    assert!(!v.directions.is_empty());
}

#[test]
fn a_shebang_makes_an_extract_whatever_its_name() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[
            ("a.fake", "site scripts\n"),
            ("scripts/tool", "#!/usr/bin/env bash\necho hi\n"),
            ("scripts/data", "plain text\n"),
        ],
    );
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    assert_eq!(
        rules(&graph),
        vec![("scripts/tool".to_owned(), 1, Rule::OrphanExtract)]
    );
}

#[test]
fn the_layout_root_bounds_the_scan_without_any_site() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[("tools/lost.sh", "echo\n"), ("scripts/free.sh", "echo\n")],
    );
    let mirror = config("version = 1\n[extract]\nlayout = \"mirror\"\nroot = \"tools\"\n");
    let graph = ok(&sandbox, &root, &mirror, &[]);
    assert_eq!(
        rules(&graph),
        vec![("tools/lost.sh".to_owned(), 1, Rule::OrphanExtract)]
    );
    // The default layout places nothing under `root`: no root, no scan.
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
}

#[test]
fn a_rule_path_bounds_the_scan() {
    let sandbox = Sandbox::new();
    let root = repo(
        &sandbox,
        &[("ci/hk-lost.sh", "echo\n"), ("ci/other.sh", "echo\n")],
    );
    let rule =
        config("version = 1\n[[extract.rule]]\nguest = \"shell\"\npath = \"ci/hk-{name}.{ext}\"\n");
    let graph = ok(&sandbox, &root, &rule, &[]);
    assert_eq!(
        rules(&graph),
        vec![("ci/hk-lost.sh".to_owned(), 1, Rule::OrphanExtract)]
    );
}

#[test]
fn an_excluded_extract_is_not_an_orphan() {
    let sandbox = Sandbox::new();
    let root = repo(&sandbox, &orphan_tree());
    let excluded = config(
        "version = 1\n[graph]\nexclude = [{ glob = \"scripts/orphan.sh\", reason = \"kept\" }]\n",
    );
    let graph = ok(&sandbox, &root, &excluded, &[]);
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
}

#[test]
fn a_named_path_run_judges_no_orphan() {
    // Named paths are a partial view: the host loading an extract may be
    // one nobody named (`src/graph` §I, orphan).
    let sandbox = Sandbox::new();
    let root = repo(&sandbox, &orphan_tree());
    let graph = ok(
        &sandbox,
        &root,
        &Config::default(),
        &["a.fake", "scripts/orphan.sh"],
    );
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
}

#[test]
fn a_host_without_loads_is_not_mentioned_where_no_orphan_scan_runs() {
    // `src/graph:B1`: the warning explains a skipped orphan scan, so a
    // named-path run (never scanned) and a tree with no extract root
    // (nothing to scan) stay silent.
    let sandbox = Sandbox::new();
    let mut files = orphan_tree();
    files.push(("b.noload", "anything\n"));
    let root = repo(&sandbox, &files);
    let named = ok(&sandbox, &root, &Config::default(), &["b.noload"]);
    assert!(codes(&named).is_empty(), "{:?}", codes(&named));
    let bare = Sandbox::new();
    let rootless = repo(&bare, &[("b.noload", "anything\n")]);
    let whole = ok(&bare, &rootless, &Config::default(), &[]);
    assert!(codes(&whole).is_empty(), "{:?}", codes(&whole));
}

#[test]
fn a_host_without_loads_stops_the_orphan_scan_and_says_so() {
    let sandbox = Sandbox::new();
    let mut files = orphan_tree();
    files.push(("b.noload", "anything\n"));
    files.push(("c.noload", "anything\n"));
    let root = repo(&sandbox, &files);
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
    // One warning per host language, not per file.
    assert_eq!(codes(&graph), vec![LOADS_UNSUPPORTED]);
    let message = graph
        .report
        .warnings()
        .first()
        .map(|w| w.message.clone())
        .unwrap_or_default();
    assert!(message.contains("yaml"), "{message}");
    assert!(message.contains("orphan"), "{message}");
}

#[test]
fn a_host_that_does_not_parse_stops_the_orphan_scan() {
    let sandbox = Sandbox::new();
    let mut files = orphan_tree();
    files.push(("broken.fake", "!\n"));
    let root = repo(&sandbox, &files);
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    // `[parse] host_errors = "error"` (default): the file is reported,
    // and the loads it may hold are unknown, so no orphan is judged.
    assert_eq!(
        rules(&graph),
        vec![("broken.fake".to_owned(), 1, Rule::HostParseError)]
    );
    let warn = config("version = 1\n[parse]\nhost_errors = \"warn\"\n");
    let graph = ok(&sandbox, &root, &warn, &[]);
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
}

// ---------------------------------------------------------------------
// refusals and the unclaimed (`src:V13`, `src:V57`)
// ---------------------------------------------------------------------

#[test]
fn strict_hosts_refuses_a_file_no_host_claims_and_no_guest_reads() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[("notes.txt", "hi\n"), ("x.sh", "echo\n")]);
    let options = Options {
        paths: vec!["x.sh".into()],
        strict_hosts: true,
    };
    let graph = graph_with(&root, &Config::default(), &options, &fakes(), &|| {
        sandbox.git()
    });
    assert!(graph.is_ok(), "an extract is read by a guest: {graph:?}");
    let options = Options {
        paths: vec!["notes.txt".into()],
        strict_hosts: true,
    };
    let e = graph_with(&root, &Config::default(), &options, &fakes(), &|| {
        sandbox.git()
    })
    .err()
    .unwrap_or_else(|| panic!("notes.txt is unclaimed"));
    assert!(matches!(e, GraphError::Unclaimed { .. }), "{e:?}");
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("notes.txt"), "{e}");
}

#[test]
fn a_missing_named_path_is_refused() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    let e = run(&sandbox, &root, &Config::default(), &["nope.fake"])
        .err()
        .unwrap_or_else(|| panic!("nothing to scan"));
    assert_eq!(e.exit_code(), 2);
    assert!(e.to_string().contains("nope.fake"), "{e}");
}

#[test]
fn a_path_outside_the_root_is_refused() {
    let sandbox = Sandbox::new();
    let root = tree(&sandbox, &[]);
    write(sandbox.path(), "outside.fake", "load x.sh\n");
    let e = run(&sandbox, &root, &Config::default(), &["../outside.fake"])
        .err()
        .unwrap_or_else(|| panic!("outside the root"));
    assert!(matches!(e, GraphError::Outside { .. }), "{e:?}");
    assert_eq!(e.exit_code(), 2);
}

#[cfg(unix)]
#[test]
fn a_symlinked_candidate_is_skipped_with_a_warning() {
    use std::os::unix::fs::symlink;

    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "a.fake", "site scripts\n");
    write(&root, "real.sh", "echo\n");
    std::fs::create_dir_all(root.join("scripts")).unwrap_or_else(|e| panic!("{e}"));
    symlink("../real.sh", root.join("scripts/link.sh")).unwrap_or_else(|e| panic!("{e}"));
    sandbox.run_git(&root, &["add", "."]);
    let graph = ok(&sandbox, &root, &Config::default(), &[]);
    assert!(rules(&graph).is_empty(), "{:?}", rules(&graph));
    assert!(
        codes(&graph).contains(&crate::discover::SYMLINK_SKIPPED),
        "{:?}",
        codes(&graph)
    );
}

// ---------------------------------------------------------------------
// json (`src/graph` §I)
// ---------------------------------------------------------------------

#[test]
fn json_is_the_envelope_with_edges() {
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[("a.fake", "load a.sh\nload gone.sh\n"), ("a.sh", "")],
    );
    let graph = ok(&sandbox, &root, &Config::default(), &["a.fake"]);
    let text = graph.to_json();
    assert_eq!(text, graph.to_json(), "byte-stable");
    assert!(text.ends_with('\n'));
    let value: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    assert_eq!(
        value,
        serde_json::json!({
            "schema": 1,
            "edges": [{
                "host": "a.fake", "sink": "", "line": 1, "col": 1,
                "extract": "a.sh", "guest": "shell", "params": [],
            }],
            "violations": value.get("violations").cloned().unwrap_or_default(),
            "warnings": [],
        })
    );
    let rules: Vec<&str> = value
        .get("violations")
        .and_then(serde_json::Value::as_array)
        .map(|vs| {
            vs.iter()
                .filter_map(|v| v.get("rule").and_then(serde_json::Value::as_str))
                .collect()
        })
        .unwrap_or_default();
    assert_eq!(rules, vec!["dangling-load"]);
}

// ---------------------------------------------------------------------
// real hosts through the registry
// ---------------------------------------------------------------------

#[cfg(all(feature = "lang-pkl", feature = "lang-shell"))]
#[test]
fn a_pkl_hk_step_loading_a_script_is_an_edge_or_dangling() {
    let step = |command: &str| {
        [
            "amends \"package://github.com/jdx/hk/releases/download/\
             v1.2.0/hk@1.2.0#/Config.pkl\"",
            "",
            "hooks {",
            "  [\"pre-commit\"] {",
            "    steps {",
            "      [\"lint\"] {",
            &format!("        check = \"{command}\""),
            "      }",
            "    }",
            "  }",
            "}",
            "",
        ]
        .join("\n")
    };
    let sandbox = Sandbox::new();
    let root = tree(
        &sandbox,
        &[
            ("hk.pkl", &step("bash scripts/lint.sh {{files}}")),
            ("scripts/lint.sh", "echo\n"),
            ("sub/hk.pkl", &step("bash scripts/lint.sh")),
        ],
    );
    let options = Options {
        paths: vec!["hk.pkl".into(), "sub/hk.pkl".into()],
        ..Options::default()
    };
    let graph = super::graph(&root, &Config::default(), &options).unwrap_or_else(|e| panic!("{e}"));
    let extracts: Vec<String> = graph
        .edges
        .iter()
        .map(|e| e.extract.display().to_string())
        .collect();
    assert_eq!(extracts, vec!["scripts/lint.sh"]);
    assert_eq!(
        rules(&graph),
        vec![("sub/hk.pkl".to_owned(), 7, Rule::DanglingLoad)]
    );
}
