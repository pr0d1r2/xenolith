//! The shell host's sinks as fixtures (`languages/shells/shell:T15`,
//! `tests:V14`, `tests:V15`).
//!
//! Each case is a directory under `tests/fixtures/` holding `input.txt`
//! and `sites.txt`: the sites the host must report, one per line, in span
//! order. Cases prefixed `pos-` must report at least one site and cases
//! prefixed `neg-` must report none, so every rule has a flagged case and
//! a clean one.
//!
//! The input is shell kept as `.txt`, not `.sh`: it is test DATA, and a
//! `.sh` file outside `scripts/` is a script `scripts:C13` requires a
//! mirrored bats test for. It carries no shebang, so no host claims it.
//!
//! One line per site, fields separated by ` | `:
//!
//! ```text
//! <sink> | <guest> | <open delim> | <dialect and options | -> | <holes | -> | <body, Rust literal>
//! ```
//!
//! Lines starting with `#` say why the case exists and are ignored.

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, Site};
use xenolith_lang_shell::ShellHost;

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

/// One site in the `sites.txt` line format.
fn render(src: &str, site: &Site) -> String {
    let open = site.delim.open.of(src).unwrap_or("?");
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
    let holes = if holes.is_empty() {
        "-".to_owned()
    } else {
        holes.join(", ")
    };
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
        let actual: Vec<String> = match ShellHost.sites(&src) {
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
