//! The xml host: the mirror of `src/host.rs` (`src:C139`).
//!
//! What is pinned: which files the host claims (`languages/data/xml:V188`);
//! that a file with a parse error yields no sites at all
//! (`languages:V78`); that the host check is `xmllint --noout`
//! (`languages/data/xml` §I, R186); and that the extract direction and
//! its inverse are REFUSED, not faked, while `languages/data/xml:T190` is
//! open (`languages/api:V37`).

use std::path::{Path, PathBuf};

use xenolith_lang_api::{
    Delim, DelimKind, Error, FileArg, Format, Host, Invoke, LangId, LoadRef, Span,
};

use super::{XmlHost, parse};

const PLIST_HEAD: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>";

const JOB: &str = "<?xml version=\"1.0\"?>\n<plist version=\"1.0\">\n<dict>\n\
                   <key>ProgramArguments</key>\n<array>\n<string>/bin/sh</string>\n\
                   <string>-c</string>\n<string>a; b</string>\n</array>\n</dict>\n</plist>\n";

fn claims(path: &str, head: &str) -> bool {
    XmlHost.claims(Path::new(path), head)
}

fn is_unsupported<T>(result: &Result<T, Error>, op: &str) -> bool {
    matches!(result, Err(Error::Unsupported { lang: LangId::Xml, operation }) if *operation == op)
}

#[test]
fn the_host_is_xml() {
    assert_eq!(XmlHost.id(), LangId::Xml);
}

#[test]
fn every_xml_file_is_claimed_whatever_its_head() {
    assert!(claims("pom.xml", PLIST_HEAD));
    assert!(claims("config/app.xml", ""));
    assert!(claims("a/b/c.xml", "<root/>"));
}

#[test]
fn a_plist_is_claimed_only_when_its_head_is_text_xml() {
    assert!(claims("com.example.job.plist", PLIST_HEAD));
    assert!(claims(
        "x.plist",
        "<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\""
    ));
    assert!(claims("x.plist", "<plist version=\"1.0\">"));
    // A byte-order mark or leading blanks do not hide the declaration.
    assert!(claims("x.plist", "\u{feff}<?xml version=\"1.0\"?>"));
    assert!(claims("x.plist", "  <plist>"));
}

#[test]
fn a_binary_plist_is_not_claimed() {
    assert!(!claims("x.plist", "bplist00\u{fffd}\u{fffd}"));
    assert!(!claims("x.plist", ""));
    assert!(!claims("x.plist", "{ Label = job; }"));
}

#[test]
fn other_xml_extensions_are_not_claimed_yet() {
    for path in [
        "logo.svg",
        "schema.xsd",
        "t.xsl",
        "page.xhtml",
        "x.xml.bak",
        "xml",
    ] {
        assert!(!claims(path, PLIST_HEAD), "{path}");
    }
}

#[test]
fn parse_reads_well_formed_xml() {
    let tree = parse(JOB).unwrap_or_else(|e| panic!("{e}"));
    assert!(!tree.root_node().has_error());
}

#[test]
fn a_launchd_job_has_its_site() {
    let found = XmlHost.sites(JOB).unwrap_or_else(|e| panic!("{e}"));
    let sinks: Vec<&str> = found.iter().map(|site| site.sink.as_str()).collect();
    assert_eq!(sinks, ["ProgramArguments"]);
}

#[test]
fn a_file_that_does_not_parse_has_no_sites_but_an_error() {
    let broken = JOB.replace("</array>", "</arr>");
    assert!(matches!(
        XmlHost.sites(&broken),
        Err(Error::Parse {
            lang: LangId::Xml,
            ..
        })
    ));
}

#[test]
fn the_host_check_is_xmllint_noout() {
    assert_eq!(
        XmlHost.checks(),
        [xenolith_lang_api::LintCmd {
            argv: vec!["xmllint".to_owned(), "--noout".to_owned()],
            file_arg: FileArg::Append,
            format: Format::Raw,
        }]
    );
    assert!(XmlHost.fixers().is_empty());
}

#[test]
fn loads_are_unsupported_until_the_load_idiom_is_decided() {
    assert!(is_unsupported(&XmlHost.loads(JOB), "loads"));
}

#[test]
fn rewrite_is_refused_until_the_load_idiom_is_decided() {
    let found = XmlHost.sites(JOB).unwrap_or_else(|e| panic!("{e}"));
    let Some(site) = found.first() else {
        panic!("the job has a site");
    };
    let invoke = Invoke {
        argv: vec!["sh".to_owned(), "job.sh".to_owned()],
    };
    let result = XmlHost.rewrite(JOB, site, &invoke, Path::new("job.sh"));
    assert!(is_unsupported(&result, "rewrite"), "{result:?}");
}

#[test]
fn inline_is_refused_with_rewrite() {
    let load = LoadRef {
        span: Span::new(0, 0),
        path: PathBuf::from("job.sh"),
        guest: LangId::Shell,
    };
    assert!(is_unsupported(
        &XmlHost.inline(JOB, &load, "a; b"),
        "inline"
    ));
}

#[test]
fn placement_is_not_offered() {
    let found = XmlHost.sites(JOB).unwrap_or_else(|e| panic!("{e}"));
    let Some(site) = found.first() else {
        panic!("the job has a site");
    };
    assert!(is_unsupported(&XmlHost.placement(site), "placement"));
}

#[test]
fn unescape_and_escape_are_the_text_module_s() {
    let delim = Delim {
        kind: DelimKind::ArgvString,
        open: Span::new(0, 0),
        body: Span::new(0, 0),
        close: Span::new(0, 0),
    };
    assert_eq!(
        XmlHost.unescape(&delim, "a &amp;&amp; b").as_deref(),
        Ok("a && b")
    );
    assert_eq!(
        XmlHost.escape(&delim, "a && b").as_deref(),
        Ok("a &amp;&amp; b")
    );
}
