//! Placement's helpers one by one (`languages/nix:T55`, `src:C139`):
//! `kebab`'s word splitting, `name`'s tail and fallback, the fixed dir
//! and the advice list.

use xenolith_lang_api::{Delim, DelimKind, GuestEnv, LangId, Site, Span};

use super::{DIR, TAIL, hole_advice, kebab, name, placement};

#[test]
fn kebab_splits_camel_case_and_punctuation() {
    assert_eq!(kebab("script"), "script");
    assert_eq!(kebab("shellHook"), "shell-hook");
    assert_eq!(kebab("ExecStartPre"), "exec-start-pre");
    assert_eq!(kebab("X11/xinit/xinitrc"), "x11-xinit-xinitrc");
    assert_eq!(kebab("x11Forwarding"), "x11-forwarding");
    assert_eq!(kebab("web-app"), "web-app");
    assert_eq!(kebab("cargo_fmt"), "cargo-fmt");
}

#[test]
fn kebab_leaves_no_empty_word_and_no_edge_dash() {
    assert_eq!(kebab("--a__b--"), "a-b");
    assert_eq!(kebab("${name}"), "name");
    assert_eq!(kebab("HTTPServer"), "httpserver");
    assert_eq!(kebab("-/-"), "");
    assert_eq!(kebab(""), "");
    // Only ASCII letters and digits are words: anything else separates.
    assert_eq!(kebab("aéb"), "a-b");
}

#[test]
fn the_name_keeps_the_last_two_attribute_segments() {
    assert_eq!(TAIL, 2);
    assert_eq!(name("systemd.services.foo.script"), "foo-script");
    assert_eq!(name("demo.preCheck"), "demo-pre-check");
    assert_eq!(name("shellHook"), "shell-hook");
    assert_eq!(
        name("systemd.services.web.serviceConfig.ExecStart"),
        "service-config-exec-start"
    );
}

#[test]
fn builder_segments_are_not_attributes() {
    // `languages/nix:V53`: the sink path names a builder the body passed
    // through, which says nothing about what is being built.
    assert_eq!(
        name("packages.deploy.writeShellApplication.text"),
        "deploy-text"
    );
    assert_eq!(name("greet.writeShellScript"), "greet");
    assert_eq!(name("docs.runCommandLocal"), "docs");
    assert_eq!(name("hook.writeScript"), "hook");
}

#[test]
fn no_attribute_left_falls_back_to_the_host_stem() {
    // `languages/api/src/site:V43`: `<host_stem>-<sink>`, the stem left as
    // a `src/extract:V46` template.
    assert_eq!(name("writeShellScript"), "{host_stem}-write-shell-script");
    // An attribute of the builder's own argument set is still one.
    assert_eq!(name("writeShellApplication.text"), "text");
    // Nothing to name it by at all: the stem alone, never a dangling `-`.
    assert_eq!(name(""), "{host_stem}");
    assert_eq!(name("${x}"), "x");
}

fn site(sink: &str) -> Site {
    Site {
        sink: sink.to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::NixIndented,
            open: Span::new(0, 2),
            body: Span::new(2, 2),
            close: Span::new(2, 4),
        },
        holes: Vec::new(),
    }
}

#[test]
fn placement_pairs_the_name_with_the_dir_beside_the_host() {
    let at = placement(&site("systemd.services.foo.script"));
    assert_eq!(at.name, "foo-script");
    assert_eq!(at.dir, DIR);
    assert_eq!(DIR, "{host_dir}/{host_stem}");
}

#[test]
fn hole_advice_is_replace_vars_argv_env_in_that_order() {
    assert_eq!(
        hole_advice(),
        [
            "replaceVars: write each `${…}` as `@var@` in the extract and load it \
             with `replaceVars ./<file> { var = …; }`",
            "argv: pass each `${…}` to the extract as an argument",
            "env: pass each `${…}` to the extract as an environment variable",
        ]
    );
}
