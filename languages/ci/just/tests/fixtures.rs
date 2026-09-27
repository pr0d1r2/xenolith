//! The just host's rules as fixtures (`languages/ci/just:T16`,
//! `tests:V14`, `tests:V15`).
//!
//! Each case is a directory under `tests/fixtures/` holding `input.just`
//! and `sites.txt`: the sites the host must report, in span order. Cases
//! prefixed `pos-` must report at least one site and `neg-` none, so every
//! rule has a flagged case and a clean one.
//!
//! One line per site (wrapped here only), fields separated by ` | `:
//!
//! ```text
//! <sink> | <guest> | <delim kind> | <dialect and options | -> | <holes | ->
//!   | <body, unescaped, Rust literal>
//! ```
//!
//! The body is what the guest reads (`languages/api/src/lens:V39`), holes
//! as written. Lines starting with `#` say why the case exists.
//!
//! A case MAY also hold:
//!
//! - `placement.txt`: `<sink> | <name template> | <dir template>` per site;
//! - `loads.txt`: `<path> | <guest> | <load line as written>` per load;
//! - `rewrite.txt`: per site, `<sink> | ok | <load line> | <extract body,
//!   Rust literal>` or `<sink> | refused | <text the refusal contains>` --
//!   the extract direction with invoke `sh ./scripts/just/<sink>.sh`
//!   (`languages/ci/just:V180`).

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Error, Host, Invoke, Site};
use xenolith_lang_just::{JustHost, unescape};

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

fn case_name(case: &Path) -> String {
    case.file_name()
        .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
}

fn dash_or(parts: &[&str], sep: &str) -> String {
    if parts.is_empty() {
        "-".to_owned()
    } else {
        parts.join(sep)
    }
}

/// One site in the `sites.txt` line format.
fn render(src: &str, site: &Site) -> String {
    let mut env: Vec<&str> = site.env.dialect.iter().map(String::as_str).collect();
    env.extend(site.env.options.iter().map(String::as_str));
    let holes: Vec<&str> = site
        .holes
        .iter()
        .map(|hole| hole.of(src).unwrap_or("?"))
        .collect();
    let body = match site
        .delim
        .body
        .of(src)
        .map(|raw| unescape(&site.delim, raw))
    {
        Some(Ok(body)) => format!("{body:?}"),
        Some(Err(e)) => format!("ERROR {e}"),
        None => "?".to_owned(),
    };
    format!(
        "{} | {} | {:?} | {} | {} | {body}",
        site.sink,
        site.guest,
        site.delim.kind,
        dash_or(&env, " "),
        dash_or(&holes, ", "),
    )
}

/// Compare `actual` to the case file `name`, when the case has one.
fn compare(
    case: &Path,
    name: &str,
    actual: impl FnOnce(&str) -> Vec<String>,
    failures: &mut Vec<String>,
) -> bool {
    let file = case.join(name);
    if !file.exists() {
        return false;
    }
    let src = read(&case.join("input.just"));
    let actual = actual(&src);
    let expected = expected(&file);
    if actual != expected {
        failures.push(format!(
            "{}/{name}\n  expected:\n    {}\n  actual:\n    {}",
            case_name(case),
            expected.join("\n    "),
            actual.join("\n    ")
        ));
    }
    true
}

fn sites(src: &str) -> Vec<Site> {
    JustHost
        .sites(src)
        .unwrap_or_else(|e| panic!("sites failed: {e}"))
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
        let expected = expected(&case.join("sites.txt"));
        compare(
            case,
            "sites.txt",
            |src| match JustHost.sites(src) {
                Ok(found) => found.iter().map(|site| render(src, site)).collect(),
                Err(e) => vec![format!("ERROR {e}")],
            },
            &mut failures,
        );
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
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let found = compare(
            &case,
            "placement.txt",
            |src| {
                sites(src)
                    .iter()
                    .map(|site| match JustHost.placement(site) {
                        Ok(at) => format!("{} | {} | {}", site.sink, at.name, at.dir),
                        Err(e) => format!("{} | ERROR {e}", site.sink),
                    })
                    .collect()
            },
            &mut failures,
        );
        checked += usize::from(found);
    }
    assert!(checked > 0, "no placement.txt found -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

#[test]
fn every_loads_file_lists_the_loads_in_span_order() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let found = compare(
            &case,
            "loads.txt",
            |src| match JustHost.loads(src) {
                Ok(loads) => loads
                    .iter()
                    .map(|load| {
                        format!(
                            "{} | {} | {}",
                            load.path.display(),
                            load.guest,
                            load.span.of(src).unwrap_or("?")
                        )
                    })
                    .collect(),
                Err(e) => vec![format!("ERROR {e}")],
            },
            &mut failures,
        );
        checked += usize::from(found);
    }
    assert!(checked > 0, "no loads.txt found -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}

/// The line after the recipe header once rewritten, trimmed: the load.
fn load_line(after: &str, site: &Site) -> String {
    after
        .get(site.delim.open.end..)
        .and_then(|rest| rest.lines().nth(1))
        .map_or_else(|| "?".to_owned(), |line| line.trim().to_owned())
}

/// One site in the `rewrite.txt` line format. A refusal whose operation
/// holds `expected`'s text is written as `expected` is: the file names
/// the part of the reason that matters, not all of it.
fn rewritten(src: &str, site: &Site, expected: Option<&String>) -> String {
    let path = format!("./scripts/just/{}.sh", site.sink);
    let invoke = Invoke {
        argv: vec!["sh".to_owned(), path.clone()],
    };
    let body = site
        .delim
        .body
        .of(src)
        .and_then(|raw| unescape(&site.delim, raw).ok())
        .unwrap_or_default();
    match JustHost.rewrite_bound(src, site, &invoke, Path::new(&path), &body, &[]) {
        Ok(done) => format!(
            "{} | ok | {} | {:?}",
            site.sink,
            load_line(&done.src, site),
            done.body
        ),
        Err(Error::Unsupported { operation, .. }) => {
            match expected.and_then(|e| e.split_once(" | refused | ")) {
                Some((sink, part)) if sink == site.sink && operation.contains(part) => {
                    format!("{sink} | refused | {part}")
                }
                _ => format!("{} | refused | {operation}", site.sink),
            }
        }
        Err(e) => format!("{} | ERROR {e}", site.sink),
    }
}

#[test]
fn every_rewrite_file_says_what_the_extract_direction_does() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for case in cases() {
        let file = case.join("rewrite.txt");
        if !file.exists() {
            continue;
        }
        checked += 1;
        let expected = expected(&file);
        compare(
            &case,
            "rewrite.txt",
            |src| {
                sites(src)
                    .iter()
                    .enumerate()
                    .map(|(i, site)| rewritten(src, site, expected.get(i)))
                    .collect()
            },
            &mut failures,
        );
    }
    assert!(checked > 0, "no rewrite.txt found -- nothing was checked");
    assert!(failures.is_empty(), "{}", failures.join("\n\n"));
}
