//! `docs/THIRD-PARTY-NOTICES.md`, rendered from its owners (`dev:V347`,
//! `docs:V108`).
//!
//! Three owners, each handed in as text by the caller: `cargo metadata`
//! for the crates and their licences, each vendored grammar's `UPSTREAM`
//! record and licence files, and the nix package's `passthru.toolLicenses`
//! for the linters it wraps. The siblings keep this file by hand beside a
//! "reproduce these numbers" section; generating it is the same claim with
//! the runner attached.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde_json::Value;

use crate::badge::shipped;

#[cfg(test)]
mod tests;

/// One vendored grammar directory, read by the caller.
#[derive(Debug, Clone, Default)]
pub struct Vendored {
    /// Repo-relative directory, `languages/ci/pkl/vendor/tree-sitter-pkl`.
    pub dir: String,
    /// The `UPSTREAM` record.
    pub upstream: String,
    /// Licence and notice file names in the directory, sorted.
    pub files: Vec<String>,
    /// `NOTICE.txt`, verbatim, when upstream ships one.
    pub notice: Option<String>,
}

/// One third-party crate as the notices list it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Crate {
    /// Crate name.
    pub name: String,
    /// Exact version.
    pub version: String,
    /// The SPDX expression its manifest declares.
    pub license: String,
    /// A proc-macro runs inside the compiler and ships nothing.
    pub proc_macro: bool,
}

/// One field of an `UPSTREAM` record: `key:` at column 0, continued by
/// indented lines until a blank line or the next field.
#[must_use]
pub fn upstream_field(text: &str, key: &str) -> Option<String> {
    let label = format!("{key}:");
    let mut lines = text.lines();
    let first = lines.find_map(|l| l.strip_prefix(label.as_str()))?;
    let mut value = first.trim().to_string();
    for l in lines {
        if l.trim().is_empty() || !l.starts_with(' ') {
            break;
        }
        value.push(' ');
        value.push_str(l.trim());
    }
    Some(value)
}

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v.get(key).and_then(Value::as_str).unwrap_or_default()
}

/// One resolved edge: the package it reaches, and whether it is a normal
/// and/or a build dependency.
type Edge<'a> = (&'a str, bool, bool);

fn edges_of(node: &Value) -> Vec<Edge<'_>> {
    let deps = node.get("deps").and_then(Value::as_array);
    deps.into_iter()
        .flatten()
        .map(|d| {
            let kinds = d.get("dep_kinds").and_then(Value::as_array);
            let kinds: Vec<&Value> = kinds
                .into_iter()
                .flatten()
                .map(|k| k.get("kind").unwrap_or(&Value::Null))
                .collect();
            let normal = kinds.iter().any(|k| k.is_null());
            let build = kinds.iter().any(|k| k.as_str() == Some("build"));
            (text(d, "pkg"), normal, build)
        })
        .collect()
}

/// Every package reachable from `start` over normal edges, and over build
/// edges too when `build_too`.
fn reach<'a>(
    start: Vec<&'a str>,
    edges: &BTreeMap<&'a str, Vec<Edge<'a>>>,
    build_too: bool,
) -> BTreeSet<&'a str> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut todo = start;
    while let Some(id) = todo.pop() {
        if !seen.insert(id) {
            continue;
        }
        for (pkg, normal, build) in edges.get(id).into_iter().flatten() {
            if *normal || (build_too && *build) {
                todo.push(pkg);
            }
        }
    }
    seen
}

fn crate_of(p: &Value) -> Crate {
    let targets = p.get("targets").and_then(Value::as_array);
    Crate {
        name: text(p, "name").to_string(),
        version: text(p, "version").to_string(),
        license: p
            .get("license")
            .and_then(Value::as_str)
            .unwrap_or("(none declared)")
            .to_string(),
        proc_macro: targets.into_iter().flatten().any(|t| {
            let kinds = t.get("kind").and_then(Value::as_array);
            kinds
                .into_iter()
                .flatten()
                .any(|k| k.as_str() == Some("proc-macro"))
        }),
    }
}

/// The runtime and build-only third-party crates of the SHIPPED workspace
/// members (every member whose `publish` is not `[]`).
///
/// Runtime = reachable from a shipped member over NORMAL edges. Build-only =
/// reachable from a runtime package's `build` edge, over normal and build
/// edges, and not runtime: `cc` compiles the vendored C and ships nothing.
///
/// # Errors
/// Metadata that is not `cargo metadata`'s JSON.
pub fn closure(metadata: &str) -> Result<(Vec<Crate>, Vec<Crate>), String> {
    let bad = |what: &str| format!("xenolith-dev: cargo metadata has no {what} (dev:V347)");
    let meta: Value = serde_json::from_str(metadata)
        .map_err(|e| format!("xenolith-dev: cargo metadata is not JSON: {e}"))?;
    let packages: BTreeMap<&str, &Value> = meta
        .get("packages")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("`packages`"))?
        .iter()
        .map(|p| (text(p, "id"), p))
        .collect();
    let edges: BTreeMap<&str, Vec<Edge<'_>>> = meta
        .pointer("/resolve/nodes")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("`resolve.nodes`"))?
        .iter()
        .map(|n| (text(n, "id"), edges_of(n)))
        .collect();
    let shipped: Vec<&str> = meta
        .get("workspace_members")
        .and_then(Value::as_array)
        .ok_or_else(|| bad("`workspace_members`"))?
        .iter()
        .filter_map(Value::as_str)
        .filter(|id| {
            let publish = packages.get(id).and_then(|p| p.get("publish"));
            shipped(publish.map(Value::to_string).as_deref())
        })
        .collect();
    let runtime = reach(shipped, &edges, false);
    let build_roots: Vec<&str> = runtime
        .iter()
        .flat_map(|id| edges.get(id).into_iter().flatten())
        .filter(|(_, _, build)| *build)
        .map(|(pkg, _, _)| *pkg)
        .collect();
    let build: BTreeSet<&str> = reach(build_roots, &edges, true)
        .difference(&runtime)
        .copied()
        .collect();
    let listed = |ids: &BTreeSet<&str>| -> Vec<Crate> {
        let mut out: Vec<Crate> = ids
            .iter()
            .filter_map(|id| packages.get(id))
            .filter(|p| p.get("source").is_some_and(|s| !s.is_null()))
            .map(|p| crate_of(p))
            .collect();
        out.sort();
        out
    };
    Ok((listed(&runtime), listed(&build)))
}

/// One tool the nix package wraps, as `passthru.toolLicenses` reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tool {
    /// The command `xnl` runs.
    pub command: String,
    /// The nixpkgs package that provides it.
    pub package: String,
    /// Its version.
    pub version: String,
    /// Its SPDX licence ids.
    pub licenses: Vec<String>,
}

/// The tool table, sorted by command.
///
/// # Errors
/// Text that is not the `{command: {package, version, licenses}}` object
/// `nix eval --json .#default.toolLicenses` prints.
pub fn tools(json: &str) -> Result<Vec<Tool>, String> {
    let v: Value = serde_json::from_str(json)
        .map_err(|e| format!("xenolith-dev: the nix tool table is not JSON: {e}"))?;
    let map = v.as_object().ok_or_else(|| {
        "xenolith-dev: the nix tool table is not an object of commands (dev:V347)".to_string()
    })?;
    Ok(map
        .iter()
        .map(|(command, t)| Tool {
            command: command.clone(),
            package: text(t, "package").to_string(),
            version: text(t, "version").to_string(),
            licenses: t
                .get("licenses")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect(),
        })
        .collect())
}

fn crate_table(s: &mut String, crates: &[Crate]) {
    s.push_str("| crate | version | licence |\n|---|---|---|\n");
    for c in crates {
        let note = if c.proc_macro { " (proc-macro)" } else { "" };
        let _ = writeln!(
            s,
            "| `{}`{note} | {} | `{}` |",
            c.name, c.version, c.license
        );
    }
}

fn licence_summary(s: &mut String, crates: &[Crate]) {
    let mut by: BTreeMap<&str, usize> = BTreeMap::new();
    for c in crates {
        *by.entry(c.license.as_str()).or_default() += 1;
    }
    let mut rows: Vec<(&str, usize)> = by.into_iter().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
    s.push_str("| licence | crates |\n|---|---|\n");
    for (licence, n) in rows {
        let _ = writeln!(s, "| `{licence}` | {n} |");
    }
}

fn names(crates: &[&Crate]) -> String {
    crates
        .iter()
        .map(|c| format!("`{}`", c.name))
        .collect::<Vec<_>>()
        .join(", ")
}

const HEAD: &str = "\
# Third-party notices

<!-- GENERATED by `xenolith-dev notices` (dev:V347) from `cargo metadata`, the
vendored grammars' UPSTREAM records and the nix package's tool table. Edit
dev/src/notices.rs, never this file: `xenolith-dev notices --check` is a gate
step, and a hand edit is the drift it reports. -->

`xnl` carries three kinds of other people's work: Rust crates compiled into
the binary, tree-sitter grammars vendored as C source, and -- in the nix
package only -- the linters it runs. Each is listed below with the licence
its owner declares. Nothing here is typed by hand: the file is regenerated
from those three sources, and the gate fails when it drifts from them.
";

const TAIL: &str = "\
## Trademarks

Nominative use only; no affiliation or endorsement is implied.

- **Rust** and **Cargo** are trademarks of the Rust Foundation.
- **NixOS** and **Nix** are trademarks of the NixOS Foundation.
- **GitHub** is a trademark of GitHub, Inc.
- **Claude** and **Anthropic** are trademarks of Anthropic PBC.

## `xenolith` itself

Everything not covered above is licensed under the MIT License -- see
[`LICENSE`](../LICENSE).
";

fn crates_section(s: &mut String, runtime: &[Crate], build: &[Crate]) {
    let _ = write!(
        s,
        "\n## Crates compiled into `xnl`\n\n\
         **{} third-party crates** are in the normal-dependency closure of the\n\
         published crates -- `xenolith` and its language crates, every `lang-*`\n\
         feature on. Dev-dependencies are excluded: they are not distributed in\n\
         anything you run. So is `xenolith-dev`, which is never published.\n\n",
        runtime.len()
    );
    licence_summary(s, runtime);
    s.push_str(
        "\nWhere an expression offers a choice, `xenolith` is distributed under MIT\n\
         and takes the MIT option.\n",
    );
    let and: Vec<&Crate> = runtime
        .iter()
        .filter(|c| c.license.contains(" AND "))
        .collect();
    if !and.is_empty() {
        let _ = write!(
            s,
            "\n**An `AND` is not a choice.** Every part of an expression joined by `AND`\n\
             applies, whichever option is taken for the rest: {}.\n",
            names(&and)
        );
    }
    let macros: Vec<&Crate> = runtime.iter().filter(|c| c.proc_macro).collect();
    if !macros.is_empty() {
        let _ = write!(
            s,
            "\nProc-macro crates ({}) run inside the compiler and put no code of their\n\
             own in the binary. They are listed because cargo resolves them as\n\
             normal dependencies all the same.\n",
            names(&macros)
        );
    }
    s.push('\n');
    crate_table(s, runtime);
    if !build.is_empty() {
        let _ = write!(
            s,
            "\n## Build-time only\n\n\
             **{} crates** run only while compiling -- they build the C sources of the\n\
             grammars -- and none of their code is in `xnl`.\n\n",
            build.len()
        );
        crate_table(s, build);
    }
}

fn grammar_section(s: &mut String, vendored: &[Vendored]) {
    if vendored.is_empty() {
        return;
    }
    let _ = write!(
        s,
        "\n## Vendored tree-sitter grammars\n\n\
         **{} grammars** are compiled from C vendored beside the crate that uses\n\
         them (`languages:V121`), because none is on crates.io in a form this\n\
         workspace can link. Each directory keeps upstream's licence file verbatim\n\
         and an `UPSTREAM` record of where it came from and what, if anything, was\n\
         patched.\n",
        vendored.len()
    );
    for v in vendored {
        let name = v.dir.rsplit('/').next().unwrap_or(&v.dir);
        let field = |key: &str| upstream_field(&v.upstream, key);
        let _ = write!(s, "\n### `{name}`\n\n");
        let _ = writeln!(s, "- vendored in [`{}`](../{}/UPSTREAM)", v.dir, v.dir);
        if let Some(repo) = field("repo") {
            let rev = field("rev").unwrap_or_default();
            let rev = rev.split_whitespace().next().unwrap_or("unrecorded");
            let _ = writeln!(s, "- upstream: <{repo}> at `{rev}`");
        }
        if let Some(licence) = field("license") {
            let _ = writeln!(s, "- licence: {licence}");
        }
        for f in &v.files {
            let _ = writeln!(s, "- [`{f}`](../{}/{f}), verbatim", v.dir);
        }
        if let Some(notice) = &v.notice {
            let _ = write!(
                s,
                "\nThe `NOTICE.txt` upstream ships with it, reproduced as its licence\n\
                 asks:\n\n```text\n{}\n```\n",
                notice.trim_end()
            );
        }
    }
}

fn tool_section(s: &mut String, tools: &[Tool]) {
    if tools.is_empty() {
        return;
    }
    s.push_str(
        "\n## Tools the nix package runs\n\n\
         `packages.default` wraps `xnl` with the linters of its compiled-in\n\
         languages on `PATH` (`nix:V96`). They are separate programs `xnl` runs,\n\
         never linked into it -- aggregation, not a derived work -- and each keeps\n\
         its own licence. A cargo build ships none of them.\n\n\
         | command | nixpkgs package | version | licence |\n|---|---|---|---|\n",
    );
    for t in tools {
        let licences = if t.licenses.is_empty() {
            "(none declared)".to_string()
        } else {
            t.licenses
                .iter()
                .map(|l| format!("`{l}`"))
                .collect::<Vec<_>>()
                .join(", ")
        };
        let _ = writeln!(
            s,
            "| `{}` | {} | {} | {licences} |",
            t.command, t.package, t.version
        );
    }
}

/// The whole file.
///
/// # Errors
/// Either JSON input not in the shape its owner prints.
pub fn render(metadata: &str, vendored: &[Vendored], tools_json: &str) -> Result<String, String> {
    let (runtime, build) = closure(metadata)?;
    let tools = tools(tools_json)?;
    let mut s = String::from(HEAD);
    crates_section(&mut s, &runtime, &build);
    grammar_section(&mut s, vendored);
    tool_section(&mut s, &tools);
    s.push('\n');
    s.push_str(TAIL);
    Ok(s)
}
