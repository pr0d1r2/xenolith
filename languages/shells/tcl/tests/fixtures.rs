//! The tcl host's sinks as fixtures (`languages/shells/tcl:T199`,
//! `tests:V14`, `tests:V15`).
//!
//! Each case is a directory under `tests/fixtures/` holding `input.txt`
//! and `sites.txt`: the sites the host must report, one per line, in span
//! order. Cases prefixed `pos-` must report at least one site and cases
//! prefixed `neg-` must report none, so every rule has a flagged case and
//! a clean one.
//!
//! The input is tcl kept as `.txt`, not `.tcl`: it is test DATA holding
//! embeds on purpose, and a `.tcl` file would be claimed by this very
//! host in the repo's own `xnl check` (`.:V19`). It carries no shebang,
//! so no host claims it.
//!
//! One line per site, fields separated by ` | `, `-` for an empty field:
//!
//! ```text
//! <sink> | <guest> | <open delim> | <dialect and options> | <holes> | <body, Rust literal>
//! ```
//!
//! Lines starting with `#` say why the case exists and are ignored.

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, Site};
use xenolith_lang_tcl::TclHost;

/// Every case directory, sorted, so a failure report reads the same on
/// every machine.
fn cases() -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut dirs: Vec<PathBuf> = fs::read_dir(&root)
        .unwrap_or_else(|e| panic!("{}: {e}", root.display()))
        .filter_map(|entry| entry.ok().map(|e| e.path()))
        .filter(|path| path.is_dir())
        .collect();
    dirs.sort();
    dirs
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// `-` for an empty list or text, so no field is ever blank.
fn or_dash(parts: &[&str]) -> String {
    if parts.iter().all(|p| p.is_empty()) {
        "-".to_owned()
    } else {
        parts.join(", ")
    }
}

/// One site in the `sites.txt` line format.
fn render(src: &str, site: &Site) -> String {
    let open = or_dash(&[site.delim.open.of(src).unwrap_or("?")]);
    let mut env: Vec<&str> = site.env.dialect.iter().map(String::as_str).collect();
    env.extend(site.env.options.iter().map(String::as_str));
    let env = if env.is_empty() {
        "-".to_owned()
    } else {
        env.join(" ")
    };
    let holes: Vec<&str> = site
        .holes
        .iter()
        .map(|hole| hole.of(src).unwrap_or("?"))
        .collect();
    let holes = or_dash(&holes);
    let body = site.delim.body.of(src).unwrap_or("?");
    format!(
        "{} | {} | {open} | {env} | {holes} | {body:?}",
        site.sink, site.guest
    )
}

#[test]
fn every_case_reports_exactly_its_expected_sites() {
    let cases = cases();
    assert!(
        !cases.is_empty(),
        "no fixtures found -- nothing was checked"
    );
    let mut failures = Vec::new();
    for case in &cases {
        let name = case
            .file_name()
            .map_or_else(String::new, |n| n.to_string_lossy().into_owned());
        let src = read(&case.join("input.txt"));
        let expected: Vec<String> = read(&case.join("sites.txt"))
            .lines()
            .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
            .map(str::to_owned)
            .collect();
        let actual: Vec<String> = match TclHost.sites(&src) {
            Ok(sites) => sites.iter().map(|site| render(&src, site)).collect(),
            Err(e) => vec![format!("ERROR {e}")],
        };
        if actual != expected {
            failures.push(format!(
                "{name}\n  expected:\n    {}\n  actual:\n    {}",
                expected.join("\n    "),
                actual.join("\n    ")
            ));
        }
        // `tests:V15`: the prefix is a promise about the case.
        if name.starts_with("pos-") && expected.is_empty() {
            failures.push(format!("{name}: a positive case must expect a site"));
        }
        if name.starts_with("neg-") && !expected.is_empty() {
            failures.push(format!("{name}: a negative case must expect no site"));
        }
        if !name.starts_with("pos-") && !name.starts_with("neg-") {
            failures.push(format!("{name}: a case is `pos-` or `neg-`"));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
