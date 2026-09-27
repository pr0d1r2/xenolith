//! Pkl as a host: hk steps holding shell (`languages/ci/pkl:T13`).
//!
//! A hk config is a pkl module that amends hk's `Config.pkl`, and every
//! step in it carries commands in `check`, `fix`, `shell`, `check_diff`
//! and `check_list_files` (`languages/ci/pkl` §I). Those five properties,
//! inside a step entry, inside a hk config, are the sink. The same
//! `"""…"""` anywhere else -- a doc comment, a `message`, a top-level
//! `local`, a module that is not a hk config -- is inert data
//! (`languages:V2`, `languages/api/src/site:V38`), and each of those has
//! a negative fixture here (`tests:V15`).
//!
//! Fixtures live in this crate (`tests:V14`), one directory per case.

use std::path::{Path, PathBuf};

use xenolith_lang_api::{DelimKind, GuestEnv, Host, Invoke, LangId, LoadRef, Site};
use xenolith_lang_pkl::{PklHost, unescape};

fn fixture(case: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(case)
        .join("input.pkl");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

fn sites(src: &str) -> Vec<Site> {
    PklHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
}

fn loads(src: &str) -> Vec<LoadRef> {
    PklHost
        .loads(src)
        .unwrap_or_else(|e| panic!("loads failed: {e}"))
}

fn sinks(found: &[Site]) -> Vec<&str> {
    found.iter().map(|site| site.sink.as_str()).collect()
}

fn body(src: &str, site: &Site) -> String {
    let raw = site
        .delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("body span {:?} does not fit", site.delim.body));
    unescape(&site.delim, raw).unwrap_or_else(|e| panic!("unescape failed: {e}"))
}

/// Whitespace-normalized, the comparison `languages/api/src/lens:V34`(a)
/// asks for: indentation is the host's to choose, content is not.
fn normalized(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn the_host_is_pkl() {
    assert_eq!(PklHost.id(), LangId::Pkl);
}

#[test]
fn claims_pkl_files_only() {
    assert!(PklHost.claims(Path::new("hk.pkl"), "amends \"pkl/Config.pkl\""));
    assert!(PklHost.claims(Path::new("config/ci/Matrix.pkl"), ""));
    assert!(PklHost.claims(Path::new("PklProject"), "amends \"pkl:Project\""));
    assert!(!PklHost.claims(Path::new("flake.nix"), "{"));
    assert!(!PklHost.claims(Path::new("hk.pkl.bak"), ""));
    assert!(!PklHost.claims(Path::new("scripts/hk/run-tool.sh"), "#!/usr/bin/env bash"));
}

#[test]
fn every_script_in_a_hk_step_sink_is_a_site() {
    let src = fixture("hk-step-script");
    let found = sites(&src);
    assert_eq!(
        sinks(&found),
        [
            "shellcheck.check",
            "shellcheck.fix",
            "typos.check_list_files"
        ]
    );
    for site in &found {
        assert_eq!(site.guest, LangId::Shell, "{}", site.sink);
        assert!(site.holes.is_empty(), "{} has no `\\(…)`", site.sink);
    }
}

#[test]
fn sites_come_back_sorted_by_span() {
    // `languages/api:V36`: the engine merges hosts' results and must get
    // the same bytes every run.
    let found = sites(&fixture("hk-step-script"));
    let starts: Vec<usize> = found.iter().map(|site| site.delim.open.start).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(starts, sorted);
}

#[test]
fn the_delimiter_comes_from_the_grammar() {
    let src = fixture("hk-step-script");
    let found = sites(&src);
    let [check, fix, _] = found.as_slice() else {
        panic!("expected three sites, got {:?}", sinks(&found));
    };

    assert_eq!(check.delim.kind, DelimKind::PklMultiline { pounds: 0 });
    assert_eq!(check.delim.open.of(&src), Some("\"\"\""));
    assert_eq!(check.delim.close.of(&src), Some("\"\"\""));
    assert_eq!(check.delim.open.end, check.delim.body.start);
    assert_eq!(check.delim.body.end, check.delim.close.start);

    assert_eq!(fix.delim.kind, DelimKind::PklMultiline { pounds: 1 });
    assert_eq!(fix.delim.open.of(&src), Some("#\"\"\""));
    assert_eq!(fix.delim.close.of(&src), Some("\"\"\"#"));
}

#[test]
fn the_body_is_the_string_pkl_would_evaluate() {
    // Leading newline, closing-line indent and the common indent go;
    // everything else stays. Under `#"""` a backslash is literal, so
    // `\t` is two characters in the script, exactly as sed will see it.
    let src = fixture("hk-step-script");
    let found = sites(&src);
    let [check, fix, list] = found.as_slice() else {
        panic!("expected three sites, got {:?}", sinks(&found));
    };
    assert_eq!(
        body(&src, check),
        concat!(
            "if [ -n \"$CI\" ]; then\n",
            "  shellcheck --format=gcc {{files}}\n",
            "else\n",
            "  shellcheck {{files}}\n",
            "fi"
        )
    );
    assert_eq!(
        body(&src, fix),
        "for f in {{files}}; do\n  sed -i 's/\\t/  /g' \"$f\"\ndone"
    );
    assert_eq!(body(&src, list), "typos --files | grep -v '^vendor/'");
}

#[test]
fn escapes_are_resolved_in_a_plain_multiline_string() {
    let src = concat!(
        "amends \"pkl/Config.pkl\"\n",
        "steps {\n",
        "  [\"x\"] {\n",
        "    check = \"\"\"\n",
        "      printf '%s\\t%s\\n' \\\"a\\\" \\u{62}\n",
        "      \"\"\"\n",
        "  }\n",
        "}\n"
    );
    let found = sites(src);
    let [site] = found.as_slice() else {
        panic!("expected one site, got {:?}", sinks(&found));
    };
    assert_eq!(body(src, site), "printf '%s\t%s\n' \"a\" b");
}

#[test]
fn interpolations_are_holes() {
    // `\#(tool)` is pkl, not shell: an extract cannot carry it, which is
    // what makes it a hole (`languages/api/src/holes:V40`).
    let src = fixture("hk-step-holes");
    let found = sites(&src);
    let [site] = found.as_slice() else {
        panic!("expected one site, got {:?}", sinks(&found));
    };
    assert_eq!(site.sink, "format.check");
    assert_eq!(site.delim.kind, DelimKind::PklMultiline { pounds: 1 });
    let holes: Vec<&str> = site
        .holes
        .iter()
        .map(|hole| hole.of(&src).unwrap_or_default())
        .collect();
    assert_eq!(holes, ["\\#(tool)", "\\#(tool)"]);
}

#[test]
fn a_crlf_config_gives_the_sites_and_bodies_of_its_lf_twin() {
    // `languages/ci/pkl:B1`: the same steps saved with CRLF line endings
    // are the same sites holding the same scripts, so the guest's
    // verdict cannot differ. Spans stay on the host's own bytes.
    let crlf = |text: &str| text.replace('\n', "\r\n");
    for case in ["hk-step-script", "hk-step-holes"] {
        let lf = fixture(case);
        let dos = crlf(&lf);
        let (lf_sites, dos_sites) = (sites(&lf), sites(&dos));
        assert_eq!(sinks(&dos_sites), sinks(&lf_sites), "{case}");
        for (a, b) in lf_sites.iter().zip(&dos_sites) {
            assert_eq!(b.delim.kind, a.delim.kind, "{case} {}", a.sink);
            assert_eq!(body(&dos, b), body(&lf, a), "{case} {}", a.sink);
            let raw = a.delim.body.of(&lf).map(crlf);
            assert_eq!(b.delim.body.of(&dos).map(str::to_owned), raw, "{case}");
            let holes = |site: &Site, src: &str| -> Vec<String> {
                site.holes
                    .iter()
                    .map(|hole| crlf(hole.of(src).unwrap_or_default()))
                    .collect()
            };
            assert_eq!(holes(b, &dos), holes(a, &lf), "{case} {}", a.sink);
        }
    }
    for case in ["inert-strings", "not-hk"] {
        assert!(sites(&crlf(&fixture(case))).is_empty(), "{case}");
    }
}

#[test]
fn inert_strings_are_not_sites() {
    // Comments, a non-sink property and a `check` outside any step entry
    // (`languages:V2`).
    assert!(sites(&fixture("inert-strings")).is_empty());
}

#[test]
fn a_module_that_is_not_a_hk_config_has_no_hk_sinks() {
    assert!(sites(&fixture("not-hk")).is_empty());
}

#[test]
fn a_load_is_the_invoke_of_an_extract_with_files_forwarded() {
    let src = fixture("hk-step-loaded");
    let found = loads(&src);
    let paths: Vec<&Path> = found.iter().map(|load| load.path.as_path()).collect();
    assert_eq!(
        paths,
        [
            Path::new("scripts/hk/shellcheck.sh"),
            Path::new("scripts/hk/shellcheck-fix.sh")
        ]
    );
    for load in &found {
        assert_eq!(load.guest, LangId::Shell);
    }
    assert_eq!(
        found.first().and_then(|load| load.span.of(&src)),
        Some("\"bash scripts/hk/shellcheck.sh {{files}}\"")
    );
    // Single-line strings are not multi-line delimiters, so a load is
    // not also a site.
    assert!(sites(&src).is_empty());
}

#[test]
fn a_command_that_is_not_a_load_is_not_one() {
    // `cargo fmt --check` in `hk-step-loaded` runs no extract, and a
    // script body is not a load either: `xnl graph` must not chase them.
    assert_eq!(loads(&fixture("hk-step-loaded")).len(), 2);
    assert!(loads(&fixture("hk-step-script")).is_empty());
    assert!(loads(&fixture("inert-strings")).is_empty());
}

#[test]
fn rewrite_replaces_the_string_with_a_load_forwarding_files() {
    // `languages/ci/pkl` §I: `bash scripts/hk/<name>.sh {{files}}`.
    let src = fixture("hk-step-script");
    let found = sites(&src);
    let Some(check) = found.first() else {
        panic!("no sites");
    };
    let path = Path::new("scripts/hk/shellcheck.sh");
    let invoke = Invoke {
        argv: vec!["bash".to_owned(), path.display().to_string()],
    };
    let rewritten = PklHost
        .rewrite(&src, check, &invoke, path)
        .unwrap_or_else(|e| panic!("rewrite failed: {e}"));
    assert!(
        rewritten
            .contains("    check = \"bash scripts/hk/shellcheck.sh {{files}}\"\n    fix = #\"\"\""),
        "{rewritten}"
    );
}

#[test]
fn lens_laws_hold_for_every_site_without_holes() {
    // `languages/api/src/lens:V34` (a) inline undoes rewrite, (b) the
    // site is gone, (c) the load is there.
    let src = fixture("hk-step-script");
    for site in sites(&src) {
        let name = site.sink.replace(['.', '_'], "-");
        let path = PathBuf::from(format!("scripts/hk/{name}.sh"));
        let invoke = Invoke {
            argv: vec!["bash".to_owned(), path.display().to_string()],
        };
        let rewritten = PklHost
            .rewrite(&src, &site, &invoke, &path)
            .unwrap_or_else(|e| panic!("{}: rewrite failed: {e}", site.sink));

        assert!(
            !sinks(&sites(&rewritten)).contains(&site.sink.as_str()),
            "{}: still a site after rewrite",
            site.sink
        );
        let after = loads(&rewritten);
        let Some(load) = after.iter().find(|load| load.path == path) else {
            panic!("{}: no load of {} after rewrite", site.sink, path.display());
        };

        let inlined = PklHost
            .inline(&rewritten, load, &body(&src, &site))
            .unwrap_or_else(|e| panic!("{}: inline failed: {e}", site.sink));
        assert_eq!(normalized(&inlined), normalized(&src), "{}", site.sink);
    }
}

// --- placement (`languages/ci/pkl:T54`) ------------------------------------

/// `(sink, name, dir)` for every site in `case`.
fn placements(case: &str) -> Vec<(String, String, String)> {
    sites(&fixture(case))
        .iter()
        .map(|site| {
            let at = PklHost
                .placement(site)
                .unwrap_or_else(|e| panic!("{}: no placement: {e}", site.sink));
            (site.sink.clone(), at.name, at.dir)
        })
        .collect()
}

fn row(sink: &str, name: &str) -> (String, String, String) {
    (sink.to_owned(), name.to_owned(), "scripts/hk".to_owned())
}

#[test]
fn placement_names_each_extract_after_its_step_in_scripts_hk() {
    // `languages/ci/pkl:V52`: the step key, whatever the property -- the
    // engine's collision suffix tells `check` and `fix` apart
    // (`src/extract:V47`).
    assert_eq!(
        placements("hk-step-script"),
        [
            row("shellcheck.check", "shellcheck"),
            row("shellcheck.fix", "shellcheck"),
            row("typos.check_list_files", "typos"),
        ]
    );
}

#[test]
fn placement_kebab_cases_the_step_key_and_falls_back_to_the_stem() {
    // `languages/api/src/site:V43`: kebab-case, and a key naming nothing
    // gives `<host_stem>-<sink>`, the stem a `src/extract:V46` template.
    assert_eq!(
        placements("hk-step-names"),
        [
            row("cargo_clippy.check", "cargo-clippy"),
            row("Rust.Fmt.fix", "rust-fmt"),
            row("+++.check", "{host_stem}-check"),
        ]
    );
}

#[test]
fn a_placed_extract_loads_back_with_files_forwarded() {
    // The placement, the guest's invoke and `rewrite` together make the
    // load `languages/ci/pkl:V52` names, and `loads` reads it back.
    let src = fixture("hk-step-script");
    for site in sites(&src) {
        let at = PklHost
            .placement(&site)
            .unwrap_or_else(|e| panic!("{}: no placement: {e}", site.sink));
        let path = PathBuf::from(format!("{}/{}.sh", at.dir, at.name));
        let invoke = Invoke {
            argv: vec!["bash".to_owned(), path.display().to_string()],
        };
        let rewritten = PklHost
            .rewrite(&src, &site, &invoke, &path)
            .unwrap_or_else(|e| panic!("{}: rewrite failed: {e}", site.sink));
        let command = format!("\"bash {} {{{{files}}}}\"", path.display());
        let after = loads(&rewritten);
        assert!(
            after
                .iter()
                .any(|load| load.path == path && load.span.of(&rewritten) == Some(&command)),
            "{}: no load {command} in {after:?}",
            site.sink
        );
    }
}

#[test]
fn inline_picks_pounds_the_body_needs() {
    // A body holding a backslash cannot go back into a plain `"""`
    // without changing what pkl evaluates it to.
    let src = fixture("hk-step-loaded");
    let found = loads(&src);
    let Some(load) = found.first() else {
        panic!("no loads");
    };
    let inlined = PklHost
        .inline(&src, load, "printf 'a\\tb'\necho \"\"\"")
        .unwrap_or_else(|e| panic!("inline failed: {e}"));
    let reparsed = sites(&inlined);
    let [site] = reparsed.as_slice() else {
        panic!("expected one site, got {:?}", sinks(&reparsed));
    };
    assert_eq!(site.delim.kind, DelimKind::PklMultiline { pounds: 1 });
    assert_eq!(body(&inlined, site), "printf 'a\\tb'\necho \"\"\"");
}

#[test]
fn a_site_that_is_not_in_the_source_is_refused() {
    let src = fixture("hk-step-script");
    let found = sites(&src);
    let Some(site) = found.first() else {
        panic!("no sites");
    };
    let invoke = Invoke {
        argv: vec!["bash".to_owned(), "x.sh".to_owned()],
    };
    assert!(
        PklHost
            .rewrite(&fixture("inert-strings"), site, &invoke, Path::new("x.sh"))
            .is_err()
    );
}

// --- the shell hk runs a step under (`languages/ci/pkl:V172`) ------------

fn env(dialect: &str, options: &[&str]) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: options.iter().map(|&option| option.to_owned()).collect(),
    }
}

fn envs(case: &str) -> Vec<(String, GuestEnv)> {
    sites(&fixture(case))
        .into_iter()
        .map(|site| (site.sink, site.env))
        .collect()
}

#[test]
fn a_step_without_a_shell_runs_under_hks_default() {
    // hk runs a step's command as `sh -o errexit -c` (`pkl/Config.pkl`,
    // `Step.shell`): errexit, and nothing the guest's own default would
    // add. `nounset` and `pipefail` in the extract would make it a
    // program the inline step never was (`languages/ci/pkl:B3`).
    let found = envs("hk-step-script");
    assert_eq!(found.len(), 3);
    for (sink, env_found) in found {
        assert_eq!(env_found, env("sh", &["errexit"]), "{sink}");
    }
}

#[test]
fn a_steps_own_shell_or_its_groups_is_the_env() {
    let row =
        |sink: &str, dialect: &str, options: &[&str]| (sink.to_owned(), env(dialect, options));
    assert_eq!(
        envs("hk-step-shell"),
        [
            row("plain.check", "sh", &["errexit"]),
            row("strict.check", "bash", &["errexit", "pipefail"]),
            row("letters.fix", "bash", &["errexit", "nounset"]),
            // A per-OS `Script` is no one shell: hk's default stands.
            row("os.check", "sh", &["errexit"]),
            row("inherits.check", "zsh", &[]),
            row("overrides.check", "sh", &[]),
        ]
    );
}
