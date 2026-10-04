//! The launchd sink: the mirror of `src/launchd.rs` (`src:C139`).
//!
//! What is pinned is `languages/data/xml:V189` shape by shape: a site
//! needs the top `<plist><dict>`, a `ProgramArguments` array of strings,
//! no `Program` key, a plain shell as argv[0] by basename and `-c`
//! EXACTLY as argv[1] -- the argv form of `languages/shells/shell:V139`.
//! Every other shape is no site.

use xenolith_lang_api::{DelimKind, GuestEnv, LangId, Site};

use super::sites;
use crate::host::parse;

/// A launchd plist whose top dict holds `entries` (already XML).
fn plist(entries: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\">\n<dict>\n\
         <key>Label</key>\n<string>com.example.job</string>\n{entries}</dict>\n</plist>\n"
    )
}

/// `<key>ProgramArguments</key>` and an array of `<string>` elements.
fn program_arguments(argv: &[&str]) -> String {
    let strings: String = argv
        .iter()
        .map(|arg| ["  <string>", arg, "</string>\n"].concat())
        .collect();
    format!("<key>ProgramArguments</key>\n<array>\n{strings}</array>\n")
}

fn found(src: &str) -> Vec<Site> {
    let tree = parse(src).unwrap_or_else(|e| panic!("parse failed: {e}"));
    sites(&tree, src)
}

/// The one site `src` holds.
fn one(src: &str) -> Site {
    match found(src).as_slice() {
        [site] => site.clone(),
        other => panic!("expected one site, found {other:?}"),
    }
}

fn body<'s>(src: &'s str, site: &Site) -> &'s str {
    site.delim
        .body
        .of(src)
        .unwrap_or_else(|| panic!("body span {:?} does not fit", site.delim.body))
}

fn env(dialect: &str) -> GuestEnv {
    GuestEnv {
        dialect: Some(dialect.to_owned()),
        options: Vec::new(),
    }
}

#[test]
fn sh_dash_c_in_program_arguments_is_a_site() {
    let src = plist(&program_arguments(&[
        "/bin/sh",
        "-c",
        "cd /tmp; ls | wc -l",
    ]));
    let site = &one(&src);
    assert_eq!(site.sink, "ProgramArguments");
    assert_eq!(site.guest, LangId::Shell);
    assert_eq!(site.delim.kind, DelimKind::ArgvString);
    assert!(site.holes.is_empty());
    assert_eq!(body(&src, site), "cd /tmp; ls | wc -l");
    assert_eq!(site.delim.open.of(&src), Some("<string>"));
    assert_eq!(site.delim.close.of(&src), Some("</string>"));
}

#[test]
fn the_dialect_is_argv0_by_basename_with_no_options() {
    for (argv0, dialect) in [
        ("/bin/sh", "sh"),
        ("sh", "sh"),
        ("/bin/bash", "bash"),
        ("/opt/homebrew/bin/bash", "bash"),
        ("/bin/zsh", "zsh"),
        ("/bin/dash", "dash"),
    ] {
        let src = plist(&program_arguments(&[argv0, "-c", "a; b"]));
        assert_eq!(one(&src).env, env(dialect), "{argv0}");
    }
}

#[test]
fn a_single_command_is_still_a_site_for_the_guest_to_judge() {
    // Trivial or not is the shell guest's verdict (`languages/api` §I),
    // never the host's: the host reports every sink.
    let src = plist(&program_arguments(&[
        "/bin/bash",
        "-c",
        "exec /usr/local/bin/tool",
    ]));
    assert_eq!(found(&src).len(), 1);
}

#[test]
fn trailing_positional_arguments_leave_the_site_in_argv2() {
    let src = plist(&program_arguments(&[
        "/bin/sh",
        "-c",
        "echo \"$1\"",
        "job",
        "one",
    ]));
    assert_eq!(body(&src, &one(&src)), "echo \"$1\"");
}

#[test]
fn a_program_key_means_argv0_is_not_the_executable() {
    let src = plist(&format!(
        "<key>Program</key>\n<string>/usr/bin/tool</string>\n{}",
        program_arguments(&["/bin/sh", "-c", "a; b"])
    ));
    assert!(found(&src).is_empty());
}

#[test]
fn only_c_exactly_is_the_flag() {
    for flag in ["-lc", "-ec", "-e", "--command", "-C", "c"] {
        let src = plist(&program_arguments(&["/bin/sh", flag, "a; b"]));
        assert!(found(&src).is_empty(), "{flag}");
    }
}

#[test]
fn argv0_must_be_a_plain_shell() {
    for argv0 in [
        "/usr/bin/env",
        "/usr/bin/python3",
        "/bin/ksh",
        "/bin/fish",
        "shell",
        "",
    ] {
        let src = plist(&program_arguments(&[argv0, "-c", "a; b"]));
        assert!(found(&src).is_empty(), "{argv0}");
    }
}

#[test]
fn argv_with_no_script_is_no_site() {
    assert!(found(&plist(&program_arguments(&["/bin/sh", "-c"]))).is_empty());
    assert!(found(&plist(&program_arguments(&["/bin/sh"]))).is_empty());
    assert!(found(&plist(&program_arguments(&[]))).is_empty());
}

#[test]
fn an_empty_element_script_has_no_body_to_report() {
    let src = plist(
        "<key>ProgramArguments</key>\n<array>\n<string>/bin/sh</string>\n\
         <string>-c</string>\n<string/>\n</array>\n",
    );
    assert!(found(&src).is_empty());
}

#[test]
fn an_array_holding_anything_but_strings_is_no_site() {
    let src = plist(
        "<key>ProgramArguments</key>\n<array>\n<string>/bin/sh</string>\n\
         <string>-c</string>\n<string>a; b</string>\n<integer>1</integer>\n</array>\n",
    );
    assert!(found(&src).is_empty());
}

#[test]
fn program_arguments_that_is_not_an_array_is_no_site() {
    let src = plist("<key>ProgramArguments</key>\n<string>/bin/sh -c 'a; b'</string>\n");
    assert!(found(&src).is_empty());
}

#[test]
fn a_repeated_program_arguments_key_is_no_site() {
    let twice = program_arguments(&["/bin/sh", "-c", "a; b"]).repeat(2);
    assert!(found(&plist(&twice)).is_empty());
}

#[test]
fn program_arguments_below_the_top_dict_is_no_site() {
    let nested = format!(
        "<key>KeepAlive</key>\n<dict>\n{}</dict>\n",
        program_arguments(&["/bin/sh", "-c", "a; b"])
    );
    assert!(found(&plist(&nested)).is_empty());
}

#[test]
fn the_same_argv_outside_a_plist_is_inert_data() {
    let src = format!(
        "<?xml version=\"1.0\"?>\n<config>\n<dict>\n{}</dict>\n</config>\n",
        program_arguments(&["/bin/sh", "-c", "a; b"])
    );
    assert!(found(&src).is_empty());
}

#[test]
fn a_script_in_any_other_key_is_inert_data() {
    let src = plist("<key>Comment</key>\n<string>/bin/sh -c 'a; b'</string>\n");
    assert!(found(&src).is_empty());
}

#[test]
fn a_commented_out_program_arguments_is_no_site() {
    let src = plist(&format!(
        "<!--\n{}-->\n",
        program_arguments(&["/bin/sh", "-c", "a; b"])
    ));
    assert!(found(&src).is_empty());
}

#[test]
fn comments_between_the_key_and_its_array_are_skipped() {
    let src = plist(
        "<key>ProgramArguments</key>\n<!-- the job -->\n<array>\n<string>/bin/sh</string>\n\
         <!-- flag next -->\n<string>-c</string>\n<string>a; b</string>\n</array>\n",
    );
    assert_eq!(found(&src).len(), 1);
}

#[test]
fn keys_and_argv_words_are_read_decoded() {
    let src = plist(
        "<key>Program&#65;rguments</key>\n<array>\n<string>/bin/&#x73;h</string>\n\
         <string><![CDATA[-c]]></string>\n<string>a &amp;&amp; b</string>\n</array>\n",
    );
    // The body span is the RAW text; decoding it is `unescape`'s job.
    assert_eq!(body(&src, &one(&src)), "a &amp;&amp; b");
}

#[test]
fn a_script_holding_markup_is_no_site() {
    let src = plist(&program_arguments(&["/bin/sh", "-c", "a<b/>c"]));
    assert!(found(&src).is_empty());
}

#[test]
fn a_script_with_an_unknown_entity_is_still_a_site() {
    // The shape is launchd's; the body is unreadable, which `unescape`
    // reports as such -- the engine flags it `unparseable xml string`
    // rather than letting it pass unseen.
    let src = plist(&program_arguments(&["/bin/sh", "-c", "a&nbsp;b"]));
    assert_eq!(found(&src).len(), 1);
}
