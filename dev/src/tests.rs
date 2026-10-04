//! The dispatch and the file reads: the mirror of `dev/src/lib.rs`
//! (`src:C139`).
//!
//! Every case runs against a scratch repository this file builds, never
//! this one -- a tool whose job is to rewrite a README must not be pointed
//! at the real README by a test. The notices' external owners are handed in
//! as recorded files ([`External`]), so no case runs `cargo metadata` or
//! `nix eval` against a tree that is not a crate.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use super::{
    External, USAGE, capture, notices_text, readme_blocks, repo_root, run, sources, spec_paths,
    vendored,
};

/// A scratch repository, removed on drop.
struct Repo {
    dir: PathBuf,
}

impl Drop for Repo {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.dir);
    }
}

const META: &str = r#"{"packages": [
  {"id": "root", "name": "f", "version": "0.1.0", "license": "MIT", "source": null,
   "publish": null, "targets": []},
  {"id": "tool", "name": "a-tool-crate", "version": "1.2.3", "license": "MIT",
   "source": "registry", "publish": null, "targets": []}],
 "workspace_members": ["root"],
 "resolve": {"nodes": [{"id": "root", "deps": [{"pkg": "tool", "dep_kinds": [{"kind": null}]}]},
                       {"id": "tool", "deps": []}]}}"#;

const TOOLS: &str = r#"{"shellcheck": {"package": "shellcheck", "version": "0.11.0",
  "licenses": ["GPL-3.0-or-later"]}}"#;

const README: &str = "# f\n\n<!-- BEGIN badges -->\n<!-- END badges -->\n\ntext\n\n\
                      <!-- BEGIN langs -->\n<!-- END langs -->\n\ntail\n";

impl Repo {
    fn new() -> Repo {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!(
            "xenolith-dev-unit-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&dir);
        let repo = Repo { dir };
        for (rel, body) in [
            (".git/HEAD", "ref: refs/heads/main\n"),
            ("SPEC.md", "# SPEC\n"),
            ("languages/ci/nix/SPEC.md", "# SPEC\n"),
            (
                "Cargo.toml",
                "[workspace]\nmembers = [\"dev\", \"languages/ci/nix\"]\n\n\
                 [workspace.package]\nedition = \"2024\"\nrust-version = \"1.95\"\n\
                 license = \"MIT\"\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n\n\
                 [dependencies]\nserde_json = \"1\"\n\n[features]\ndefault = [\"lang-nix\"]\n\
                 lang-nix = []\n",
            ),
            (
                "dev/Cargo.toml",
                "[package]\nname = \"d\"\npublish = false\n",
            ),
            (
                "languages/ci/nix/Cargo.toml",
                "[package]\nname = \"n\"\n\n[dependencies]\nrnix = \"0.14\"\n",
            ),
            (".coverage", "lines 97.96\n"),
            (".lint-debt", "density 0.0\n"),
            (
                "hk.pkl",
                "local fast = new Mapping<String, Step> {\n  [\"fmt\"] {\n  }\n}\n",
            ),
            (
                "flake.lock",
                "{\n  \"nodes\": {\n    \"nixpkgs\": {\n      \"locked\": {\n        \
                 \"lastModified\": 1789749394,\n        \"rev\": \"cf9d2fb3e50f\"\n      },\n      \
                 \"original\": {\n        \"ref\": \"nixos-26.05\"\n      }\n    }\n  }\n}\n",
            ),
            (".github/workflows/ci.yml", "        os: [macos-latest]\n"),
            ("README.md", README),
            (
                "languages/ci/nix/vendor/tree-sitter-n/UPSTREAM",
                "repo:     https://example.org/n\n",
            ),
            (
                "languages/ci/nix/vendor/tree-sitter-n/LICENSE",
                "MIT License\n",
            ),
            (
                "languages/ci/nix/vendor/tree-sitter-n/NOTICE.txt",
                "Copyright N\n",
            ),
            ("recorded/metadata.json", META),
            ("recorded/tools.json", TOOLS),
        ] {
            repo.write(rel, body);
        }
        repo
    }

    fn write(&self, rel: &str, body: &str) {
        let path = self.dir.join(rel);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("mkdir {}: {e}", parent.display()));
        }
        fs::write(&path, body).unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }

    fn read(&self, rel: &str) -> String {
        fs::read_to_string(self.dir.join(rel)).unwrap_or_default()
    }

    fn external(&self) -> External {
        External {
            metadata: Some(self.dir.join("recorded/metadata.json")),
            tools: Some(self.dir.join("recorded/tools.json")),
        }
    }

    /// `run` over this repository; the exit code and what it wrote.
    fn run(&self, args: &[&str]) -> (u8, String) {
        let args: Vec<String> = args.iter().map(|a| (*a).to_string()).collect();
        let mut err = Vec::new();
        let code = run(&args, Some(&self.dir), &self.external(), &mut err);
        (code, String::from_utf8_lossy(&err).into_owned())
    }
}

#[test]
fn no_verb_an_unknown_verb_or_flag_is_a_usage_error() {
    let repo = Repo::new();
    for args in [
        &[][..],
        &["nonsense"],
        &["readme", "--bogus"],
        &["--check", "--write"],
    ] {
        let (code, err) = repo.run(args);
        assert_eq!(code, 2, "{args:?}");
        assert!(err.ends_with(USAGE), "{args:?}: {err}");
    }
    let (code, err) = repo.run(&["--help"]);
    assert_eq!((code, err.as_str()), (0, USAGE));
}

#[test]
fn outside_a_repository_is_a_usage_error_naming_why() {
    let mut err = Vec::new();
    let code = run(
        &["--check".to_string()],
        None,
        &External::default(),
        &mut err,
    );
    assert_eq!(code, 2);
    assert!(String::from_utf8_lossy(&err).contains("not inside the repository"));
}

#[test]
fn the_root_is_the_nearest_ancestor_with_git_spec_and_manifest() {
    let repo = Repo::new();
    repo.write("src/deep/x.rs", "");
    assert_eq!(
        repo_root(&repo.dir.join("src/deep")),
        Some(repo.dir.clone())
    );
    assert_eq!(repo_root(Path::new("/")), None);
}

/// Every SPEC.md is a node; a dot-directory (a nested worktree) and
/// `target` are not the tree being described.
#[test]
fn spec_paths_skip_dot_directories_and_target() {
    let repo = Repo::new();
    repo.write(".claude/worktrees/x/SPEC.md", "");
    repo.write("target/debug/SPEC.md", "");
    repo.write("notes/NOTSPEC.md", "");
    assert_eq!(
        spec_paths(&repo.dir),
        vec!["SPEC.md", "languages/ci/nix/SPEC.md"]
    );
}

/// Write, check, write again: the second write changes nothing and the
/// check between is clean (`dev:V343`).
#[test]
fn readme_writes_then_checks_clean_and_is_idempotent() {
    let repo = Repo::new();
    assert_eq!(repo.run(&["readme"]).0, 0);
    let once = repo.read("README.md");
    for expected in [
        "direct_dependencies-2",
        "federated_nodes-2",
        "languages-1_built,_17_planned",
        "| nix | default build | `lang-nix` | [`languages/ci/nix`](languages/ci/nix/SPEC.md) |",
        "| awk | planned | — | — |",
    ] {
        assert!(once.contains(expected), "missing {expected} in:\n{once}");
    }
    assert!(once.ends_with("\ntail\n"));
    assert_eq!(repo.run(&["readme", "--check"]), (0, String::new()));
    assert_eq!(repo.run(&["readme"]).0, 0);
    assert_eq!(repo.read("README.md"), once);
}

/// `--check` refuses and writes nothing; a scope that touches no input is
/// clean even so, and the unscoped run still refuses (`dev:V345`).
#[test]
fn a_stale_block_is_refused_without_being_touched() {
    let repo = Repo::new();
    let stale = README.replace(
        "<!-- BEGIN badges -->\n",
        "<!-- BEGIN badges -->\nold badge\n",
    );
    repo.write("README.md", &stale);
    let (code, err) = repo.run(&["readme", "--check"]);
    assert_eq!(code, 1);
    assert!(err.contains("STALE"), "{err}");
    assert!(
        err.contains("stale: badges") && err.contains("have: old badge"),
        "{err}"
    );
    assert_eq!(repo.read("README.md"), stale);
    assert_eq!(
        repo.run(&["readme", "--check", "src/cli/mod.rs"]),
        (0, String::new())
    );
    assert_eq!(repo.run(&["readme", "--check", ".coverage"]).0, 1);
}

#[test]
fn a_readme_without_markers_is_named_and_left_alone() {
    let repo = Repo::new();
    repo.write("README.md", "# f\n\nno markers\n");
    let (code, err) = repo.run(&["readme"]);
    assert_eq!(code, 1);
    assert!(err.contains("no markers for badges, langs"), "{err}");
    assert_eq!(repo.read("README.md"), "# f\n\nno markers\n");
}

/// A missing owner is an error naming the file, never a default.
#[test]
fn a_missing_owner_is_named() {
    for rel in [
        ".lint-debt",
        "languages/ci/nix/Cargo.toml",
        "README.md",
        "hk.pkl",
    ] {
        let repo = Repo::new();
        fs::remove_file(repo.dir.join(rel)).unwrap_or_else(|e| panic!("rm {rel}: {e}"));
        let (code, err) = repo.run(&["readme"]);
        assert_eq!(code, 1, "{rel}");
        assert!(err.contains(rel), "{rel}: {err}");
    }
    let repo = Repo::new();
    repo.write(".coverage", "# no row\n");
    assert!(repo.run(&["readme"]).1.contains(".coverage"));
    assert!(sources(&repo.dir).is_ok());
    assert!(readme_blocks(&repo.dir).is_err());
}

/// The notices from recorded owners: written, then clean; a hand edit is
/// refused and left; the scope selects them only through their inputs.
#[test]
fn notices_write_check_and_refuse_a_hand_edit() {
    let repo = Repo::new();
    assert_eq!(repo.run(&["notices", "--check"]).0, 1, "absent = stale");
    assert_eq!(repo.run(&["notices"]).0, 0);
    let text = repo.read("docs/THIRD-PARTY-NOTICES.md");
    for expected in [
        "| `a-tool-crate` | 1.2.3 | `MIT` |",
        "### `tree-sitter-n`",
        "- upstream: <https://example.org/n> at `unrecorded`",
        "```text\nCopyright N\n```",
        "| `shellcheck` | shellcheck | 0.11.0 | `GPL-3.0-or-later` |",
    ] {
        assert!(text.contains(expected), "missing {expected} in:\n{text}");
    }
    assert_eq!(repo.run(&["notices", "--check"]), (0, String::new()));
    repo.write(
        "docs/THIRD-PARTY-NOTICES.md",
        &text.replace("1.2.3", "9.9.9"),
    );
    let (code, err) = repo.run(&["notices", "--check"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("STALE") && err.contains("want:") && err.contains("have:"),
        "{err}"
    );
    assert_eq!(repo.run(&["notices", "--check", "src/lib.rs"]).0, 0);
    assert_eq!(repo.run(&["notices", "--check", "Cargo.lock"]).0, 1);
}

#[test]
fn notices_name_an_owner_that_cannot_be_read() {
    let repo = Repo::new();
    let mut external = repo.external();
    external.tools = Some(repo.dir.join("recorded/absent.json"));
    let err = notices_text(&repo.dir, &external).err().unwrap_or_default();
    assert!(err.contains("absent.json"), "{err}");
    repo.write("recorded/tools.json", "not json");
    assert_eq!(repo.run(&["notices"]).0, 1);
}

#[test]
fn vendored_grammars_are_found_with_their_licence_files() {
    let repo = Repo::new();
    repo.write("languages/ci/nix/vendor/tree-sitter-n/src/parser.c", "");
    repo.write("docs/vendor/UPSTREAM.md", "");
    let v = vendored(&repo.dir).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(v.len(), 1);
    let first = v.first().unwrap_or_else(|| panic!("one grammar"));
    assert_eq!(first.dir, "languages/ci/nix/vendor/tree-sitter-n");
    assert_eq!(first.files, vec!["LICENSE", "NOTICE.txt"]);
    assert_eq!(first.notice.as_deref(), Some("Copyright N\n"));
}

/// The hooks' entry point: every job runs, each failure is named, and
/// `--fix` then leaves `--check` clean.
#[test]
fn the_top_level_check_runs_every_job_and_fix_repairs_them() {
    let repo = Repo::new();
    let (code, err) = repo.run(&["--check"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("readme failed") && err.contains("notices failed"),
        "{err}"
    );
    assert_eq!(repo.run(&["--fix"]).0, 0);
    assert_eq!(repo.run(&["--check"]), (0, String::new()));
    assert_eq!(
        repo.run(&["--check", "docs/SECURITY.md"]),
        (0, String::new())
    );
}

/// A tool that exits non-zero is an error carrying its stderr; one that
/// cannot be started is named a MISSING TOOL.
#[test]
fn capture_reports_a_failing_and_a_missing_tool() {
    let repo = Repo::new();
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let version = capture(&repo.dir, &cargo, &["--version"]).unwrap_or_else(|e| panic!("{e}"));
    assert!(version.starts_with("cargo "), "{version}");
    let failed = capture(&repo.dir, &cargo, &["no-such-subcommand-here"])
        .err()
        .unwrap_or_default();
    assert!(failed.contains("failed ("), "{failed}");
    let missing = capture(&repo.dir, "xenolith-dev-no-such-tool", &[])
        .err()
        .unwrap_or_default();
    assert!(missing.contains("MISSING TOOL"), "{missing}");
}
