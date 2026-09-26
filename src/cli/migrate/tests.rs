//! `xnl migrate`: the mirror of `src/cli/migrate.rs` (`src:C139`).
//!
//! What is pinned: the legacy list format as the old hooks read it (one
//! repo path per line, `#` comments, blank lines), which root files count
//! as legacy lists, the TOML the migration writes and that it parses
//! back to the same entries (`src/config:V10`), and `src/cli` §I's
//! rules for the run -- one `[[allow]]` per flagged site, never per
//! file (`src/config:V9`); a listed file that is gone or holds nothing
//! flagged is a warning, not an entry; an existing `xenolith.toml` is
//! refused, never merged into.
//!
//! The legacy fixtures are synthetic, one per format: a nix list and a
//! pkl list. The cases that need a real host and guest are gated on the
//! features they need, as `src/cli/check/tests.rs` does.

use std::fs;
use std::path::Path;

use super::{
    Entry, LEGACY_MISSING, LEGACY_NO_SITE, creation_diff, is_legacy, legacy_lists, parse_legacy,
    render, toml_string,
};
use crate::config::{self, Allow};
use crate::discover::{Sandbox, write};

/// The nix hook's list: a header paragraph, then a comment block over
/// two entries that share it, a blank line, and a bare entry.
const NIX_LIST: &str = "\
# Files grandfathered by the nix no-embedded-shell hook.

# Both wrap services whose scripts predate
# the extraction work.
services.nix
./legacy.nix

gone.nix
";

/// The pkl hook's list: one bare entry, no comments.
const PKL_LIST: &str = "hk.pkl\n";

fn entry(path: &str, line: usize, comment: Option<&str>) -> Entry {
    Entry {
        path: path.to_owned(),
        line,
        comment: comment.map(str::to_owned),
    }
}

fn allow(path: &str, sink: &str, hash: &str, reason: &str) -> Allow {
    Allow {
        path: path.to_owned(),
        sink: sink.to_owned(),
        hash: hash.to_owned(),
        reason: reason.to_owned(),
    }
}

/// Run `xnl <args>` from `root`, returning the exit code, stdout and
/// stderr.
fn xnl(root: &Path, args: &[&str]) -> (u8, String, String) {
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = crate::cli::run_in(root, args, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

fn read(root: &Path, name: &str) -> Option<String> {
    fs::read_to_string(root.join(name)).ok()
}

// ---------------------------------------------------------------------
// the legacy format
// ---------------------------------------------------------------------

#[test]
fn a_legacy_list_is_one_path_per_line_with_comments_and_blanks_skipped() {
    assert_eq!(
        parse_legacy(NIX_LIST),
        vec![
            entry(
                "services.nix",
                5,
                Some("Both wrap services whose scripts predate the extraction work.")
            ),
            entry(
                "legacy.nix",
                6,
                Some("Both wrap services whose scripts predate the extraction work.")
            ),
            entry("gone.nix", 8, None),
        ]
    );
}

#[test]
fn a_comment_separated_from_its_entry_by_a_blank_line_is_a_header() {
    // The header paragraph of NIX_LIST names no entry; only a block
    // directly above an entry is that entry's reason.
    assert_eq!(
        parse_legacy("# header\n\nsolo.nix\n"),
        vec![entry("solo.nix", 3, None)]
    );
}

#[test]
fn a_new_comment_block_between_entries_replaces_the_shared_one() {
    assert_eq!(
        parse_legacy("# first\na.nix\n# second\n#\nb.nix\n"),
        vec![
            entry("a.nix", 2, Some("first")),
            entry("b.nix", 5, Some("second")),
        ]
    );
}

#[test]
fn surrounding_whitespace_and_crlf_are_not_part_of_a_path() {
    assert_eq!(
        parse_legacy("  a.nix \r\n\r\n\t# c\r\nb.nix\r\n"),
        vec![entry("a.nix", 1, None), entry("b.nix", 4, Some("c"))]
    );
}

#[test]
fn an_empty_list_has_no_entries() {
    assert!(parse_legacy("").is_empty());
    assert!(parse_legacy("# only a comment\n\n").is_empty());
}

#[test]
fn the_legacy_names_are_dot_lang_embedded_shell_allowlist() {
    for name in [
        ".nix-embedded-shell-allowlist",
        ".pkl-embedded-shell-allowlist",
        ".justfile-embedded-shell-allowlist",
    ] {
        assert!(is_legacy(name), "{name}");
    }
    for name in [
        "nix-embedded-shell-allowlist",
        ".-embedded-shell-allowlist",
        ".nix-embedded-shell-allowlist.bak",
        ".nix-embedded-python-allowlist",
        "xenolith.toml",
    ] {
        assert!(!is_legacy(name), "{name}");
    }
}

#[test]
fn legacy_lists_are_found_at_the_root_only_and_sorted() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".pkl-embedded-shell-allowlist", PKL_LIST);
    write(&root, ".nix-embedded-shell-allowlist", NIX_LIST);
    write(&root, "sub/.nix-embedded-shell-allowlist", NIX_LIST);
    write(&root, "README.md", "# hi\n");
    assert_eq!(
        legacy_lists(&root),
        Ok(vec![
            ".nix-embedded-shell-allowlist".to_owned(),
            ".pkl-embedded-shell-allowlist".to_owned(),
        ])
    );
}

// ---------------------------------------------------------------------
// the TOML written
// ---------------------------------------------------------------------

#[test]
fn toml_strings_are_basic_strings_with_escapes() {
    assert_eq!(toml_string("a.nix"), "\"a.nix\"");
    assert_eq!(toml_string("say \"hi\""), "\"say \\\"hi\\\"\"");
    assert_eq!(toml_string("a\\b"), "\"a\\\\b\"");
    assert_eq!(toml_string("a\nb\tc"), "\"a\\nb\\tc\"");
    assert_eq!(toml_string("\u{1}\u{7f}"), "\"\\u0001\\u007F\"");
}

#[test]
fn render_writes_the_version_then_one_table_per_allow() {
    let allows = vec![
        allow("a.nix", "s.a", "0123456789abcdef", "migrated from .nix-x"),
        allow("b \"q\".nix", "s.b", "fedcba9876543210", "why\nnot"),
    ];
    let text = render(&allows);
    assert_eq!(
        text,
        "version = 1\n\
         \n\
         [[allow]]\n\
         path = \"a.nix\"\n\
         sink = \"s.a\"\n\
         hash = \"0123456789abcdef\"\n\
         reason = \"migrated from .nix-x\"\n\
         \n\
         [[allow]]\n\
         path = \"b \\\"q\\\".nix\"\n\
         sink = \"s.b\"\n\
         hash = \"fedcba9876543210\"\n\
         reason = \"why\\nnot\"\n"
    );
    let parsed = config::parse(&text).unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(parsed.allow, allows);
}

#[test]
fn render_of_no_allow_is_the_version_alone_and_parses() {
    let text = render(&[]);
    assert_eq!(text, "version = 1\n");
    assert!(config::parse(&text).is_ok());
}

#[test]
fn the_diff_creates_the_file_line_by_line() {
    assert_eq!(
        creation_diff("xenolith.toml", "version = 1\n\n[[allow]]\n"),
        "--- /dev/null\n\
         +++ b/xenolith.toml\n\
         @@ -0,0 +1,3 @@\n\
         +version = 1\n\
         +\n\
         +[[allow]]\n"
    );
}

// ---------------------------------------------------------------------
// the run, in any feature subset
// ---------------------------------------------------------------------

#[test]
fn no_legacy_list_is_nothing_to_do_and_silent() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    assert_eq!(xnl(&root, &["migrate"]), (0, String::new(), String::new()));
    let (code, out, err) = xnl(&root, &["migrate", "--verbose"]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert!(err.contains("no legacy allowlist"), "{err:?}");
    assert_eq!(read(&root, "xenolith.toml"), None);
}

#[test]
fn an_existing_config_is_refused_not_merged() {
    // `src/cli` §I: exit 2, ⊥ merge -- the file is left byte for byte.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".nix-embedded-shell-allowlist", NIX_LIST);
    write(&root, "xenolith.toml", "version = 1\n");
    for args in [&["migrate"][..], &["migrate", "--write"][..]] {
        let (code, out, err) = xnl(&root, args);
        assert_eq!(code, 2, "{args:?}: {err}");
        assert!(out.is_empty(), "{out:?}");
        assert!(err.contains("xenolith.toml"), "{err:?}");
        assert!(err.contains("merge"), "{err:?}");
    }
    assert_eq!(
        read(&root, "xenolith.toml").as_deref(),
        Some("version = 1\n")
    );
}

#[test]
fn listed_files_that_are_gone_or_outside_the_root_are_warned_about() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(
        &root,
        ".nix-embedded-shell-allowlist",
        "gone.nix\n../up.nix\n/abs.nix\n",
    );
    let (code, out, err) = xnl(&root, &["migrate"]);
    assert_eq!(code, 1, "{err}");
    assert_eq!(
        out,
        creation_diff("xenolith.toml", "version = 1\n"),
        "nothing flagged: the file holds the version alone"
    );
    for (line, path) in [(1, "gone.nix"), (2, "../up.nix"), (3, "/abs.nix")] {
        let want = format!(
            ".nix-embedded-shell-allowlist: warning: {LEGACY_MISSING}: line {line}: `{path}`"
        );
        assert!(err.contains(&want), "{want} in {err:?}");
    }
}

#[test]
fn a_listed_file_no_host_claims_holds_no_site() {
    // An unclaimed file is not scanned (`src:V13`), so it holds no site
    // xenolith flags and gets no entry.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".pkl-embedded-shell-allowlist", "notes.txt\n");
    write(&root, "notes.txt", "make && make install\n");
    let (code, _, err) = xnl(&root, &["migrate"]);
    assert_eq!(code, 1, "{err}");
    let want =
        format!(".pkl-embedded-shell-allowlist: warning: {LEGACY_NO_SITE}: line 1: `notes.txt`");
    assert!(err.contains(&want), "{want} in {err:?}");
}

#[test]
fn strict_hosts_passes_through_to_the_engine() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".pkl-embedded-shell-allowlist", "notes.txt\n");
    write(&root, "notes.txt", "hello\n");
    let (code, out, err) = xnl(&root, &["migrate", "--strict-hosts"]);
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty(), "{out:?}");
    assert!(err.contains("notes.txt: host unsupported"), "{err:?}");
}

#[test]
fn write_creates_the_file_and_prints_nothing() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".nix-embedded-shell-allowlist", "gone.nix\n");
    let (code, out, _) = xnl(&root, &["migrate", "--write"]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(
        read(&root, "xenolith.toml").as_deref(),
        Some("version = 1\n")
    );
    assert_eq!(
        read(&root, ".nix-embedded-shell-allowlist").as_deref(),
        Some("gone.nix\n"),
        "the legacy list is left for the user to delete"
    );
}

// ---------------------------------------------------------------------
// the run with real languages
// ---------------------------------------------------------------------

/// Two flagged nix sites and one single command.
#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
const SERVICES: &str = "{\n  systemd.services.a.script = ''\n    make && make install\n  '';\n  \
                        systemd.services.b.script = ''\n    curl x | sh\n  '';\n  \
                        systemd.services.c.script = \"make\";\n}\n";

#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
#[test]
fn a_nix_list_becomes_one_allow_per_flagged_site() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".nix-embedded-shell-allowlist", NIX_LIST);
    write(&root, "services.nix", SERVICES);
    write(
        &root,
        "legacy.nix",
        "{ systemd.services.a.script = \"make\"; }\n",
    );
    let reason = "migrated from .nix-embedded-shell-allowlist: Both wrap services whose \
                  scripts predate the extraction work.";
    let want = render(&[
        allow(
            "services.nix",
            "systemd.services.a.script",
            &crate::check::body_hash("\n    make && make install\n  "),
            reason,
        ),
        allow(
            "services.nix",
            "systemd.services.b.script",
            &crate::check::body_hash("\n    curl x | sh\n  "),
            reason,
        ),
    ]);
    let (code, out, err) = xnl(&root, &["migrate"]);
    assert_eq!(code, 1, "{err}");
    assert_eq!(out, creation_diff("xenolith.toml", &want));
    assert!(
        err.contains(&format!(
            ".nix-embedded-shell-allowlist: warning: {LEGACY_NO_SITE}: line 6: `legacy.nix`"
        )),
        "{err:?}"
    );
    assert!(
        err.contains(&format!(
            ".nix-embedded-shell-allowlist: warning: {LEGACY_MISSING}: line 8: `gone.nix`"
        )),
        "{err:?}"
    );
    assert_eq!(
        read(&root, "xenolith.toml"),
        None,
        "no write without --write"
    );

    let (code, out, _) = xnl(&root, &["migrate", "--write"]);
    assert_eq!((code, out.as_str()), (0, ""));
    assert_eq!(read(&root, "xenolith.toml"), Some(want));
    // The round trip: the migrated config allows exactly what the legacy
    // list grandfathered, and nothing in it is stale (`src/config:V9`).
    let (code, out, err) = xnl(&root, &["check", "services.nix", "legacy.nix"]);
    assert_eq!((code, out.as_str()), (0, ""), "{err}");
}

#[cfg(all(feature = "lang-nix", feature = "lang-shell"))]
#[test]
fn a_file_in_two_lists_gets_its_sites_once_from_the_first_list() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".nix-embedded-shell-allowlist", "services.nix\n");
    write(
        &root,
        ".pkl-embedded-shell-allowlist",
        "# dup\nservices.nix\n",
    );
    write(&root, "services.nix", SERVICES);
    let (code, _, err) = xnl(&root, &["migrate", "--write"]);
    assert_eq!(code, 0, "{err}");
    let text = read(&root, "xenolith.toml").unwrap_or_default();
    assert_eq!(text.matches("[[allow]]").count(), 2, "{text}");
    assert_eq!(
        text.matches("reason = \"migrated from .nix-embedded-shell-allowlist\"")
            .count(),
        2,
        "{text}"
    );
}

#[cfg(all(feature = "lang-pkl", feature = "lang-shell"))]
#[test]
fn a_pkl_list_becomes_an_allow_for_the_hk_step() {
    let hk = [
        "amends \"package://github.com/jdx/hk/releases/download/\
         v1.2.0/hk@1.2.0#/Config.pkl\"",
        "",
        "hooks {",
        "  [\"pre-commit\"] {",
        "    steps {",
        "      [\"lint\"] {",
        "        check = \"\"\"",
        "          cargo fmt --check && cargo clippy",
        "        \"\"\"",
        "      }",
        "    }",
        "  }",
        "}",
        "",
    ]
    .join("\n");
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    write(&root, ".pkl-embedded-shell-allowlist", PKL_LIST);
    write(&root, "hk.pkl", &hk);
    let (code, _, err) = xnl(&root, &["migrate", "--write"]);
    assert_eq!(code, 0, "{err}");
    let text = read(&root, "xenolith.toml").unwrap_or_default();
    let parsed = config::parse(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    let [only] = parsed.allow.as_slice() else {
        panic!("one allow expected: {text}");
    };
    assert_eq!(
        (only.path.as_str(), only.sink.as_str()),
        ("hk.pkl", "lint.check")
    );
    assert_eq!(only.reason, "migrated from .pkl-embedded-shell-allowlist");
    let (code, out, err) = xnl(&root, &["check", "hk.pkl"]);
    assert_eq!((code, out.as_str()), (0, ""), "{err}");
}
