//! The sink matrix as fixtures (`languages/nix:T12`, `tests:V14`,
//! `tests:V15`).
//!
//! Each case is a directory under `tests/fixtures/` holding `input.nix`
//! and `sites.txt`: the sites the host must report, one per line, in span
//! order. Cases prefixed `pos-` must report at least one site and cases
//! prefixed `neg-` must report none -- so every rule has a flagged case
//! and a clean one, and the prefix makes a missing half visible in a
//! directory listing.
//!
//! `sites.txt` is a HOST-level expectation, not the `expected.json` /
//! `expected/` pair of `tests:V67`: that pair is `xnl check` output and
//! the post-extract tree, which need the engines (`tests:T68`). What this
//! crate can promise on its own is which strings are sites, and that is
//! what is pinned here.
//!
//! One line per site, fields separated by ` | `:
//!
//! ```text
//! <sink> | <open delim> | <dialect and options | -> | <holes | -> | <body, Rust literal>
//! ```
//!
//! Lines starting with `#` say why the case exists and are ignored.
//!
//! A case MAY also hold `placement.txt`: where the host would put the
//! extract of each site, in the same order (`languages/nix:V53`,
//! `languages/nix:T55`), one line per site:
//!
//! ```text
//! <sink> | <name template> | <dir template>
//! ```
//!
//! And `loads.txt`: the loads `loads` must report, in span order
//! (`languages/nix:V53`):
//!
//! ```text
//! <path> | <guest> | <load expression as written, whitespace collapsed>
//! ```

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Host, Site};
use xenolith_lang_nix::NixHost;

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

/// The expectation lines of a case file: comments and blank lines gone.
fn expected(path: &Path) -> Vec<String> {
    read(path)
        .lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

/// The case directory's own name.
fn case_name(case: &Path) -> String {
    case.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
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
    format!("{} | {open} | {env} | {holes} | {body:?}", site.sink)
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
        let name = case_name(case);
        let src = read(&case.join("input.nix"));
        let expected = expected(&case.join("sites.txt"));
        let actual: Vec<String> = match NixHost.sites(&src) {
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

#[test]
fn every_placement_file_names_where_each_site_goes() {
    // `languages/nix:V53`: the name is cut from the attribute path, the
    // dir sits beside the host file -- both `src/extract:V46` templates.
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let file = case.join("placement.txt");
        if !file.exists() {
            continue;
        }
        checked += 1;
        let src = read(&case.join("input.nix"));
        let actual: Vec<String> = match NixHost.sites(&src) {
            Ok(sites) => sites
                .iter()
                .map(|site| match NixHost.placement(site) {
                    Ok(at) => format!("{} | {} | {}", site.sink, at.name, at.dir),
                    Err(e) => format!("{} | ERROR {e}", site.sink),
                })
                .collect(),
            Err(e) => vec![format!("ERROR {e}")],
        };
        let expected = expected(&file);
        if actual != expected {
            failures.push(format!(
                "{}\n  expected:\n    {}\n  actual:\n    {}",
                case_name(&case),
                expected.join("\n    "),
                actual.join("\n    ")
            ));
        }
    }
    assert!(checked > 0, "no placement.txt found -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn every_loads_file_lists_the_loads_in_span_order() {
    // `languages/nix:V53`: the calls an extraction leaves behind, which
    // `xnl graph` follows to the extract.
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let file = case.join("loads.txt");
        if !file.exists() {
            continue;
        }
        checked += 1;
        let src = read(&case.join("input.nix"));
        let actual: Vec<String> = match NixHost.loads(&src) {
            Ok(loads) => loads
                .iter()
                .map(|load| {
                    format!(
                        "{} | {} | {}",
                        load.path.display(),
                        load.guest,
                        load.span.of(&src).map_or_else(
                            || "?".to_owned(),
                            |text| text.split_whitespace().collect::<Vec<_>>().join(" ")
                        )
                    )
                })
                .collect(),
            Err(e) => vec![format!("ERROR {e}")],
        };
        let expected = expected(&file);
        if actual != expected {
            failures.push(format!(
                "{}\n  expected:\n    {}\n  actual:\n    {}",
                case_name(&case),
                expected.join("\n    "),
                actual.join("\n    ")
            ));
        }
    }
    assert!(checked > 0, "no loads.txt found -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
