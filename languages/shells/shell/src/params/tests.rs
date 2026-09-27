//! Unit tests for `params` (`src:C139`): the shell guest's side of holes
//! as params (`languages/api/src/holes:V40`) -- which names a body
//! already uses, where a marker may become `"$NAME"` or `${NAME}`, and
//! finding those references again for the inverse.

use xenolith_lang_api::holes::{Bound, Param, marker};
use xenolith_lang_api::{Error, LangId, Span};

use super::{param_refs, params, vars};

fn param(i: usize, name: &str) -> Param {
    Param {
        name: name.to_owned(),
        hole: format!("${{pkgs.{}}}", name.to_ascii_lowercase()),
        marker: marker(i),
    }
}

/// `body` with `@` standing for marker 0 and `%` for marker 1.
fn marked(body: &str) -> String {
    body.replace('@', &marker(0)).replace('%', &marker(1))
}

fn bound(body: &str) -> Bound {
    match params(&marked(body), &[param(0, "FOO"), param(1, "BAR")]) {
        Ok(bound) => bound,
        Err(e) => panic!("{body:?} should parse: {e}"),
    }
}

fn owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

// --- vars -----------------------------------------------------------------

#[test]
fn vars_are_every_name_read_or_assigned_sorted() {
    let body = "X=1\necho \"$Y\" ${Z:-a} $1 $@\nfor I in a; do :; done\n\
                export E\nlocal L=1\nread -r R S\nunset U\n";
    assert_eq!(
        vars(body),
        Ok(owned(&["E", "I", "L", "R", "S", "U", "X", "Y", "Z"]))
    );
}

#[test]
fn a_marker_is_not_a_var() {
    assert_eq!(vars(&marked("@ --a \"x @\"\n")), Ok(Vec::new()));
}

#[test]
fn vars_of_broken_shell_is_a_parse_error() {
    assert!(matches!(
        vars("if then fi ((\n"),
        Err(Error::Parse {
            lang: LangId::Shell,
            ..
        })
    ));
}

// --- params: where a marker becomes a reference ---------------------------

#[test]
fn a_word_gets_the_quoted_reference() {
    for (body, want) in [
        ("@ --a\n", "\"$FOO\" --a\n"),
        ("run --flag=@ %\n", "run --flag=\"$FOO\" \"$BAR\"\n"),
        ("A=@\n", "A=\"$FOO\"\n"),
        ("local a=@\n", "local a=\"$FOO\"\n"),
        ("x=$(@ --v)\n", "x=$(\"$FOO\" --v)\n"),
        ("cmd > @\n", "cmd > \"$FOO\"\n"),
        ("arr=(@ x)\n", "arr=(\"$FOO\" x)\n"),
        (
            "for f in @; do :; done\n",
            "for f in \"$FOO\"; do :; done\n",
        ),
        (
            "case @ in a) @ ;; esac\n",
            "case \"$FOO\" in a) \"$FOO\" ;; esac\n",
        ),
        ("f() { @; }\n", "f() { \"$FOO\"; }\n"),
    ] {
        assert_eq!(bound(body), Bound::Body(want.to_owned()), "{body:?}");
    }
}

#[test]
fn inside_double_quotes_the_reference_is_braced_and_bare() {
    // `"$FOO"` inside `"…"` would close and reopen the string -- the same
    // word, but not what anyone writes. `${FOO}` also stops a following
    // name character from joining the variable name.
    for (body, want) in [
        ("echo \"x @ y\"\n", "echo \"x ${FOO} y\"\n"),
        ("echo \"@\"\n", "echo \"${FOO}\"\n"),
        ("echo \"@x\"\n", "echo \"${FOO}x\"\n"),
        ("echo \"$(@ -v)\"\n", "echo \"$(\"$FOO\" -v)\"\n"),
    ] {
        assert_eq!(bound(body), Bound::Body(want.to_owned()), "{body:?}");
    }
}

#[test]
fn a_marker_the_shell_does_not_expand_is_refused() {
    // `languages/api/src/holes:V40`, conservative: anywhere a reference
    // would be read literally, or where quoting it would change what the
    // shell does, the hole stays a judgement.
    for body in [
        "echo '@'\n",
        "echo $'@'\n",
        "cat <<EOF\n@\nEOF\n",
        "cat <<'EOF'\n@\nEOF\n",
        "echo hi # @\n",
        "echo $((@ + 1))\n",
        "echo ${V:-@}\n",
        "[[ -x @ ]]\n",
        "case $x in @) : ;; esac\n",
        "@=1\n",
        "@() { :; }\n",
        "export @\n",
        "echo $\"@\"\n",
    ] {
        assert_eq!(bound(body), Bound::Unexpanded("FOO".into()), "{body:?}");
    }
}

#[test]
fn the_first_param_refused_is_named() {
    assert_eq!(bound("echo @ '%'\n"), Bound::Unexpanded("BAR".into()));
}

#[test]
fn every_occurrence_is_replaced_and_markers_never_mix() {
    // `XNL_HOLE_1_` is not a prefix of `XNL_HOLE_11_`: with twelve
    // params, the second and the twelfth stay apart.
    let many: Vec<Param> = (0..12).map(|i| param(i, &format!("P{i}"))).collect();
    let body = format!("{} {} {}\n", marker(1), marker(11), marker(1));
    assert_eq!(
        params(&body, &many),
        Ok(Bound::Body("\"$P1\" \"$P11\" \"$P1\"\n".into()))
    );
}

#[test]
fn params_of_broken_shell_is_a_parse_error() {
    assert!(matches!(
        params(&marked("if @ then\n"), &[param(0, "FOO")]),
        Err(Error::Parse { .. })
    ));
}

// --- param_refs: the inverse -----------------------------------------------

#[test]
fn param_refs_cover_what_params_wrote() {
    let body = "\"$FOO\" \"${FOO}\" $FOO \"a ${FOO} b\" \"$BAR\" ${FOO:-x} $OTHER\n";
    let refs = param_refs(body, &owned(&["FOO", "BAR"]));
    let Ok(refs) = refs else {
        panic!("should parse: {refs:?}");
    };
    let texts: Vec<(Option<&str>, &str)> = refs
        .iter()
        .map(|(span, name)| (span.of(body), name.as_str()))
        .collect();
    assert_eq!(
        texts,
        [
            (Some("\"$FOO\""), "FOO"),
            (Some("${FOO}"), "FOO"),
            (Some("$FOO"), "FOO"),
            (Some("${FOO}"), "FOO"),
            (Some("\"$BAR\""), "BAR"),
        ]
    );
}

#[test]
fn params_then_param_refs_is_the_marked_body_again() {
    // The round trip `languages/api/src/lens:V34` (e) will lean on: every
    // reference `params` wrote is found, and putting the marker back where
    // it points gives the body `params` started from.
    let ps = [param(0, "FOO"), param(1, "BAR")];
    for body in [
        "@ --a\n% @\n",
        "echo \"x @ y\" --flag=%\n",
        "echo \"@\" \"%\"\n",
        "A=@\nB=\"$(% -v)\"\n",
    ] {
        let start = marked(body);
        let Ok(Bound::Body(out)) = params(&start, &ps) else {
            panic!("{body:?} should bind");
        };
        let Ok(refs) = param_refs(&out, &owned(&["FOO", "BAR"])) else {
            panic!("{out:?} should parse");
        };
        let mut back = String::new();
        let mut at = 0;
        for (span, name) in &refs {
            back.push_str(out.get(at..span.start).unwrap_or_default());
            let i = usize::from(name == "BAR");
            back.push_str(&marker(i));
            at = span.end;
        }
        back.push_str(out.get(at..).unwrap_or_default());
        assert_eq!(back, start, "{body:?} via {out:?}");
    }
}

#[test]
fn param_refs_are_sorted_by_span() {
    let Ok(refs) = param_refs("$B $A $B\n", &owned(&["A", "B"])) else {
        panic!("should parse");
    };
    let spans: Vec<Span> = refs.iter().map(|(span, _)| *span).collect();
    let mut sorted = spans.clone();
    sorted.sort_unstable();
    assert_eq!(spans, sorted);
    assert_eq!(spans.len(), 3);
}
