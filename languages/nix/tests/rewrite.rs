//! The write side of the nix host, shape by shape (`languages/nix:V170`,
//! `languages/nix:V53`).
//!
//! `tests/lens.rs` runs the laws over every fixture; this file pins the
//! exact text `rewrite` and `inline` write and every refusal, each with
//! the reason it is a refusal rather than a guess.

use std::path::Path;

use xenolith_lang_api::{Error, Host, Invoke, LangId, LoadRef, Site};
use xenolith_lang_nix::NixHost;

const NIX: NixHost = NixHost;

/// The one site in `src`.
fn only_site(src: &str) -> Site {
    let sites = NIX
        .sites(src)
        .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"));
    match <[Site; 1]>::try_from(sites) {
        Ok([site]) => site,
        Err(found) => panic!("{src:?}: expected one site, got {found:#?}"),
    }
}

/// The one load in `src`.
fn only_load(src: &str) -> LoadRef {
    let loads = NIX
        .loads(src)
        .unwrap_or_else(|e| panic!("{src:?} did not parse: {e}"));
    match <[LoadRef; 1]>::try_from(loads) {
        Ok([load]) => load,
        Err(found) => panic!("{src:?}: expected one load, got {found:#?}"),
    }
}

fn bash(path: &str) -> Invoke {
    Invoke {
        argv: vec!["bash".to_owned(), path.to_owned()],
    }
}

/// `rewrite` of the one site in `src` to `path`.
fn rewrite(src: &str, path: &str) -> Result<String, Error> {
    NIX.rewrite(src, &only_site(src), &bash(path), Path::new(path))
}

fn refused(operation: &'static str) -> Result<String, Error> {
    Err(Error::unsupported(LangId::Nix, operation))
}

// --- rewrite -----------------------------------------------------------

#[test]
fn without_nix_shebang_in_scope_the_load_is_read_file() {
    // `languages/nix:V53`: a name nothing binds would not evaluate, and
    // `builtins.readFile` always does.
    let src = "{ systemd.services.a.script = ''\n  echo a\n  echo b\n''; }";
    assert_eq!(
        rewrite(src, "./a/script.sh"),
        Ok("{ systemd.services.a.script = builtins.readFile ./a/script.sh; }".into())
    );
    let module = "{ pkgs, ... }: { shellHook = ''\n  a\n  b\n''; x = pkgs.a; }";
    assert_eq!(
        rewrite(module, "./x.sh"),
        Ok("{ pkgs, ... }: { shellHook = builtins.readFile ./x.sh; x = pkgs.a; }".into())
    );
}

#[test]
fn a_module_taking_nix_shebang_gets_read_without_strict() {
    // The shell guest's prelude always carries a shebang, and this is
    // the load that strips it again.
    let src = "{ nix-shebang, ... }: { script = ''\n  a\n  b\n''; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        Ok("{ nix-shebang, ... }: { script = nix-shebang.lib.readWithoutStrict ./x.sh; }".into())
    );
}

#[test]
fn a_let_bound_nix_shebang_gets_read_without_strict() {
    let src = "{ inputs }: let inherit (inputs) nix-shebang; in { script = ''\n  a\n''; }";
    let want = "{ inputs }: let inherit (inputs) nix-shebang; in \
                { script = nix-shebang.lib.readWithoutStrict ./x.sh; }";
    assert_eq!(rewrite(src, "./x.sh"), Ok(want.into()));
}

#[test]
fn a_path_the_file_already_uses_is_the_load_s_prefix() {
    let src = "{ inputs }: {\n  a = inputs.nix-shebang.lib.toShellScript;\n  \
               script = ''\n    a\n  '';\n}";
    let written = rewrite(src, "./x.sh").unwrap_or_else(|e| panic!("{e}"));
    assert!(
        written.contains("script = inputs.nix-shebang.lib.readWithoutStrict ./x.sh;"),
        "{written}"
    );
    assert_eq!(only_load(&written).path, Path::new("./x.sh"));
}

#[test]
fn a_shadowed_prefix_falls_back_to_read_file() {
    // The inner `inputs` is not the value the path was used on.
    let src = "{ inputs }: {\n  a = inputs.nix-shebang.lib.toShellScript;\n  \
               b = inputs: { script = ''\n    a\n  ''; };\n}";
    let written = rewrite(src, "./x.sh").unwrap_or_else(|e| panic!("{e}"));
    assert!(
        written.contains("script = builtins.readFile ./x.sh;"),
        "{written}"
    );
}

#[test]
fn a_bare_relative_path_gets_the_dot_slash_nix_needs() {
    // `x.sh` is no nix path at all; `./x.sh` is, relative to the host
    // file's dir (`languages/api/src/lens:V66`).
    let src = "{ shellHook = ''\n  a\n  b\n''; }";
    let written = rewrite(src, "sub/x.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(only_load(&written).path, Path::new("./sub/x.sh"));
    let parent = rewrite(src, "../x.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(only_load(&parent).path, Path::new("../x.sh"));
}

#[test]
fn an_argument_slot_gets_parentheses_and_inline_takes_them_away() {
    // Application is left-associative: `f "n" builtins.readFile ./x`
    // would hand `f` two more arguments than it had.
    let src = "{ pkgs }: pkgs.writeShellScript \"n\" ''\n  a\n  b\n''";
    let written = rewrite(src, "./x.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        written,
        "{ pkgs }: pkgs.writeShellScript \"n\" (builtins.readFile ./x.sh)"
    );
    let back = NIX.inline(&written, &only_load(&written), "a\nb\n");
    assert_eq!(
        back,
        Ok("{ pkgs }: pkgs.writeShellScript \"n\" ''\n  a\n  b\n''".into())
    );
}

#[test]
fn a_binop_operand_needs_no_parentheses() {
    let src = "{ extra }: { shellHook = ''\n  a\n  b\n'' + extra; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        Ok("{ extra }: { shellHook = builtins.readFile ./x.sh + extra; }".into())
    );
}

#[test]
fn a_site_with_holes_is_refused() {
    // `languages/nix:V54`: `replaceVars` is advice, and copying `${…}`
    // into the extract verbatim would run it as shell
    // (`languages/api/src/holes:V40`).
    let src = "{ pkgs }: { shellHook = ''\n  ${pkgs.hello}/bin/hello\n  b\n''; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        refused("rewrite of a string with holes")
    );
}

#[test]
fn a_systemd_exec_line_is_refused() {
    // `languages/nix:V69`: its load is `toShellScript`, `languages/nix:T71`.
    let src = "{ systemd.services.a.serviceConfig.ExecStart = \"a && b\"; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        refused("rewrite of a systemd exec line")
    );
}

#[test]
fn a_guest_other_than_shell_is_refused() {
    // `readWithoutStrict` and `loads` both speak shell extracts only.
    let src = "{ pkgs }: pkgs.writeText \"r.py\" ''\n  #!/usr/bin/env python3\n  print(1)\n''";
    assert_eq!(
        rewrite(src, "./r.py"),
        refused("rewrite of a guest other than shell")
    );
}

#[test]
fn a_body_led_by_the_strict_line_is_refused_under_read_without_strict() {
    // nix-shebang's `stripStrict` drops a `set -euo pipefail` right
    // under the shebang whether the prelude wrote it or the body did.
    let src = "{ nix-shebang }: { shellHook = ''\n  set -euo pipefail\n  a\n''; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        refused("rewrite of a body led by `set -euo pipefail`")
    );
}

#[test]
fn a_body_led_by_the_strict_line_is_kept_under_read_file() {
    // `builtins.readFile` strips nothing, so the body's own line stays.
    let src = "{ shellHook = ''\n  set -euo pipefail\n  a\n''; }";
    assert_eq!(
        rewrite(src, "./x.sh"),
        Ok("{ shellHook = builtins.readFile ./x.sh; }".into())
    );
}

#[test]
fn a_path_nix_cannot_load_back_is_refused() {
    let src = "{ shellHook = ''\n  a\n  b\n''; }";
    let bad = refused("rewrite to a path nix cannot load");
    for path in [
        "/abs/x.sh",
        "./a b.sh",
        "./x.py",
        "./x",
        "./a//x.sh",
        "./${x}.sh",
    ] {
        assert_eq!(rewrite(src, path), bad, "{path}");
    }
}

#[test]
fn a_string_the_author_parenthesised_is_no_site_to_rewrite() {
    // So every parenthesis `inline` takes away is one `rewrite` added,
    // and the lens stays exact.
    let src = "{ pkgs }: pkgs.writeShellScript \"n\" (''\n  a\n  b\n'')";
    assert_eq!(NIX.sites(src), Ok(Vec::new()));
}

#[test]
fn a_site_that_is_not_in_the_source_is_refused() {
    let site = only_site("{ shellHook = ''\n  a\n  b\n''; }");
    let found = NIX.rewrite("{ }", &site, &bash("./x.sh"), Path::new("./x.sh"));
    assert!(
        matches!(
            found,
            Err(Error::Parse {
                lang: LangId::Nix,
                ..
            })
        ),
        "{found:?}"
    );
}

// --- inline ------------------------------------------------------------

#[test]
fn inline_writes_a_multi_line_body_as_an_indented_string() {
    let src = "{\n  a.script = builtins.readFile ./a/x.sh;\n}";
    assert_eq!(
        NIX.inline(src, &only_load(src), "echo ${a}\necho b\n"),
        Ok("{\n  a.script = ''\n    echo ''${a}\n    echo b\n  '';\n}".into())
    );
}

#[test]
fn inline_writes_a_one_line_body_as_a_double_quoted_string() {
    let src = "{ a.script = nix-shebang.lib.readWithoutStrict ./x.sh; }";
    assert_eq!(
        NIX.inline(src, &only_load(src), "echo \"hi\""),
        Ok("{ a.script = \"echo \\\"hi\\\"\"; }".into())
    );
}

#[test]
fn inline_takes_either_load_through_any_prefix() {
    // `languages/nix:V170`: whichever load `rewrite` chose, and the
    // parentheses it added around either.
    for call in [
        "builtins.readFile ./x.sh",
        "nix-shebang.lib.readWithoutStrict ./x.sh",
        "inputs.nix-shebang.lib.readWithoutStrict ./x.sh",
    ] {
        let src = format!("{{ inputs }}: f ({call})");
        assert_eq!(
            NIX.inline(&src, &only_load(&src), "a"),
            Ok("{ inputs }: f \"a\"".into()),
            "{call}"
        );
    }
}

#[test]
fn inline_keeps_parentheses_it_did_not_need_to_add() {
    // Around an attribute value the parentheses were the author's.
    let src = "{ a.script = (builtins.readFile ./x.sh); }";
    assert_eq!(
        NIX.inline(src, &only_load(src), "a"),
        Ok("{ a.script = (\"a\"); }".into())
    );
}

#[test]
fn inline_refuses_a_load_that_is_not_in_the_source() {
    let load = only_load("{ a.script = builtins.readFile ./x.sh; }");
    let found = NIX.inline("{ }", &load, "a");
    assert!(
        matches!(
            found,
            Err(Error::Parse {
                lang: LangId::Nix,
                ..
            })
        ),
        "{found:?}"
    );
}
