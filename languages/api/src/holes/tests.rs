//! Unit tests for `holes` (`src:C139`): host interpolations become named
//! env params of the extract (`languages/api/src/holes:V40`), or the site
//! stays a judgement call.
//!
//! The host here is a toy whose `unescape` is the identity, and the guest
//! a toy that writes `$NAME` and refuses a marker inside `'…'`, so what is
//! under test is the naming, the counting and the order of the steps --
//! the real shell contexts are `languages/shells/shell`'s to pin.

use std::path::Path;

use super::{
    Bound, Hole, Limits, Outcome, Param, Refusal, base_name, bind, collect, marker, names, reserved,
};
use crate::{
    Delim, DelimKind, Error, Guest, GuestEnv, Host, Invoke, LangId, LintCmd, LoadRef, Prelude,
    Result, Site, Span,
};

/// The config default, `[threshold.load] max_params = 6` (`src/config` §I).
const DEFAULT: Limits<'static> = Limits {
    max_params: 6,
    prefix: "",
};

/// A nix-like file holding `body` in a `''…''` string, and its site, with
/// every `${…}` in the body a hole.
fn fixture(body: &str) -> (String, Site) {
    let src = format!("x = ''{body}'';");
    let start = "x = ''".len();
    let end = start + body.len();
    // Each `${` to the first `}` after it; the fixtures never nest.
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
    (src, site)
}

fn run(body: &str, limits: &Limits<'_>) -> Outcome {
    let (src, site) = fixture(body);
    match bind(&ToyHost, &ToyGuest, &src, &site, limits) {
        Ok(outcome) => outcome,
        Err(e) => panic!("the toys never fail, got {e}"),
    }
}

fn mechanical(outcome: Outcome) -> (String, Vec<Param>) {
    match outcome {
        Outcome::Mechanical { body, params } => (body, params),
        Outcome::Judgment(why) => panic!("expected mechanical, got a judgement: {why}"),
    }
}

fn judgment(outcome: Outcome) -> Refusal {
    match outcome {
        Outcome::Judgment(why) => why,
        Outcome::Mechanical { body, .. } => panic!("expected a judgement, got {body:?}"),
    }
}

fn param_names(params: &[Param]) -> Vec<&str> {
    params.iter().map(|p| p.name.as_str()).collect()
}

// --- the three fixtures `languages/api/src/holes:T76` names ---------------

#[test]
fn the_same_hole_twice_is_one_param() {
    let (body, params) = mechanical(run(
        "${pkgs.foo}/bin/foo --a\n${pkgs.foo}/bin/foo --b\n",
        &DEFAULT,
    ));
    assert_eq!(
        params,
        vec![Param {
            name: "FOO_BIN".into(),
            hole: "${pkgs.foo}/bin/foo".into(),
            marker: "XNL_HOLE_0_".into(),
        }]
    );
    assert_eq!(body, "$FOO_BIN --a\n$FOO_BIN --b\n");
}

#[test]
fn a_hole_the_guest_does_not_expand_is_a_judgement() {
    assert_eq!(
        judgment(run("echo '${pkgs.foo}'\n", &DEFAULT)),
        Refusal::Unexpanded { name: "FOO".into() }
    );
}

#[test]
fn more_holes_than_max_params_is_a_judgement() {
    let seven = "${a.p} ${a.q} ${a.r} ${a.s} ${a.t} ${a.u} ${a.v}\n";
    assert_eq!(
        judgment(run(seven, &DEFAULT)),
        Refusal::TooMany { count: 7, max: 6 }
    );
    let six = "${a.p} ${a.q} ${a.r} ${a.s} ${a.t} ${a.u}\n";
    let (_, params) = mechanical(run(six, &DEFAULT));
    assert_eq!(param_names(&params), ["P", "Q", "R", "S", "T", "U"]);
}

#[test]
fn the_threshold_counts_distinct_holes_not_occurrences() {
    let body = "${a.p} ${a.p} ${a.p} ${a.p} ${a.p} ${a.p} ${a.p}\n";
    let (_, params) = mechanical(run(
        body,
        &Limits {
            max_params: 1,
            prefix: "",
        },
    ));
    assert_eq!(param_names(&params), ["P"]);
}

// --- no holes, no guest -------------------------------------------------

#[test]
fn a_body_without_holes_is_its_unescape_and_the_guest_is_not_asked() {
    // A guest with no param support still extracts a hole-free body: the
    // capability is only needed when there is something to bind.
    let (src, site) = fixture("echo hi\n");
    assert_eq!(
        bind(&ToyHost, &Plain, &src, &site, &DEFAULT),
        Ok(Outcome::Mechanical {
            body: "echo hi\n".into(),
            params: Vec::new(),
        })
    );
}

#[test]
fn a_guest_without_params_leaves_holes_a_judgement() {
    // `languages/api:V37`: missing capability is a refusal, and V40 turns
    // it into a judgement rather than an error.
    let (src, site) = fixture("${pkgs.foo}/bin/foo\n");
    assert_eq!(
        bind(&ToyHost, &Plain, &src, &site, &DEFAULT),
        Ok(Outcome::Judgment(Refusal::Unsupported {
            operation: "vars"
        }))
    );
}

#[test]
fn a_host_that_cannot_unescape_is_an_error() {
    let (src, mut site) = fixture("${a.b}\n");
    site.delim.kind = DelimKind::ArgvString;
    assert_eq!(
        bind(&ToyHost, &ToyGuest, &src, &site, &DEFAULT),
        Err(Error::unsupported(LangId::Nix, "unescape"))
    );
}

#[test]
fn a_body_already_holding_the_marker_is_a_judgement() {
    // The markers are how the unescaped body says where each hole was; a
    // body that already spells one would bind text nobody interpolated.
    assert_eq!(
        judgment(run("echo XNL_HOLE_0_ ${a.b}\n", &DEFAULT)),
        Refusal::Marker
    );
}

// --- collect: holes, tails, duplicates ------------------------------------

#[test]
fn a_tail_is_a_slash_led_run_of_path_characters() {
    let (src, site) = fixture("${p.foo}/bin/foo-x.y+z;${p.foo}/bin ${p.foo}:${p.foo}\n");
    let holes = collect(&src, &site);
    let texts: Vec<String> = holes.iter().map(Hole::text).collect();
    // `;` and `:` end a tail: swallowing them would move shell syntax
    // into the param's value.
    assert_eq!(
        texts,
        ["${p.foo}/bin/foo-x.y+z", "${p.foo}/bin", "${p.foo}"]
    );
    assert_eq!(
        holes.get(2).map(|h| h.spans.len()),
        Some(2),
        "both bare `${{p.foo}}` are one hole"
    );
}

#[test]
fn holes_keep_first_occurrence_order_and_every_span() {
    let (src, site) = fixture("${b.y} ${a.x} ${b.y}\n");
    let holes = collect(&src, &site);
    let exprs: Vec<&str> = holes.iter().map(|h| h.expr.as_str()).collect();
    assert_eq!(exprs, ["${b.y}", "${a.x}"]);
    let Some(first) = holes.first() else {
        panic!("no holes collected");
    };
    assert_eq!(first.tail, "");
    let spans: Vec<Option<&str>> = first.spans.iter().map(|s| s.of(&src)).collect();
    assert_eq!(spans, [Some("${b.y}"), Some("${b.y}")]);
}

#[test]
fn a_hole_outside_the_body_is_not_collected() {
    let (src, mut site) = fixture("${a.b}\n");
    site.holes.push(Span::new(0, 1));
    assert_eq!(collect(&src, &site).len(), 1);
}

#[test]
fn a_marker_is_a_plain_word_with_a_terminator() {
    // `XNL_HOLE_1_` is not a prefix of `XNL_HOLE_11_`, so a text search
    // for one never lands inside the other.
    assert_eq!(marker(1), "XNL_HOLE_1_");
    assert!(!marker(11).starts_with(&marker(1)));
}

// --- base_name ------------------------------------------------------------

fn hole(expr: &str, tail: &str) -> Hole {
    Hole {
        expr: expr.into(),
        tail: tail.into(),
        spans: Vec::new(),
    }
}

#[test]
fn a_name_is_the_last_segment_plus_the_tail_dir() {
    for (expr, tail, want) in [
        ("${pkgs.foo}", "/bin/foo", "FOO_BIN"),
        ("${cfg.port}", "", "PORT"),
        ("${pkgs.foo}", "", "FOO"),
        ("${pkgs.foo}", "/share/doc/x.conf", "X_CONF_SHARE"),
        ("${pkgs.foo}", "/bin", "BIN"),
        ("${cfg.listenPort}", "", "LISTEN_PORT"),
        ("${lib.getExe pkgs.git-lfs}", "", "GIT_LFS"),
        ("\\(cfg.host)", "", "HOST"),
        ("${{ github.ref_name }}", "", "REF_NAME"),
    ] {
        assert_eq!(base_name(&hole(expr, tail)), want, "{expr}{tail}");
    }
}

#[test]
fn a_name_with_no_word_or_a_leading_digit_is_still_an_env_name() {
    assert_eq!(base_name(&hole("${ }", "")), "PARAM");
    assert_eq!(base_name(&hole("${toString 8080}", "")), "PARAM_8080");
}

// --- names: prefix, reserved, taken, collisions ---------------------------

#[test]
fn colliding_names_count_up_from_two() {
    let holes = [
        hole("${a.foo}", ""),
        hole("${b.foo}", ""),
        hole("${c.foo}", ""),
    ];
    assert_eq!(names(&holes, "", &[]), ["FOO", "FOO_2", "FOO_3"]);
}

#[test]
fn a_reserved_or_taken_name_gets_the_param_suffix() {
    let holes = [
        hole("${cfg.path}", ""),
        hole("${pkgs.bash}", "/bin/bash"),
        hole("${cfg.port}", ""),
    ];
    let taken = ["PORT".to_owned()];
    assert_eq!(
        names(&holes, "", &taken),
        ["PATH_PARAM", "BASH_BIN_PARAM", "PORT_PARAM"]
    );
}

#[test]
fn a_suffixed_name_still_counts_up_when_it_collides() {
    let holes = [hole("${a.home}", ""), hole("${b.home}", "")];
    assert_eq!(names(&holes, "", &[]), ["HOME_PARAM", "HOME_PARAM_2"]);
}

#[test]
fn a_count_that_lands_on_a_reserved_prefix_takes_the_param_suffix() {
    // `LC_2` is an `LC_*` name, and so is every count after it: counting
    // on would never end.
    let holes = [
        hole("${a.lc}", ""),
        hole("${b.lc}", ""),
        hole("${c.lc}", ""),
    ];
    assert_eq!(names(&holes, "", &[]), ["LC", "LC_PARAM", "LC_PARAM_2"]);
}

#[test]
fn the_prefix_comes_first_and_is_checked_with_the_rest() {
    let holes = [hole("${cfg.path}", ""), hole("${a.foo}", "")];
    assert_eq!(names(&holes, "XNL_", &[]), ["XNL_PATH", "XNL_FOO"]);
    assert_eq!(
        names(&holes, "XNL_", &["XNL_FOO".to_owned()]),
        ["XNL_PATH", "XNL_FOO_PARAM"]
    );
}

#[test]
fn the_reserved_set_is_the_spec_list() {
    for name in [
        "PATH",
        "HOME",
        "IFS",
        "PWD",
        "OLDPWD",
        "SHELL",
        "USER",
        "LOGNAME",
        "TERM",
        "TMPDIR",
        "LANG",
        "LC_ALL",
        "BASH",
        "BASH_ENV",
        "ZSH_NAME",
        "SHLVL",
        "PS1",
        "PS4",
        "HOSTNAME",
        "UID",
        "EUID",
        "CI",
        "GITHUB_TOKEN",
        "RUNNER_OS",
    ] {
        assert!(reserved(name), "{name} is reserved");
    }
    for name in ["FOO", "PS5", "LCD", "PATHS", "CIA", "GITHUB", "XNL_HOLE_0_"] {
        assert_eq!(reserved(name), name.starts_with("XNL_HOLE"), "{name}");
    }
}

#[test]
fn a_prefix_and_a_body_var_reach_bind() {
    let (_, params) = mechanical(run(
        "PORT=1 ${cfg.port}\n",
        &Limits {
            max_params: 6,
            prefix: "",
        },
    ));
    assert_eq!(param_names(&params), ["PORT_PARAM"]);
    let (body, params) = mechanical(run(
        "${cfg.port}\n",
        &Limits {
            max_params: 6,
            prefix: "APP_",
        },
    ));
    assert_eq!(param_names(&params), ["APP_PORT"]);
    assert_eq!(body, "$APP_PORT\n");
}

// --- the refusal is the why of a judgement ---------------------------------

#[test]
fn a_refusal_reads_as_the_why_of_a_judgement() {
    let why = Refusal::TooMany { count: 7, max: 6 }.to_string();
    assert!(
        why.contains("7 holes") && why.contains("max_params = 6"),
        "{why}"
    );
    let why = Refusal::Unexpanded { name: "FOO".into() }.to_string();
    assert!(why.contains("FOO") && why.contains("V40"), "{why}");
    let why = Refusal::Unsupported {
        operation: "params",
    }
    .to_string();
    assert!(why.contains("params"), "{why}");
    assert!(Refusal::Marker.to_string().contains("XNL_HOLE"));
}

// --- toys -----------------------------------------------------------------

/// Unescape is the identity for `''…''`, and refused for anything else.
struct ToyHost;

impl Host for ToyHost {
    fn id(&self) -> LangId {
        LangId::Nix
    }
    fn claims(&self, _path: &Path, _head: &str) -> bool {
        true
    }
    fn sites(&self, _src: &str) -> Result<Vec<Site>> {
        Ok(Vec::new())
    }
    fn loads(&self, _src: &str) -> Result<Vec<LoadRef>> {
        Ok(Vec::new())
    }
    fn rewrite(&self, src: &str, _: &Site, _: &Invoke, _: &Path) -> Result<String> {
        Ok(src.to_owned())
    }
    fn inline(&self, src: &str, _load: &LoadRef, _body: &str) -> Result<String> {
        Ok(src.to_owned())
    }
    fn unescape(&self, delim: &Delim, raw: &str) -> Result<String> {
        match delim.kind {
            DelimKind::NixIndented => Ok(raw.to_owned()),
            _ => Err(Error::unsupported(LangId::Nix, "unescape")),
        }
    }
    fn checks(&self) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self) -> Vec<LintCmd> {
        Vec::new()
    }
}

/// A guest whose vars are the `NAME=` words, whose param reference is
/// `$NAME`, and which does not expand inside `'…'`.
struct ToyGuest;

impl Guest for ToyGuest {
    fn id(&self) -> LangId {
        LangId::Shell
    }
    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "sh"
    }
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec!["sh".into(), path.display().to_string()],
        }
    }
    fn trivial(&self, _body: &str) -> Result<bool> {
        Ok(false)
    }
    fn prelude(&self, _env: &GuestEnv) -> Prelude {
        Prelude {
            shebang: None,
            strict: None,
        }
    }
    fn executable(&self) -> bool {
        true
    }
    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn vars(&self, body: &str) -> Result<Vec<String>> {
        Ok(body
            .split_whitespace()
            .filter_map(|word| word.split_once('=').map(|(name, _)| name.to_owned()))
            .collect())
    }
    fn params(&self, body: &str, params: &[Param]) -> Result<Bound> {
        let mut out = body.to_owned();
        for param in params {
            for (at, _) in body.match_indices(&param.marker) {
                let before = body.get(..at).unwrap_or_default();
                if before.matches('\'').count() % 2 == 1 {
                    return Ok(Bound::Unexpanded(param.name.clone()));
                }
            }
            out = out.replace(&param.marker, &format!("${}", param.name));
        }
        Ok(Bound::Body(out))
    }
}

/// A guest with no param support at all: every default.
struct Plain;

impl Guest for Plain {
    fn id(&self) -> LangId {
        LangId::Python
    }
    fn extension(&self, _env: &GuestEnv) -> &'static str {
        "py"
    }
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec!["python".into(), path.display().to_string()],
        }
    }
    fn trivial(&self, _body: &str) -> Result<bool> {
        Ok(false)
    }
    fn prelude(&self, _env: &GuestEnv) -> Prelude {
        Prelude {
            shebang: None,
            strict: None,
        }
    }
    fn executable(&self) -> bool {
        false
    }
    fn checks(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self, _env: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
}
