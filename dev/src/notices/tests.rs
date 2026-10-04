//! The notices: the mirror of `dev/src/notices.rs` (`src:C139`).
//!
//! The metadata fixture is `cargo metadata`'s shape cut to what the closure
//! reads: a shipped root, an unpublished dev member whose dependency must
//! not appear, a normal chain, a dev-only edge, a build edge whose own
//! dependency is build-only, a proc-macro and an `AND` licence.

use super::{Crate, Tool, Vendored, closure, render, tools, upstream_field};
use crate::badge::shipped;

#[test]
fn metadata_publish_values_use_the_badge_shipped_rule() {
    assert!(!shipped(Some("[]")));
    assert!(!shipped(Some("false")));
    assert!(shipped(None));
}

const META: &str = r#"{
  "packages": [
    {"id": "root", "name": "xenolith", "version": "0.1.0", "license": "MIT",
     "source": null, "publish": null, "targets": []},
    {"id": "dev", "name": "xenolith-dev", "version": "0.1.0", "license": "MIT",
     "source": null, "publish": [], "targets": []},
    {"id": "json", "name": "serde_json", "version": "1.0.1",
     "license": "MIT OR Apache-2.0", "source": "registry", "publish": null,
     "targets": [{"kind": ["lib"]}]},
    {"id": "ident", "name": "unicode-ident", "version": "1.0.2",
     "license": "(MIT OR Apache-2.0) AND Unicode-3.0", "source": "registry",
     "publish": null, "targets": [{"kind": ["lib"]}]},
    {"id": "derive", "name": "serde_derive", "version": "1.0.3",
     "license": "MIT OR Apache-2.0", "source": "registry", "publish": null,
     "targets": [{"kind": ["proc-macro"]}]},
    {"id": "cc", "name": "cc", "version": "1.4.7", "license": "MIT OR Apache-2.0",
     "source": "registry", "publish": null, "targets": []},
    {"id": "shlex", "name": "shlex", "version": "2.0.1", "license": null,
     "source": "registry", "publish": null, "targets": []},
    {"id": "private", "name": "private-only", "version": "9.9.9", "license": "MIT",
     "source": "registry", "publish": null, "targets": []},
    {"id": "tester", "name": "test-only", "version": "1.0.0", "license": "MIT",
     "source": "registry", "publish": null, "targets": []}
  ],
  "workspace_members": ["dev", "root"],
  "resolve": {"nodes": [
    {"id": "root", "deps": [
      {"pkg": "json", "dep_kinds": [{"kind": null}]},
      {"pkg": "derive", "dep_kinds": [{"kind": null}]},
      {"pkg": "tester", "dep_kinds": [{"kind": "dev"}]},
      {"pkg": "cc", "dep_kinds": [{"kind": "build"}]}
    ]},
    {"id": "dev", "deps": [{"pkg": "private", "dep_kinds": [{"kind": null}]}]},
    {"id": "json", "deps": [{"pkg": "ident", "dep_kinds": [{"kind": null}]}]},
    {"id": "cc", "deps": [{"pkg": "shlex", "dep_kinds": [{"kind": null}]}]},
    {"id": "ident", "deps": []},
    {"id": "derive", "deps": []},
    {"id": "shlex", "deps": []},
    {"id": "private", "deps": []},
    {"id": "tester", "deps": []}
  ]}
}"#;

const TOOLS: &str = r#"{
  "xmllint": {"package": "libxml2", "version": "2.14.5", "licenses": ["MIT"]},
  "checkbashisms": {"package": "checkbashisms", "version": "2.25.15",
                    "licenses": ["GPL-2.0-or-later"]},
  "mystery": {"package": "mystery", "version": "1"}
}"#;

fn krate(name: &str, version: &str, license: &str, proc_macro: bool) -> Crate {
    Crate {
        name: name.to_string(),
        version: version.to_string(),
        license: license.to_string(),
        proc_macro,
    }
}

/// Runtime = normal edges from SHIPPED members; the dev member's crate, the
/// dev-dependency and our own path crates are absent; `cc` and what it pulls
/// in are build-only.
#[test]
fn the_closure_separates_runtime_from_build_and_skips_what_ships_nowhere() {
    let (runtime, build) = closure(META).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        runtime,
        vec![
            krate("serde_derive", "1.0.3", "MIT OR Apache-2.0", true),
            krate("serde_json", "1.0.1", "MIT OR Apache-2.0", false),
            krate(
                "unicode-ident",
                "1.0.2",
                "(MIT OR Apache-2.0) AND Unicode-3.0",
                false
            ),
        ]
    );
    assert_eq!(
        build,
        vec![
            krate("cc", "1.4.7", "MIT OR Apache-2.0", false),
            krate("shlex", "2.0.1", "(none declared)", false),
        ]
    );
}

#[test]
fn metadata_that_is_not_cargos_is_an_error_naming_what_is_missing() {
    let err = |m: &str| closure(m).err().unwrap_or_default();
    assert!(err("not json").contains("not JSON"));
    assert!(err("{}").contains("`packages`"));
    assert!(err(r#"{"packages": []}"#).contains("`resolve.nodes`"));
    assert!(err(r#"{"packages": [], "resolve": {"nodes": []}}"#).contains("`workspace_members`"));
}

#[test]
fn tools_are_read_sorted_by_command() {
    let t = tools(TOOLS).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        t.first(),
        Some(&Tool {
            command: "checkbashisms".to_string(),
            package: "checkbashisms".to_string(),
            version: "2.25.15".to_string(),
            licenses: vec!["GPL-2.0-or-later".to_string()],
        })
    );
    assert_eq!(t.len(), 3);
    assert!(
        tools("[]")
            .err()
            .unwrap_or_default()
            .contains("not an object")
    );
    assert!(tools("nope").err().unwrap_or_default().contains("not JSON"));
}

const UPSTREAM: &str = "\
Vendored grammar record (languages:V121).

repo:     https://example.org/tree-sitter-x
rev:      0123456789abcdef (the crate's .cargo_vcs_info.json)
          Still upstream `main` on 2026-09-27.
license:  MIT per the crate manifest; the repo's LICENSE
          file is Apache-2.0 text.
abi:      tree-sitter LANGUAGE_VERSION 15
";

#[test]
fn an_upstream_field_joins_its_indented_continuation() {
    assert_eq!(
        upstream_field(UPSTREAM, "license").as_deref(),
        Some("MIT per the crate manifest; the repo's LICENSE file is Apache-2.0 text.")
    );
    assert_eq!(
        upstream_field(UPSTREAM, "repo").as_deref(),
        Some("https://example.org/tree-sitter-x")
    );
    assert_eq!(
        upstream_field(UPSTREAM, "abi").as_deref(),
        Some("tree-sitter LANGUAGE_VERSION 15")
    );
    assert_eq!(upstream_field(UPSTREAM, "missing"), None);
}

fn vendored() -> Vec<Vendored> {
    vec![
        Vendored {
            dir: "languages/ci/x/vendor/tree-sitter-x".to_string(),
            upstream: UPSTREAM.to_string(),
            files: vec!["LICENSE.txt".to_string(), "NOTICE.txt".to_string()],
            notice: Some("Copyright 2024 Somebody\n\n".to_string()),
        },
        Vendored {
            dir: "languages/ci/y/vendor/tree-sitter-y".to_string(),
            upstream: "no fields at all\n".to_string(),
            files: vec![],
            notice: None,
        },
    ]
}

/// Every owner reaches the file: counts, the licence summary, the `AND`
/// and proc-macro notes, the build-only table, each grammar with its
/// record and NOTICE, and each tool with its licence.
#[test]
fn every_source_reaches_the_rendered_file() {
    let out = render(META, &vendored(), TOOLS).unwrap_or_else(|e| panic!("{e}"));
    for expected in [
        "# Third-party notices\n",
        "**3 third-party crates**",
        "| `MIT OR Apache-2.0` | 2 |\n| `(MIT OR Apache-2.0) AND Unicode-3.0` | 1 |",
        "whichever option is taken for the rest: `unicode-ident`.",
        "Proc-macro crates (`serde_derive`) run inside the compiler",
        "| `serde_derive` (proc-macro) | 1.0.3 | `MIT OR Apache-2.0` |",
        "## Build-time only\n\n**2 crates**",
        "| `shlex` | 2.0.1 | `(none declared)` |",
        "**2 grammars**",
        "### `tree-sitter-x`",
        "- vendored in [`languages/ci/x/vendor/tree-sitter-x`](../languages/ci/x/vendor/\
         tree-sitter-x/UPSTREAM)",
        "- upstream: <https://example.org/tree-sitter-x> at `0123456789abcdef`",
        "- licence: MIT per the crate manifest;",
        "- [`NOTICE.txt`](../languages/ci/x/vendor/tree-sitter-x/NOTICE.txt), verbatim",
        "```text\nCopyright 2024 Somebody\n```",
        "### `tree-sitter-y`",
        "| `checkbashisms` | checkbashisms | 2.25.15 | `GPL-2.0-or-later` |",
        "| `mystery` | mystery | 1 | (none declared) |",
        "## `xenolith` itself",
    ] {
        assert!(out.contains(expected), "missing {expected} in:\n{out}");
    }
    assert!(
        !out.contains("private-only"),
        "a dev member's crate ships nowhere"
    );
    assert!(!out.contains("test-only"), "a dev-dependency ships nowhere");
    assert!(!out.contains("/Users/"), "no absolute path (dev:V347)");
    assert!(out.ends_with("[`LICENSE`](../LICENSE).\n"));
}

/// Sections with nothing to say are left out rather than rendered empty.
#[test]
fn empty_sources_leave_their_sections_out() {
    let meta = r#"{"packages": [{"id": "root", "name": "xenolith", "version": "0.1.0",
        "license": "MIT", "source": null, "publish": null, "targets": []}],
        "workspace_members": ["root"], "resolve": {"nodes": [{"id": "root", "deps": []}]}}"#;
    let out = render(meta, &[], "{}").unwrap_or_else(|e| panic!("{e}"));
    assert!(out.contains("**0 third-party crates**"));
    for absent in [
        "Build-time only",
        "Vendored tree-sitter",
        "Tools the nix",
        "AND",
        "proc-macro",
    ] {
        assert!(!out.contains(absent), "{absent} in:\n{out}");
    }
    assert!(render("x", &[], "{}").is_err());
    assert!(render(meta, &[], "x").is_err());
}
