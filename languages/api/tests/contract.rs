//! The contract, tested from outside the crate -- the way a language crate
//! sees it.
//!
//! These are not tests of behaviour: the traits have no behaviour, which
//! is the point of `languages/api:V36` (pure functions, `&str` in, values
//! out). What they pin is SHAPE, and shape is what breaks silently: a
//! trait that stops being object-safe, a `LangId` missing a variant for a
//! language nobody compiled in, an ordering that no longer sorts.

use std::path::Path;

use xenolith_lang_api::shebang::Shebang;
use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Guest, GuestEnv, Host, Invoke, LangId, LintCmd,
    LoadRef, Placement, Site, Span,
};

#[test]
fn lang_id_names_every_language_in_the_federation() {
    // Every language with a node under `languages/` (`languages` §F). The
    // list is written out rather than derived, so adding a crate without
    // adding its variant fails here (`languages/api:V33`).
    for (id, name) in [
        (LangId::Awk, "awk"),
        (LangId::Css, "css"),
        (LangId::Dockerfile, "dockerfile"),
        (LangId::Html, "html"),
        (LangId::Jq, "jq"),
        (LangId::Js, "js"),
        (LangId::Just, "just"),
        (LangId::Nix, "nix"),
        (LangId::Perl, "perl"),
        (LangId::Pkl, "pkl"),
        (LangId::Python, "python"),
        (LangId::Ruby, "ruby"),
        (LangId::Rust, "rust"),
        (LangId::Shell, "shell"),
        (LangId::Sql, "sql"),
        (LangId::Yaml, "yaml"),
    ] {
        assert_eq!(id.as_str(), name);
        assert_eq!(LangId::from_name(name), Some(id));
    }
}

#[test]
fn lang_id_all_is_complete_and_sorted() {
    // The registry iterates in `LangId` order (`src:V41`), so the order
    // has to be a property of this type rather than of each caller.
    let all = LangId::ALL;
    assert_eq!(all.len(), 16, "a language was added without a variant");
    let mut sorted = all.to_vec();
    sorted.sort_unstable();
    assert_eq!(sorted.as_slice(), all, "LangId::ALL must be sorted");
}

#[test]
fn an_unknown_name_is_none_rather_than_a_guess() {
    assert_eq!(LangId::from_name("cobol"), None);
    assert_eq!(LangId::from_name("Nix"), None, "matching is exact");
}

#[test]
fn spans_sort_by_start_then_end() {
    let mut spans = vec![Span::new(10, 20), Span::new(0, 5), Span::new(0, 3)];
    spans.sort_unstable();
    assert_eq!(
        spans,
        vec![Span::new(0, 3), Span::new(0, 5), Span::new(10, 20)]
    );
}

#[test]
fn a_span_knows_the_text_it_covers() {
    let src = "let x = ''echo hi'';";
    let span = Span::new(10, 17);
    assert_eq!(span.of(src), Some("echo hi"));
    assert_eq!(
        Span::new(10, 999).of(src),
        None,
        "out of bounds is None, not a panic"
    );
}

#[test]
fn traits_are_object_safe_so_the_registry_can_hold_them() {
    // `src:V41` stores `&'static [&'static dyn Host]`. A trait that stops
    // being object-safe breaks that registry and nothing else, which is
    // exactly the kind of break that is found late.
    let host: &dyn Host = &FakeHost;
    let guest: &dyn Guest = &FakeGuest;
    assert_eq!(host.id(), LangId::Nix);
    // `unescape` goes through the object too (`languages/api/src/lens:V39`).
    let site = fake_site();
    assert_eq!(host.unescape(&site.delim, "\n  x\n"), Ok("x".to_owned()));
    assert_eq!(guest.id(), LangId::Shell);
    assert_eq!(guest.extension(&GuestEnv::default()), "sh");
    assert_eq!(guest.invoke(Path::new("x.sh")).argv, vec!["bash", "x.sh"]);
}

#[test]
fn a_host_that_never_reads_a_shebang_says_so_by_default() {
    // `languages/api` §I: the one default method, answered through the
    // object like the rest (`src:V42` asks it of every host).
    let host: &dyn Host = &FakeHost;
    let site = fake_site();
    assert!(!host.guest_by_shebang("{ script = ''#!/bin/sh\n''; }", &site));
}

#[test]
fn a_site_carries_what_the_host_found() {
    let site = fake_site();
    assert_eq!(site.guest, LangId::Shell);
    assert_eq!(site.sink, "script");
    assert_eq!(site.delim.kind, DelimKind::NixIndented);
    assert_eq!(site.delim.body, Span::new(10, 17));
    assert!(site.holes.is_empty());
    assert_eq!(site.env.dialect.as_deref(), Some("bash"));
}

#[test]
fn a_load_ref_points_at_a_path_and_a_guest() {
    let load = LoadRef {
        span: Span::new(0, 12),
        path: "scripts/hk/fmt.sh".into(),
        guest: LangId::Shell,
    };
    assert_eq!(load.guest, LangId::Shell);
    assert_eq!(load.path, Path::new("scripts/hk/fmt.sh"));
}

#[test]
fn a_lint_cmd_states_how_the_file_reaches_the_tool() {
    let appended = LintCmd {
        argv: vec!["shellcheck".into(), "-s".into(), "bash".into()],
        file_arg: FileArg::Append,
        format: Format::Json("shellcheck"),
    };
    assert_eq!(appended.file_arg, FileArg::Append);

    // `[lint.<guest>] checks = ["jq -n -f {file}"]` (`src/lint` §I): the
    // path goes where the author put the token, not at the end.
    let placed = LintCmd {
        argv: vec!["jq".into(), "-n".into(), "-f".into(), "{file}".into()],
        file_arg: FileArg::Placeholder,
        format: Format::Raw,
    };
    assert_eq!(placed.file_arg, FileArg::Placeholder);
}

#[test]
fn an_error_says_which_language_and_what_went_wrong() {
    let err = Error::parse(LangId::Nix, "unterminated indented string");
    let shown = err.to_string();
    assert!(shown.contains("nix"), "got {shown:?}");
    assert!(shown.contains("unterminated"), "got {shown:?}");

    let err = Error::unsupported(LangId::Pkl, "inline");
    let shown = err.to_string();
    assert!(
        shown.contains("pkl") && shown.contains("inline"),
        "got {shown:?}"
    );
}

#[test]
fn errors_implement_the_standard_trait_so_callers_can_box_them() {
    let boxed: Box<dyn std::error::Error> = Box::new(Error::parse(LangId::Nix, "x"));
    assert!(!boxed.to_string().is_empty());
}

fn fake_site() -> Site {
    Site {
        sink: "script".into(),
        guest: LangId::Shell,
        env: GuestEnv {
            dialect: Some("bash".into()),
            options: vec!["errexit".into()],
        },
        delim: Delim {
            kind: DelimKind::NixIndented,
            open: Span::new(8, 10),
            body: Span::new(10, 17),
            close: Span::new(17, 19),
        },
        holes: Vec::new(),
    }
}

struct FakeHost;

impl Host for FakeHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }
    fn claims(&self, path: &Path, _head: &str) -> bool {
        path.extension().is_some_and(|e| e == "nix")
    }
    fn sites(&self, _src: &str) -> Result<Vec<Site>, Error> {
        Ok(vec![fake_site()])
    }
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>, Error> {
        Ok(Vec::new())
    }
    fn rewrite(
        &self,
        src: &str,
        _site: &Site,
        _invoke: &Invoke,
        _path: &Path,
    ) -> Result<String, Error> {
        Ok(src.to_owned())
    }
    fn inline(&self, src: &str, _load: &LoadRef, _body: &str) -> Result<String, Error> {
        Ok(src.to_owned())
    }
    fn unescape(&self, _delim: &Delim, raw: &str) -> Result<String, Error> {
        Ok(raw.trim().to_owned())
    }
    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

struct FakeGuest;

impl Guest for FakeGuest {
    fn prelude(&self, env: &GuestEnv) -> xenolith_lang_api::shebang::Prelude {
        xenolith_lang_api::shebang::Prelude {
            shebang: Some(Shebang::env(env.dialect.as_deref().unwrap_or("bash"))),
            strict: Some("set -euo pipefail".to_owned()),
        }
    }
    fn executable(&self) -> bool {
        true
    }
    fn id(&self) -> LangId {
        LangId::Shell
    }
    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "sh"
    }
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec!["bash".into(), path.display().to_string()],
        }
    }
    fn trivial(&self, body: &str) -> Result<bool, Error> {
        Ok(!body.contains('\n'))
    }
    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
}

#[test]
fn a_guest_without_a_construct_vocabulary_says_so_rather_than_listing_none() {
    // `languages/api:V37`: the default is a refusal naming the operation,
    // never `Ok(vec![])`, which would read as "trivial" and keep a script
    // inline.
    let guest: &dyn Guest = &FakeGuest;
    assert_eq!(
        guest.constructs("a\nb"),
        Err(Error::Unsupported {
            lang: LangId::Shell,
            operation: "constructs",
        })
    );
}

#[test]
fn a_guest_states_what_goes_above_an_extract_and_whether_it_runs() {
    // `languages/api/src/site:T48`: the extract file is
    // `shebang::wrap(body, guest.prelude(env))`, so the prelude is the
    // guest's to decide -- bash carries `set -euo pipefail`, sh cannot
    // (`pipefail` is not POSIX), and sql carries nothing because a .sql
    // file is read rather than run.
    let guest: &dyn Guest = &FakeGuest;
    let env = GuestEnv {
        dialect: Some("bash".into()),
        options: vec!["errexit".into()],
    };
    let prelude = guest.prelude(&env);
    assert_eq!(
        prelude.shebang.as_ref().map(Shebang::line).as_deref(),
        Some("#!/usr/bin/env bash")
    );
    assert_eq!(prelude.strict.as_deref(), Some("set -euo pipefail"));
    assert!(guest.executable());
}

#[test]
fn a_host_without_a_placement_says_so_rather_than_guessing_one() {
    // `languages/api/src/site:T48`: the default is a refusal naming the
    // operation (`languages/api:V37`). A made-up name would read as the
    // host's own layer-D choice (`src/extract:V45`), and `--verbose`
    // would explain a placement no host ever decided.
    let host: &dyn Host = &FakeHost;
    assert_eq!(
        host.placement(&fake_site()),
        Err(Error::Unsupported {
            lang: LangId::Nix,
            operation: "placement",
        })
    );
}

#[test]
fn a_placement_is_a_name_and_a_dir_both_templates() {
    // `languages/api/src/site` §I: a site does not carry its host's path,
    // so a dir beside the host file is a `src/extract:V46` template the
    // engine renders, and a fixed dir is a template without variables.
    let nix = Placement {
        name: "foo-script".into(),
        dir: "{host_dir}/{host_stem}".into(),
    };
    let pkl = Placement {
        name: "fmt".into(),
        dir: "scripts/hk".into(),
    };
    assert_eq!(nix.dir, "{host_dir}/{host_stem}");
    assert_eq!(pkl.name, "fmt");
    assert_ne!(nix, pkl);
}

#[test]
fn a_host_without_escape_says_so_rather_than_writing_the_body_raw() {
    // `languages/api/src/lens` §I: `inline` writes `escape(body)`, and a
    // body written back unescaped is a different program once the host
    // reads it again (`languages/api/src/lens:V39`).
    let host: &dyn Host = &FakeHost;
    assert_eq!(
        host.escape(&fake_site().delim, "echo $HOME"),
        Err(Error::Unsupported {
            lang: LangId::Nix,
            operation: "escape",
        })
    );
}

#[test]
fn a_host_without_hole_advice_says_so_rather_than_advising_nothing() {
    // `languages/api/src/holes` §I: an empty list would read as "these
    // holes have no way out" (`languages/api:V37`).
    let host: &dyn Host = &FakeHost;
    assert_eq!(
        host.hole_advice(&fake_site()),
        Err(Error::Unsupported {
            lang: LangId::Nix,
            operation: "hole_advice",
        })
    );
}
