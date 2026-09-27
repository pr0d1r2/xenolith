//! Shell as the GUEST of holes as params (`languages/api/src/holes:V40`),
//! through `holes::bind` the way a host's rewrite will call it: the three
//! fixtures `languages/api/src/holes:T76` names, on the real grammar.
//!
//! The host is a toy whose `unescape` is the identity, so each body below
//! is exactly what the shell guest reads.

use std::path::Path;

use xenolith_lang_api::holes::{Limits, Outcome, Param, Refusal, bind};
use xenolith_lang_api::{
    Delim, DelimKind, Error, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef, Site, Span,
};
use xenolith_lang_shell::ShellGuest;

/// `[threshold.load]` defaults (`src/config` §I).
const DEFAULT: Limits<'static> = Limits {
    max_params: 6,
    prefix: "",
};

/// `x = ''<body>'';` with every `${…}` in the body a hole.
fn bound(body: &str) -> Outcome {
    let src = format!("x = ''{body}'';");
    let start = "x = ''".len();
    let end = start + body.len();
    let holes = body
        .match_indices("${")
        .filter_map(|(open, _)| {
            let len = body.get(open..)?.find('}')? + 1;
            Some(Span::new(start + open, start + open + len))
        })
        .collect();
    let site = Site {
        sink: "x".into(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::NixIndented,
            open: Span::new(start - 2, start),
            body: Span::new(start, end),
            close: Span::new(end, end + 2),
        },
        holes,
    };
    match bind(&Identity, &ShellGuest, &src, &site, &DEFAULT) {
        Ok(outcome) => outcome,
        Err(e) => panic!("{body:?} should bind or refuse: {e}"),
    }
}

#[test]
fn the_same_hole_twice_is_one_param() {
    assert_eq!(
        bound("${pkgs.foo}/bin/foo --a\n\"${pkgs.foo}/bin/foo\" --b\n"),
        Outcome::Mechanical {
            body: "\"$FOO_BIN\" --a\n\"${FOO_BIN}\" --b\n".into(),
            params: vec![Param {
                name: "FOO_BIN".into(),
                hole: "${pkgs.foo}/bin/foo".into(),
                marker: "XNL_HOLE_0_".into(),
            }],
        }
    );
}

#[test]
fn a_hole_in_single_quotes_is_a_judgement() {
    assert_eq!(
        bound("echo '${pkgs.foo}'\n"),
        Outcome::Judgment(Refusal::Unexpanded { name: "FOO".into() })
    );
}

#[test]
fn seven_holes_is_a_judgement() {
    assert_eq!(
        bound("run ${a.p} ${a.q} ${a.r} ${a.s} ${a.t} ${a.u} ${a.v}\n"),
        Outcome::Judgment(Refusal::TooMany { count: 7, max: 6 })
    );
}

#[test]
fn a_name_the_body_already_uses_is_not_taken() {
    // `PORT` is the body's own variable; the param must not overwrite it.
    let Outcome::Mechanical { body, params } = bound("PORT=1\nserve ${cfg.port} \"$PORT\"\n")
    else {
        panic!("should bind");
    };
    assert_eq!(body, "PORT=1\nserve \"$PORT_PARAM\" \"$PORT\"\n");
    let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["PORT_PARAM"]);
}

/// `unescape` is the identity; everything else is never asked.
struct Identity;

impl Host for Identity {
    fn id(&self) -> LangId {
        LangId::Nix
    }
    fn claims(&self, _path: &Path, _head: &str) -> bool {
        false
    }
    fn sites(&self, _src: &str) -> Result<Vec<Site>, Error> {
        Err(Error::unsupported(LangId::Nix, "sites"))
    }
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>, Error> {
        Err(Error::unsupported(LangId::Nix, "loads"))
    }
    fn rewrite(&self, _: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String, Error> {
        Err(Error::unsupported(LangId::Nix, "rewrite"))
    }
    fn inline(&self, _src: &str, _load: &LoadRef, _body: &str) -> Result<String, Error> {
        Err(Error::unsupported(LangId::Nix, "inline"))
    }
    fn unescape(&self, _delim: &Delim, raw: &str) -> Result<String, Error> {
        Ok(raw.to_owned())
    }
    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}
