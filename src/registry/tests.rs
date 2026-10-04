//! The registry: the mirror of `src/registry.rs` (`src:C139`).
//!
//! What is pinned: the lists hold exactly the languages this test binary
//! was built with, sorted by `LangId` (`src/registry:V41`); a compiled-out guest
//! is named by its feature, never guessed about (`src/check:V42`); and no file
//! in `src/` but the registry reads a `lang-*` feature (`src:V30`).
//! `cargo hack --each-feature` runs these under every subset, so the
//! lang-nix-only cases below run in exactly the build they describe.

use std::fs;
use std::path::{Path, PathBuf};

use xenolith_lang_api::{Guest, Host, LangId};

use super::{
    compiled_in, existing_feature, feature, guest, guests, host, hosts, missing_shebang_guest,
    require_guest,
};

/// What the build says, stated independently of the code under test.
fn built_with(id: LangId) -> bool {
    [
        (LangId::Just, cfg!(feature = "lang-just")),
        (LangId::Nix, cfg!(feature = "lang-nix")),
        (LangId::Pkl, cfg!(feature = "lang-pkl")),
        (LangId::Shell, cfg!(feature = "lang-shell")),
        (LangId::Tcl, cfg!(feature = "lang-tcl")),
        (LangId::Xml, cfg!(feature = "lang-xml")),
    ]
    .contains(&(id, true))
}

fn host_ids() -> Vec<LangId> {
    hosts().iter().map(|h| h.id()).collect()
}

fn guest_ids() -> Vec<LangId> {
    guests().iter().map(|g| g.id()).collect()
}

// ---------------------------------------------------------------------
// the lists (`src/registry:V41`)
// ---------------------------------------------------------------------

#[test]
fn hosts_are_exactly_the_compiled_in_hosts() {
    // just, nix, pkl, shell, tcl and xml are hosts; shell and tcl are guests as well.
    let expected: Vec<LangId> = [
        LangId::Just,
        LangId::Nix,
        LangId::Pkl,
        LangId::Shell,
        LangId::Tcl,
        LangId::Xml,
    ]
    .into_iter()
    .filter(|id| built_with(*id))
    .collect();
    assert_eq!(host_ids(), expected);
}

#[test]
fn guests_are_exactly_the_compiled_in_guests() {
    let expected: Vec<LangId> = [LangId::Shell, LangId::Tcl]
        .into_iter()
        .filter(|id| built_with(*id))
        .collect();
    assert_eq!(guest_ids(), expected);
}

/// The shell HOST is the one the registry hands out for a shell file:
/// it claims `.sh` and a shell shebang, and leaves `.bats` to the bats
/// host (`languages/shells/shell:V137`), so an engine iterating [`hosts`]
/// scans this repo's scripts at all.
#[cfg(feature = "lang-shell")]
#[test]
fn the_shell_host_claims_scripts_and_not_bats_files() {
    let Some(shell) = host(LangId::Shell) else {
        panic!("lang-shell is on in this build");
    };
    let claiming = |path: &str, head: &str| {
        hosts()
            .iter()
            .filter(|h| h.claims(Path::new(path), head))
            .map(|h| h.id())
            .collect::<Vec<_>>()
    };
    assert!(shell.claims(Path::new("scripts/a.sh"), "#!/usr/bin/env bash"));
    assert_eq!(
        claiming("scripts/a.sh", "#!/usr/bin/env bash"),
        [LangId::Shell]
    );
    assert_eq!(claiming("bin/run", "#!/bin/sh"), [LangId::Shell]);
    assert!(!claiming("tests/a.bats", "#!/usr/bin/env bats").contains(&LangId::Shell));
}

/// The just HOST is the one handed out for a justfile, whatever its
/// case, and for a `.just` module (`languages/ci/just:V58`); no other
/// host claims them, so a recipe body is scanned exactly once.
#[cfg(feature = "lang-just")]
#[test]
fn the_just_host_claims_justfiles_and_nothing_else_does() {
    let claiming = |path: &str| {
        hosts()
            .iter()
            .filter(|h| h.claims(Path::new(path), ""))
            .map(|h| h.id())
            .collect::<Vec<_>>()
    };
    for path in ["justfile", "Justfile", "sub/.justfile", "ci/release.just"] {
        assert_eq!(claiming(path), [LangId::Just], "{path}");
    }
    assert!(!claiming("justfile.bak").contains(&LangId::Just));
}

/// The tcl host claims tcl and expect files and nothing the shell host
/// claims, so a `*.exp` login script is scanned by tcl alone
/// (`languages/shells/tcl:V195`).
#[cfg(feature = "lang-tcl")]
#[test]
fn the_tcl_host_claims_tcl_and_expect_files_alone() {
    let claiming = |path: &str, head: &str| {
        hosts()
            .iter()
            .filter(|h| h.claims(Path::new(path), head))
            .map(|h| h.id())
            .collect::<Vec<_>>()
    };
    assert_eq!(claiming("lib/a.tcl", ""), [LangId::Tcl]);
    assert_eq!(claiming("bin/login.exp", ""), [LangId::Tcl]);
    assert_eq!(claiming("bin/run", "#!/usr/bin/env tclsh"), [LangId::Tcl]);
    assert!(!claiming("scripts/a.sh", "#!/usr/bin/env bash").contains(&LangId::Tcl));
}

/// The xml HOST claims text plists and `.xml`, never a binary plist
/// (`languages/data/xml:V188`), and no other host claims either.
#[cfg(feature = "lang-xml")]
#[test]
fn the_xml_host_claims_text_plists_and_xml_files() {
    let claiming = |path: &str, head: &str| {
        hosts()
            .iter()
            .filter(|h| h.claims(Path::new(path), head))
            .map(|h| h.id())
            .collect::<Vec<_>>()
    };
    let text = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";
    assert_eq!(claiming("org.example.job.plist", text), [LangId::Xml]);
    assert_eq!(claiming("pom.xml", ""), [LangId::Xml]);
    assert!(claiming("org.example.job.plist", "bplist00").is_empty());
}

/// A launchd job's `sh -c` script is judged by the SHELL guest, whose
/// verdict decides what is flagged (`languages/data/xml:T192`): a
/// script is not trivial, `bash -c` with one simple command is.
#[cfg(all(feature = "lang-xml", feature = "lang-shell"))]
#[test]
fn a_launchd_script_is_judged_by_the_shell_guest() {
    let (Some(xml), Some(shell)) = (host(LangId::Xml), guest(LangId::Shell)) else {
        panic!("lang-xml and lang-shell are on in this build");
    };
    let job = |argv0: &str, script: &str| {
        format!(
            "<?xml version=\"1.0\"?>\n<plist version=\"1.0\">\n<dict>\n\
             <key>ProgramArguments</key>\n<array>\n<string>{argv0}</string>\n\
             <string>-c</string>\n<string>{script}</string>\n</array>\n</dict>\n</plist>\n"
        )
    };
    let trivial = |src: &str| -> Vec<bool> {
        let sites = xml.sites(src).unwrap_or_else(|e| panic!("{e}"));
        sites
            .iter()
            .map(|site| {
                let raw = site.delim.body.of(src).unwrap_or_default();
                let body = xml
                    .unescape(&site.delim, raw)
                    .unwrap_or_else(|e| panic!("{e}"));
                shell.trivial(&body).unwrap_or_else(|e| panic!("{e}"))
            })
            .collect()
    };
    let flagged = job("/bin/sh", "cd /tmp &amp;&amp; ls | wc -l");
    assert_eq!(trivial(&flagged), [false]);
    let single = job("/bin/bash", "exec /usr/local/bin/tool --quiet");
    assert_eq!(trivial(&single), [true]);
}

#[test]
fn both_lists_are_sorted_by_langid_without_repeats() {
    // Strictly increasing: sorted AND each language once, so an engine
    // meets hosts in one order in every subset build (`src:V11`).
    for ids in [host_ids(), guest_ids()] {
        assert!(ids.windows(2).all(|w| w.first() < w.last()), "{ids:?}");
    }
}

#[test]
fn lookup_by_id_finds_exactly_the_listed_entries() {
    for id in LangId::ALL {
        assert_eq!(
            host(*id).map(Host::id),
            host_ids().contains(id).then_some(*id)
        );
        assert_eq!(
            guest(*id).map(Guest::id),
            guest_ids().contains(id).then_some(*id)
        );
    }
}

#[test]
fn compiled_in_matches_the_features_of_this_build() {
    for id in LangId::ALL {
        assert_eq!(compiled_in(*id), built_with(*id), "{id}");
    }
}

#[test]
fn a_language_with_no_crate_yet_is_not_compiled_in() {
    // No `lang-sql` feature exists; answering "yes" would be a claim the
    // binary cannot back.
    assert!(!compiled_in(LangId::Sql));
    assert!(!compiled_in(LangId::Yaml));
}

#[test]
fn the_feature_is_lang_dash_the_id() {
    // `LangId::as_str` is the one spelling for config, JSON and features.
    assert_eq!(feature(LangId::Shell), "lang-shell");
    assert_eq!(feature(LangId::Dockerfile), "lang-dockerfile");
}

// ---------------------------------------------------------------------
// a compiled-out guest (`src/check:V42`)
// ---------------------------------------------------------------------

#[test]
fn a_compiled_in_guest_is_handed_back() {
    for id in guest_ids() {
        assert_eq!(require_guest(id).map(Guest::id).ok(), Some(id));
    }
}

#[test]
fn a_guest_no_crate_provides_is_refused_with_exit_two_naming_no_feature() {
    // Sql has no crate in any build, so this runs everywhere -- and
    // there is no `lang-sql` to rebuild with (`src/check:B8`).
    let Err(missing) = require_guest(LangId::Sql) else {
        panic!("sql is never compiled in");
    };
    assert_eq!(missing.guest, LangId::Sql);
    assert_eq!(missing.exit_code(), 2);
    let text = missing.to_string();
    assert!(!text.contains("lang-sql"), "{text}");
    assert!(text.contains("no support for sql"), "{text}");
    assert!(text.contains("src/check:V42"), "{text}");
}

#[test]
fn a_feature_exists_exactly_for_the_languages_cargo_toml_declares() {
    // The root manifest's `lang-*` features, read as text: the list the
    // messages trust must not drift from the one cargo builds.
    let manifest = fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"))
        .unwrap_or_else(|e| panic!("read Cargo.toml: {e}"));
    let declared: Vec<String> = manifest
        .lines()
        .filter_map(|line| line.split_once(" = ").map(|(key, _)| key.trim()))
        .filter(|key| key.starts_with("lang-"))
        .map(str::to_owned)
        .collect();
    let existing: Vec<String> = LangId::ALL
        .iter()
        .filter_map(|id| existing_feature(*id))
        .collect();
    assert_eq!(existing, declared);
}

#[test]
fn a_shebang_named_guest_is_a_warning_naming_the_interpreter() {
    let python = missing_shebang_guest(LangId::Python, Path::new("m.nix"), Some("python3"));
    assert_eq!(python.code, "missing-guest");
    assert_eq!(python.file.as_deref(), Some(Path::new("m.nix")));
    assert!(python.message.contains("`python3`"), "{}", python.message);
    assert!(
        python.message.contains("no support for python"),
        "{}",
        python.message
    );
    assert!(
        !python.message.contains("lang-python"),
        "{}",
        python.message
    );
    let shell = missing_shebang_guest(LangId::Shell, Path::new("m.nix"), None);
    assert!(shell.message.contains("`lang-shell`"), "{}", shell.message);
}

/// The nix-only build `src/registry:T46` names: a nix host finding shell, with no
/// shell guest to judge it.
#[cfg(all(feature = "lang-nix", not(feature = "lang-shell")))]
#[test]
fn a_nix_only_build_refuses_a_shell_site_naming_lang_shell() {
    let src = "{ systemd.services.a.script = ''\n  make && make install\n''; }\n";
    let Some(nix) = host(LangId::Nix) else {
        panic!("lang-nix is on in this build");
    };
    let sites = nix
        .sites(src)
        .unwrap_or_else(|e| panic!("fixture parses: {e}"));
    assert_eq!(sites.len(), 1, "{sites:?}");
    for site in &sites {
        assert_eq!(site.guest, LangId::Shell);
        let Err(missing) = require_guest(site.guest) else {
            panic!("shell is compiled out in this build");
        };
        assert_eq!(missing.exit_code(), 2);
        assert!(missing.to_string().contains("`lang-shell`"), "{missing}");
    }
}

// ---------------------------------------------------------------------
// no `cfg` leak (`src:V30`, `src/registry:V41`)
// ---------------------------------------------------------------------

/// Every `.rs` under `dir`, sorted.
fn rust_files(dir: &Path, into: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, into);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            into.push(path);
        }
    }
    into.sort();
}

#[test]
fn no_file_in_src_but_the_registry_reads_a_language_feature() {
    // Test mirrors are exempt: stating what the build was made with,
    // independently of the code under test, is what they are for.
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let registry = src.join("registry.rs");
    let needle = concat!("feature = \"", "lang-");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    assert!(files.contains(&registry), "{files:?}");
    let leaks: Vec<String> = files
        .iter()
        .filter(|path| **path != registry && !path.ends_with("tests.rs"))
        .filter(|path| fs::read_to_string(path).is_ok_and(|text| text.contains(needle)))
        .map(|path| path.display().to_string())
        .collect();
    assert!(
        leaks.is_empty(),
        "lang-* feature read outside the registry: {leaks:?}"
    );
}

// ---------------------------------------------------------------------
// `[langs] missing_guest` (`src/check:V42`, `src/check:T88`)
// ---------------------------------------------------------------------

mod missing_guest {
    use std::path::Path;

    use xenolith_lang_api::LangId;

    use super::super::{MISSING_GUEST, on_missing_guest};
    use crate::config::{self, Policy};

    fn policy(toml: &str) -> Policy {
        config::parse(toml)
            .unwrap_or_else(|e| panic!("fixture config parses: {e}"))
            .langs
            .missing_guest
    }

    #[test]
    fn the_default_policy_is_error() {
        assert_eq!(policy("version = 1\n"), Policy::Error);
    }

    #[test]
    fn error_refuses_with_exit_two_naming_the_feature_and_the_file() {
        let Err(missing) = on_missing_guest(Policy::Error, LangId::Shell, Path::new("db/q.nix"))
        else {
            panic!("error refuses");
        };
        assert_eq!(missing.exit_code(), 2);
        let text = missing.to_string();
        assert!(text.starts_with("db/q.nix: "), "{text}");
        assert!(text.contains("`lang-shell`"), "{text}");
    }

    #[test]
    fn warn_returns_a_missing_guest_warning_about_the_file() {
        let found = on_missing_guest(Policy::Warn, LangId::Sql, Path::new("db/q.nix"));
        let Ok(Some(warning)) = found else {
            panic!("warn warns: {found:?}");
        };
        assert_eq!(warning.code, MISSING_GUEST);
        assert_eq!(warning.code, "missing-guest");
        assert_eq!(warning.file.as_deref(), Some(Path::new("db/q.nix")));
        // Sql has no crate: the warning says so rather than name a
        // feature nobody can turn on (`src/check:B8`).
        assert!(
            warning.message.contains("no support for sql"),
            "{}",
            warning.message
        );
        assert!(!warning.message.contains("lang-sql"), "{}", warning.message);
    }

    #[test]
    fn ignore_says_nothing() {
        let found = on_missing_guest(Policy::Ignore, LangId::Sql, Path::new("db/q.nix"));
        assert_eq!(found, Ok(None));
    }

    /// The `src/check:T88` fixture: a nix host finding shell in a nix-only
    /// build, under each policy as a `xenolith.toml` states it.
    #[cfg(all(feature = "lang-nix", not(feature = "lang-shell")))]
    #[test]
    fn a_nix_only_build_applies_each_policy_to_a_shell_site() {
        let src = "{ systemd.services.a.script = ''\n  make && make install\n''; }\n";
        let file = Path::new("service.nix");
        let Some(nix) = super::host(LangId::Nix) else {
            panic!("lang-nix is on in this build");
        };
        let sites = nix
            .sites(src)
            .unwrap_or_else(|e| panic!("fixture parses: {e}"));
        let Some(site) = sites.first() else {
            panic!("the fixture holds a site");
        };
        assert!(super::guest(site.guest).is_none(), "shell is compiled out");

        let error = policy("version = 1\n");
        let Err(missing) = on_missing_guest(error, site.guest, file) else {
            panic!("the default refuses");
        };
        assert!(missing.to_string().contains("`lang-shell`"), "{missing}");

        let warn = policy("version = 1\n[langs]\nmissing_guest = \"warn\"\n");
        let Ok(Some(warning)) = on_missing_guest(warn, site.guest, file) else {
            panic!("warn warns");
        };
        assert!(
            warning.message.contains("`lang-shell`"),
            "{}",
            warning.message
        );

        let ignore = policy("version = 1\n[langs]\nmissing_guest = \"ignore\"\n");
        assert_eq!(on_missing_guest(ignore, site.guest, file), Ok(None));
    }
}
