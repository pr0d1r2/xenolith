use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, Guest, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef, Placement,
    Prelude, Result, Shebang, Site, Span,
};

use super::{Edit, ExtractError, Options, Target, extract_with, write};
use crate::check::Langs;
use crate::config::{self, Config};
use crate::discover::{Sandbox, write as put};

// ---------------------------------------------------------------------
// fakes
// ---------------------------------------------------------------------

/// A line-based host. `<sink>=<guest>: <body>` is a site (`{{…}}` in a
/// body is a hole); `<sink>< <argv…>` is a load of its last word; a
/// line `!` does not parse. Each flavour claims its own extension and
/// breaks one law on purpose.
struct Toy {
    ext: &'static str,
    flavour: Flavour,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Flavour {
    /// Every law holds.
    Sound,
    /// `loads` is unsupported.
    NoLoads,
    /// `rewrite` is unsupported.
    NoRewrite,
    /// `inline` loses the body.
    Lossy,
    /// `rewrite` keeps the site and appends the load.
    Sticky,
}

const SOUND: Toy = Toy {
    ext: "toy",
    flavour: Flavour::Sound,
};
const NO_LOADS: Toy = Toy {
    ext: "noloads",
    flavour: Flavour::NoLoads,
};
const NO_REWRITE: Toy = Toy {
    ext: "norewrite",
    flavour: Flavour::NoRewrite,
};
const LOSSY: Toy = Toy {
    ext: "lossy",
    flavour: Flavour::Lossy,
};
const STICKY: Toy = Toy {
    ext: "sticky",
    flavour: Flavour::Sticky,
};

/// Each line of `src` with its byte offset, newline dropped.
fn lines(src: &str) -> Vec<(usize, &str)> {
    let mut offset = 0;
    src.split_inclusive('\n')
        .map(|line| {
            let start = offset;
            offset += line.len();
            (start, line.trim_end_matches('\n'))
        })
        .collect()
}

fn splice(src: &str, at: Span, with: &str) -> Result<String> {
    match (src.get(..at.start), src.get(at.end..)) {
        (Some(before), Some(after)) => Ok(format!("{before}{with}{after}")),
        _ => Err(Error::parse(LangId::Just, "span outside the source")),
    }
}

impl Host for Toy {
    fn id(&self) -> LangId {
        LangId::Just
    }

    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|ext| ext == self.ext)
    }

    fn sites(&self, src: &str) -> Result<Vec<Site>> {
        let mut sites = Vec::new();
        for (start, line) in lines(src) {
            if line == "!" {
                return Err(Error::parse(LangId::Just, "a toy syntax error"));
            }
            let Some((head, body)) = line.split_once(": ") else {
                continue;
            };
            let Some((sink, guest)) = head.split_once('=') else {
                continue;
            };
            let Some(guest) = LangId::from_name(guest) else {
                continue;
            };
            let open = start + sink.len();
            let body_start = start + head.len() + 2;
            let body_end = start + line.len();
            let holes = body
                .match_indices("{{")
                .filter_map(|(at, _)| {
                    let close = body.get(at..)?.find("}}")?;
                    Some(Span::new(body_start + at, body_start + at + close + 2))
                })
                .collect();
            sites.push(Site {
                sink: sink.to_owned(),
                guest,
                env: GuestEnv::default(),
                delim: Delim {
                    kind: DelimKind::JustRecipe,
                    open: Span::new(open, body_start),
                    body: Span::new(body_start, body_end),
                    close: Span::new(body_end, body_end),
                },
                holes,
            });
        }
        Ok(sites)
    }

    fn loads(&self, src: &str) -> Result<Vec<LoadRef>> {
        if self.flavour == Flavour::NoLoads {
            return Err(Error::unsupported(LangId::Just, "loads"));
        }
        let mut loads = Vec::new();
        for (start, line) in lines(src) {
            let Some((sink, argv)) = line.split_once("< ") else {
                continue;
            };
            let Some(path) = argv.split_whitespace().last() else {
                continue;
            };
            loads.push(LoadRef {
                span: Span::new(start + sink.len(), start + line.len()),
                path: PathBuf::from(path),
                guest: LangId::Shell,
            });
        }
        Ok(loads)
    }

    fn rewrite(&self, src: &str, site: &Site, invoke: &Invoke, _path: &Path) -> Result<String> {
        if self.flavour == Flavour::NoRewrite {
            return Err(Error::unsupported(LangId::Just, "rewrite"));
        }
        if !self.sites(src)?.contains(site) {
            return Err(Error::parse(LangId::Just, "no such site"));
        }
        let load = format!("< {}", invoke.argv.join(" "));
        if self.flavour == Flavour::Sticky {
            return Ok(format!("{src}\n{}{load}", site.sink));
        }
        splice(
            src,
            Span::new(site.delim.open.start, site.delim.close.end),
            &load,
        )
    }

    fn inline(&self, src: &str, load: &LoadRef, body: &str) -> Result<String> {
        if !self.loads(src)?.contains(load) {
            return Err(Error::parse(LangId::Just, "no such load"));
        }
        match self.flavour {
            Flavour::Lossy => splice(src, load.span, "=shell: lost"),
            Flavour::Sticky => {
                let line_start = src
                    .get(..load.span.start)
                    .and_then(|before| before.rfind('\n'))
                    .unwrap_or(0);
                splice(src, Span::new(line_start, load.span.end), "")
            }
            _ => splice(
                src,
                load.span,
                &format!("={}: {}", load.guest, body.trim_end_matches('\n')),
            ),
        }
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

    /// Beside the host, in a directory named after it, the sink's dots
    /// as dashes.
    fn placement(&self, site: &Site) -> Result<Placement> {
        Ok(Placement {
            name: site.sink.replace('.', "-"),
            dir: "{host_dir}/{host_stem}".to_owned(),
        })
    }
}

/// A shell-like guest: `&&` or `|` make a body non-trivial.
struct Sh;

impl Guest for Sh {
    fn id(&self) -> LangId {
        LangId::Shell
    }

    fn extension(&self, _: &GuestEnv) -> &'static str {
        "sh"
    }

    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec!["sh".to_owned(), path.display().to_string()],
        }
    }

    fn trivial(&self, body: &str) -> Result<bool> {
        Ok(!body.contains("&&") && !body.contains('|'))
    }

    fn constructs(&self, body: &str) -> Result<Vec<&'static str>> {
        let mut names = Vec::new();
        if body.contains("&&") {
            names.push("and-or");
        }
        if body.contains('|') {
            names.push("pipeline");
        }
        Ok(names)
    }

    fn prelude(&self, _: &GuestEnv) -> Prelude {
        Prelude {
            shebang: Some(Shebang::env("sh")),
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

const HOSTS: &[&dyn Host] = &[&SOUND, &NO_LOADS, &NO_REWRITE, &LOSSY, &STICKY];
const GUESTS: &[&dyn Guest] = &[&Sh];

// ---------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------

fn target(operand: &str) -> Target {
    match operand.rsplit_once(':') {
        Some((path, line)) if line.chars().all(|c| c.is_ascii_digit()) => Target {
            path: PathBuf::from(path),
            line: line.parse().ok(),
        },
        _ => Target {
            path: PathBuf::from(operand),
            line: None,
        },
    }
}

fn try_plan(
    sandbox: &Sandbox,
    root: &Path,
    config: &Config,
    targets: &[&str],
) -> std::result::Result<Edit, ExtractError> {
    let langs = Langs {
        hosts: HOSTS,
        guests: GUESTS,
    };
    let options = Options {
        targets: targets.iter().map(|t| target(t)).collect(),
        strict_hosts: false,
    };
    extract_with(root, config, &options, &langs, &|| sandbox.git())
}

fn plan(sandbox: &Sandbox, root: &Path, targets: &[&str]) -> Edit {
    try_plan(sandbox, root, &Config::default(), targets).unwrap_or_else(|e| panic!("{e}"))
}

fn plan_with(sandbox: &Sandbox, root: &Path, toml: &str, targets: &[&str]) -> Edit {
    let config = config::parse(toml).unwrap_or_else(|e| panic!("{e}"));
    try_plan(sandbox, root, &config, targets).unwrap_or_else(|e| panic!("{e}"))
}

fn read(root: &Path, rel: &str) -> String {
    fs::read_to_string(root.join(rel)).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

fn refusals(edit: &Edit) -> String {
    edit.refusals
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

fn only_refusal(edit: &Edit) -> String {
    assert_eq!(edit.refusals.len(), 1, "{}", refusals(edit));
    assert!(edit.hosts.is_empty(), "{:?}", edit.hosts);
    assert_eq!(edit.exit_code(), 2);
    refusals(edit)
}

// ---------------------------------------------------------------------
// the move (T22)
// ---------------------------------------------------------------------

#[test]
fn a_flagged_site_moves_to_its_own_file_and_the_host_loads_it() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(
        &root,
        "a.toy",
        "build=shell: make && make test\nname=shell: echo hi\n",
    );
    let edit = plan(&sandbox, &root, &["a.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let [host] = edit.hosts.as_slice() else {
        panic!("{:?}", edit.hosts)
    };
    assert_eq!(host.path, "a.toy");
    assert_eq!(
        host.after, "build< sh ./a/build.sh\nname=shell: echo hi\n",
        "the trivial site stays inline"
    );
    let [file] = host.extracts.as_slice() else {
        panic!("{:?}", host.extracts)
    };
    assert_eq!(file.path, "a/build.sh");
    assert_eq!(file.text, "#!/usr/bin/env sh\nmake && make test\n");
    assert!(file.executable);
    assert!(!file.present);
    assert_eq!(edit.exit_code(), 1);
}

#[test]
fn the_diff_announces_each_extract_then_shows_it_before_its_host() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    let diff = plan(&sandbox, &root, &["a.toy"]).diff();
    assert_eq!(
        diff,
        "removing xenolith → a/build.sh\n\
         --- /dev/null\n+++ b/a/build.sh\n@@ -0,0 +1,2 @@\n\
         +#!/usr/bin/env sh\n+make && make test\n\
         --- a/a.toy\n+++ b/a.toy\n@@ -1 +1 @@\n\
         -build=shell: make && make test\n+build< sh ./a/build.sh\n"
    );
}

#[test]
fn nothing_flagged_is_an_empty_edit_and_exit_zero() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "name=shell: echo hi\n");
    put(&root, "notes.md", "not a host\n");
    let edit = plan(&sandbox, &root, &["a.toy", "notes.md"]);
    assert!(
        edit.hosts.is_empty() && edit.refusals.is_empty(),
        "{edit:?}"
    );
    assert!(edit.warnings.is_empty(), "{edit:?}");
    assert_eq!(edit.exit_code(), 0);
    assert_eq!(edit.diff(), "");
}

#[test]
fn write_then_rerun_is_a_no_op() {
    // src/extract:V5: extract(extract(x)) == extract(x).
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    let edit = plan(&sandbox, &root, &["a.toy"]);
    let written = write::apply(&root, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(written, ["a/build.sh", "a.toy"]);
    assert_eq!(read(&root, "a.toy"), "build< sh ./a/build.sh\n");
    assert_eq!(
        read(&root, "a/build.sh"),
        "#!/usr/bin/env sh\nmake && make test\n"
    );
    assert_eq!(plan(&sandbox, &root, &["a.toy"]), Edit::default());
}

#[test]
fn an_extract_already_there_with_the_same_bytes_is_not_rewritten() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    put(
        &root,
        "a/build.sh",
        "#!/usr/bin/env sh\nmake && make test\n",
    );
    let edit = plan(&sandbox, &root, &["a.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let present: Vec<bool> = edit
        .hosts
        .iter()
        .flat_map(|h| &h.extracts)
        .map(|e| e.present)
        .collect();
    assert_eq!(present, [true]);
    assert!(!edit.diff().contains("/dev/null"), "{}", edit.diff());
}

#[test]
fn an_extract_path_holding_other_bytes_is_refused() {
    // src/extract:V6: never overwrite a file with other content.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    put(&root, "a/build.sh", "mine\n");
    let edit = plan(&sandbox, &root, &["a.toy"]);
    let why = only_refusal(&edit);
    assert!(why.starts_with("a.toy:1: a/build.sh exists"), "{why}");
    assert!(why.contains("src/extract:V6"), "{why}");
}

#[test]
fn a_site_with_holes_is_refused_until_params_exist() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: {{cc}} && make\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.toy"]));
    assert!(why.contains("languages/api/src/holes:T76"), "{why}");
}

#[test]
fn a_host_that_cannot_rewrite_is_refused_by_name() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.norewrite", "build=shell: make && make test\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.norewrite"]));
    assert!(why.contains("does not support `rewrite`"), "{why}");
}

#[test]
fn a_host_that_cannot_read_its_loads_cannot_prove_the_rewrite() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.noloads", "build=shell: make && make test\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.noloads"]));
    assert!(why.contains("src/extract:V4"), "{why}");
}

#[test]
fn a_rewrite_that_does_not_inline_back_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.lossy", "build=shell: make && make test\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.lossy"]));
    assert!(why.contains("not lossless (src/extract:V4)"), "{why}");
}

#[test]
fn a_rewrite_leaving_the_site_behind_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.sticky", "build=shell: make && make test\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.sticky"]));
    assert!(why.contains("src/extract:V5"), "{why}");
}

#[test]
fn a_line_extracts_only_its_site_and_one_with_none_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(
        &root,
        "a.toy",
        "one=shell: a && b\ntwo=shell: c && d\nthree=shell: echo\n",
    );
    let edit = plan(&sandbox, &root, &["a.toy:2"]);
    let paths: Vec<&str> = edit
        .hosts
        .iter()
        .flat_map(|h| &h.extracts)
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(paths, ["a/two.sh"]);
    let edit = plan(&sandbox, &root, &["a.toy:3"]);
    let why = only_refusal(&edit);
    assert!(why.starts_with("a.toy:3: no site"), "{why}");
}

#[test]
fn a_line_on_a_directory_or_a_missing_path_refuses_the_run() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "d/a.toy", "one=shell: a && b\n");
    let got = try_plan(&sandbox, &root, &Config::default(), &["d:1"]);
    assert!(matches!(got, Err(ExtractError::LineOnDir(_))), "{got:?}");
    let got = try_plan(&sandbox, &root, &Config::default(), &["nope.toy"]);
    assert!(matches!(got, Err(ExtractError::Discover(_))), "{got:?}");
}

#[test]
fn colliding_paths_take_the_sink_suffix_and_equal_ones_are_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml = "version = 1\n[[extract.rule]]\nguest = \"shell\"\npath = \"tools/{guest}.{ext}\"\n";
    put(
        &root,
        "a.toy",
        "x.script=shell: a && b\ny.preStart=shell: c && d\n",
    );
    let edit = plan_with(&sandbox, &root, toml, &["a.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let paths: Vec<&str> = edit
        .hosts
        .iter()
        .flat_map(|h| &h.extracts)
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(paths, ["tools/shell-prestart.sh", "tools/shell-script.sh"]);
    put(&root, "b.toy", "x.run=shell: a && b\ny.run=shell: c && d\n");
    let edit = plan_with(&sandbox, &root, toml, &["b.toy"]);
    assert_eq!(edit.refusals.len(), 2, "{}", refusals(&edit));
    assert!(
        refusals(&edit).contains("src/extract:V47"),
        "{}",
        refusals(&edit)
    );
}

#[test]
fn a_rule_invoke_is_rendered_with_the_path_as_loaded() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml =
        "version = 1\n[[extract.rule]]\nhost = \"just\"\ninvoke = [\"dash\", \"-e\", \"{path}\"]\n";
    put(&root, "a.toy", "build=shell: make && make test\n");
    let edit = plan_with(&sandbox, &root, toml, &["a.toy"]);
    let after: Vec<&str> = edit.hosts.iter().map(|h| h.after.as_str()).collect();
    assert_eq!(after, ["build< dash -e ./a/build.sh\n"]);
}

#[test]
fn verbose_names_the_layer_behind_every_field() {
    // src/extract:V48.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    let edit = plan(&sandbox, &root, &["a.toy"]);
    assert_eq!(
        edit.explain,
        [
            "a.toy:1:6 build: path = a/build.sh (host)",
            "a.toy:1:6 build: name = build (host)",
            "a.toy:1:6 build: base = host dir (host)",
            "a.toy:1:6 build: invoke = guest default (guest)",
            "a.toy:1:6 build: prelude = guest default (guest)",
            "a.toy:1:6 build: executable = true (guest)",
            "a.toy:1:6 build: companion = none (guest)",
        ]
    );
}

#[test]
fn a_companion_or_an_enforced_prelude_is_refused_for_now() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    for rule in [
        "companion = \"{path_stem}.test.sh\"",
        "prelude = { strict = \"enforce\" }",
    ] {
        let toml = format!("version = 1\n[[extract.rule]]\nhost = \"just\"\n{rule}\n");
        let edit = plan_with(&sandbox, &root, &toml, &["a.toy"]);
        let why = only_refusal(&edit);
        assert!(why.starts_with("a.toy:1: "), "{rule}: {why}");
    }
}

#[test]
fn a_host_that_does_not_parse_is_refused_and_the_rest_proceed() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml = "version = 1\n[parse]\nhost_errors = \"warn\"\n";
    put(&root, "bad.toy", "!\n");
    put(&root, "good.toy", "build=shell: make && make test\n");
    let edit = plan_with(&sandbox, &root, toml, &["bad.toy", "good.toy"]);
    assert_eq!(
        refusals(&edit),
        "bad.toy: did not parse as just: just: parse failed: a toy syntax error"
    );
    let hosts: Vec<&str> = edit.hosts.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(hosts, ["good.toy"]);
}

// ---------------------------------------------------------------------
// many sites in one host (T64)
// ---------------------------------------------------------------------

#[test]
fn sites_are_rewritten_back_to_front_so_every_span_holds() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(
        &root,
        "a.toy",
        "one=shell: a && b\ntwo=shell: a much longer body && c\nthree=shell: d | e\n",
    );
    let edit = plan(&sandbox, &root, &["a.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let after: Vec<&str> = edit.hosts.iter().map(|h| h.after.as_str()).collect();
    assert_eq!(
        after,
        ["one< sh ./a/one.sh\ntwo< sh ./a/two.sh\nthree< sh ./a/three.sh\n"]
    );
}

#[test]
fn one_refused_site_leaves_its_whole_file_untouched_and_others_proceed() {
    // src/extract:V64: all or nothing per file.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let three = "one=shell: a && b\ntwo=shell: {{cc}} && c\nthree=shell: d | e\n";
    put(&root, "a.toy", three);
    put(&root, "b.toy", "four=shell: f && g\n");
    let edit = plan(&sandbox, &root, &["a.toy", "b.toy"]);
    let hosts: Vec<&str> = edit.hosts.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(hosts, ["b.toy"]);
    let why = refusals(&edit);
    assert!(why.contains("a.toy:2: "), "{why}");
    assert!(why.contains("a.toy: "), "{why}");
    assert!(why.contains("src/extract:V64"), "{why}");
    assert_eq!(edit.exit_code(), 2);
    write::apply(&root, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read(&root, "a.toy"), three);
    assert!(!root.join("a/one.sh").exists());
    assert_eq!(read(&root, "b.toy"), "four< sh ./b/four.sh\n");
}

// ---------------------------------------------------------------------
// hosts below the root (T66)
// ---------------------------------------------------------------------

#[test]
fn a_host_in_a_subdirectory_loads_its_extract_relative_to_itself() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "nixos/foo.toy", "build=shell: make && make test\n");
    let edit = plan(&sandbox, &root, &["nixos/foo.toy"]);
    assert!(edit.refusals.is_empty(), "{}", refusals(&edit));
    let got: Vec<(&str, &str)> = edit
        .hosts
        .iter()
        .flat_map(|h| {
            h.extracts
                .iter()
                .map(|e| (e.path.as_str(), h.after.as_str()))
        })
        .collect();
    assert_eq!(got, [("nixos/foo/build.sh", "build< sh ./foo/build.sh\n")]);
}

#[test]
fn an_extract_placed_outside_the_host_directory_is_loaded_through_dot_dot() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml = "version = 1\n[[extract.rule]]\nhost = \"just\"\npath = \"scripts/{name}.{ext}\"\n";
    put(&root, "nixos/foo.toy", "build=shell: make && make test\n");
    let edit = plan_with(&sandbox, &root, toml, &["nixos/foo.toy"]);
    let after: Vec<&str> = edit.hosts.iter().map(|h| h.after.as_str()).collect();
    assert_eq!(after, ["build< sh ../scripts/build.sh\n"]);
}

// ---------------------------------------------------------------------
// what is left alone, and why (T81)
// ---------------------------------------------------------------------

#[test]
fn an_allowed_site_is_untouched_by_write_and_verbose_says_why() {
    // src/extract:V80.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let body = "make && make test";
    let toml = format!(
        "version = 1\n[[allow]]\npath = \"a.toy\"\nsink = \"build\"\nhash = \"{}\"\n\
         reason = \"kept on purpose\"\n",
        crate::check::body_hash(body)
    );
    put(&root, "a.toy", &format!("build=shell: {body}\n"));
    let edit = plan_with(&sandbox, &root, &toml, &["a.toy"]);
    assert!(
        edit.hosts.is_empty() && edit.refusals.is_empty(),
        "{edit:?}"
    );
    assert_eq!(
        edit.explain,
        ["a.toy:1:6 build: skipped: allowed by [[allow]] (kept on purpose) (src/extract:V80)"]
    );
    write::apply(&root, &edit).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read(&root, "a.toy"), format!("build=shell: {body}\n"));
}

#[test]
fn a_trivial_site_is_skipped_as_one_that_may_stay_inline() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "name=shell: echo hi\n");
    let edit = plan(&sandbox, &root, &["a.toy"]);
    assert_eq!(
        edit.explain,
        ["a.toy:1:5 name: skipped: xnl check does not flag it, so it may stay inline"]
    );
}

#[test]
fn a_file_under_an_extract_exclude_is_not_read() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml = "version = 1\n[extract]\nexclude = [{ glob = \"vendor\", reason = \"theirs\" }]\n";
    put(&root, "vendor/a.toy", "build=shell: make && make test\n");
    put(&root, "b.toy", "build=shell: make && make test\n");
    let edit = plan_with(&sandbox, &root, toml, &["vendor/a.toy", "b.toy"]);
    let hosts: Vec<&str> = edit.hosts.iter().map(|h| h.path.as_str()).collect();
    assert_eq!(hosts, ["b.toy"]);
    assert_eq!(
        edit.explain.first().map(String::as_str),
        Some("vendor/a.toy: skipped: excluded by `vendor` (theirs) (src/extract:V80)")
    );
}

#[test]
fn a_file_under_an_exclude_for_every_verb_is_skipped_by_name() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let toml = "version = 1\n[[exclude]]\nglob = \"gen/**\"\nreason = \"generated\"\n";
    put(&root, "gen/a.toy", "build=shell: make && make test\n");
    let edit = plan_with(&sandbox, &root, toml, &["gen/a.toy"]);
    assert!(edit.hosts.is_empty(), "{edit:?}");
    assert_eq!(
        edit.explain,
        ["gen/a.toy: skipped: excluded by `gen/**` (generated) (src/extract:V80)"]
    );
}

// ---------------------------------------------------------------------
// what a path may hold (T84)
// ---------------------------------------------------------------------

#[test]
fn a_rule_template_yielding_a_space_quote_or_leading_dash_is_refused() {
    // src/extract:V83: the path lands inside host syntax.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    for template in [
        "tools/a b/{name}.{ext}",
        "tools/it's/{name}.{ext}",
        "tools/\\\"q\\\"/{name}.{ext}",
        "-x/{name}.{ext}",
        "tools/-{name}.{ext}",
    ] {
        let toml =
            format!("version = 1\n[[extract.rule]]\nhost = \"just\"\npath = \"{template}\"\n");
        let edit = plan_with(&sandbox, &root, &toml, &["a.toy"]);
        let why = only_refusal(&edit);
        assert!(why.contains("src/extract:V83"), "{template}: {why}");
    }
}

#[test]
fn a_host_name_yielding_a_space_is_refused_too() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "my job=shell: make && make test\n");
    let why = only_refusal(&plan(&sandbox, &root, &["a.toy"]));
    assert!(why.contains("`a/my job.sh`"), "{why}");
    assert!(why.contains("src/extract:V83"), "{why}");
}

// ---------------------------------------------------------------------
// never through a symlink (T72)
// ---------------------------------------------------------------------

#[cfg(unix)]
fn link(target: &Path, at: &Path) {
    std::os::unix::fs::symlink(target, at)
        .unwrap_or_else(|e| panic!("symlink {}: {e}", at.display()));
}

#[cfg(unix)]
#[test]
fn an_extract_directory_that_is_a_symlink_inside_the_root_is_refused() {
    // src/extract:V71: never create or write through a symlink, even one
    // that stays in the repository.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build=shell: make && make test\n");
    put(&root, "real/keep", "");
    link(&root.join("real"), &root.join("a"));
    let why = only_refusal(&plan(&sandbox, &root, &["a.toy"]));
    assert!(why.contains("src/extract:V71"), "{why}");
    assert!(!root.join("real/build.sh").exists());
}

#[cfg(unix)]
#[test]
fn an_extract_path_that_is_a_symlink_out_of_the_root_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let outside = sandbox.plain("outside");
    put(&root, "a.toy", "build=shell: make && make test\n");
    put(&root, "a/keep", "");
    // Dangling: reading it fails, and writing through it would create a
    // file outside the repository.
    link(&outside.join("build.sh"), &root.join("a/build.sh"));
    let edit = plan(&sandbox, &root, &["a.toy"]);
    let why = only_refusal(&edit);
    assert!(why.contains("src/extract:V71"), "{why}");
    let _ = write::apply(&root, &edit);
    assert!(!outside.join("build.sh").exists());
}
