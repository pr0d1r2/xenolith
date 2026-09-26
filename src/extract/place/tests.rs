use xenolith_lang_api::{LangId, Placement};

use super::{
    Ask, Field, Layer, Placed, RuleAt, Vars, disambiguate, fold, render, resolve, rules_for,
    sink_matches, suffix,
};
use crate::config::{Base, Config, ExtractRule, Layout, Tree};

// ---------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------

/// A shell site in `host_path` at `sink`, placed by a nix-like host:
/// beside the host file, in a directory named after it.
fn ask<'a>(host_path: &'a str, sink: &'a str) -> Ask<'a> {
    Ask {
        host: LangId::Nix,
        host_path,
        sink,
        guest: LangId::Shell,
        ext: "sh",
        placement: Ok(Placement {
            name: "build".to_owned(),
            dir: "{host_dir}/{host_stem}".to_owned(),
        }),
        layout: Layout::Host,
        root: "scripts",
    }
}

fn rule_at<'a>(file: &str, depth: usize, index: usize, rule: &'a ExtractRule) -> RuleAt<'a> {
    RuleAt {
        file: file.to_owned(),
        depth,
        index,
        rule,
    }
}

fn placed(ask: &Ask<'_>, rules: &[RuleAt<'_>]) -> Placed {
    resolve(ask, rules).unwrap_or_else(|e| panic!("{e}"))
}

fn refused<T: std::fmt::Debug>(got: Result<T, String>) -> String {
    match got {
        Ok(value) => panic!("expected a refusal, got {value:?}"),
        Err(e) => e,
    }
}

fn layer(placed: &Placed, field: Field) -> Layer {
    placed.why.iter().find(|(f, _)| *f == field).map_or_else(
        || panic!("no `{field}` in {:?}", placed.why),
        |(_, l)| l.clone(),
    )
}

fn vars() -> Vars {
    Vars {
        name: "build".to_owned(),
        ext: "sh".to_owned(),
        host_dir: "nixos".to_owned(),
        host_stem: "foo".to_owned(),
        sink: "a.b".to_owned(),
        guest: "shell".to_owned(),
        path: None,
        path_stem: None,
    }
}

// ---------------------------------------------------------------------
// templates (V46)
// ---------------------------------------------------------------------

#[test]
fn render_replaces_every_variable_of_the_closed_set() {
    let text = render(
        "{host_dir}/{host_stem}/{name}-{guest}.{ext}:{sink}",
        &vars(),
    );
    assert_eq!(text.as_deref(), Ok("nixos/foo/build-shell.sh:a.b"));
}

#[test]
fn render_refuses_an_unknown_variable_by_name() {
    let err = refused(render("{nme}.sh", &vars()));
    assert!(err.contains("`{nme}`"), "{err}");
    assert!(err.contains("src/extract:V46"), "{err}");
}

#[test]
fn render_refuses_an_unclosed_brace() {
    assert!(render("a/{name", &vars()).is_err());
}

#[test]
fn render_refuses_the_extract_path_before_it_exists() {
    let err = refused(render("{path_stem}.sh", &vars()));
    assert!(err.contains("cannot place it"), "{err}");
    let mut known = vars();
    known.path = Some("./foo/build.sh".to_owned());
    known.path_stem = Some("./foo/build".to_owned());
    assert_eq!(
        render("sh {path} {path_stem}", &known).as_deref(),
        Ok("sh ./foo/build.sh ./foo/build")
    );
}

#[test]
fn fold_gives_one_root_relative_spelling() {
    assert_eq!(fold("/hk/x.sh").as_deref(), Ok("hk/x.sh"));
    assert_eq!(fold("a/./b//c/../x.sh").as_deref(), Ok("a/b/x.sh"));
}

#[test]
fn fold_refuses_a_path_above_the_root_or_naming_nothing() {
    assert!(fold("a/../../x.sh").is_err());
    assert!(fold("./.").is_err());
}

// ---------------------------------------------------------------------
// matching
// ---------------------------------------------------------------------

#[test]
fn a_sink_glob_star_is_one_dotted_segment() {
    assert!(sink_matches(
        "systemd.services.*.script",
        "systemd.services.foo.script"
    ));
    assert!(!sink_matches(
        "systemd.services.*.script",
        "systemd.services.a.b.script"
    ));
    assert!(sink_matches("a.pre*", "a.preStart"));
    assert!(!sink_matches("a.pre*", "a.start"));
    assert!(sink_matches("check", "check"));
    assert!(!sink_matches("check", "fix"));
}

#[test]
fn rules_for_a_host_are_those_of_its_ancestor_files_with_their_depth() {
    let rule = ExtractRule {
        host: Some(LangId::Nix),
        ..ExtractRule::default()
    };
    let mut root = Config::default();
    root.extract.rules = vec![rule.clone()];
    let mut sub = Config::default();
    sub.extract.rules = vec![rule.clone(), rule];
    let tree = Tree::new(root)
        .with("sub", sub)
        .unwrap_or_else(|e| panic!("{e}"));
    let seen = |host: &str| -> Vec<(String, usize, usize)> {
        rules_for(&tree, host)
            .into_iter()
            .map(|at| (at.file, at.depth, at.index))
            .collect()
    };
    assert_eq!(
        seen("other/a.nix"),
        vec![("xenolith.toml".to_owned(), 0, 1)]
    );
    assert_eq!(
        seen("sub/a.nix"),
        vec![
            ("xenolith.toml".to_owned(), 0, 1),
            ("sub/xenolith.toml".to_owned(), 1, 1),
            ("sub/xenolith.toml".to_owned(), 1, 2),
        ]
    );
}

// ---------------------------------------------------------------------
// layers (V45, V48)
// ---------------------------------------------------------------------

#[test]
fn with_no_rule_the_host_places_and_the_guest_decides_the_rest() {
    let p = placed(&ask("nixos/foo.nix", "a.script"), &[]);
    assert_eq!(p.path, "nixos/foo/build.sh");
    assert_eq!(p.name, "build");
    assert_eq!(p.base, Base::Host);
    assert_eq!(
        (&p.invoke, &p.prelude, p.executable, &p.companion),
        (&None, &None, None, &None)
    );
    let fields: Vec<String> = p.why.iter().map(|(f, _)| f.to_string()).collect();
    assert_eq!(
        fields.join(" "),
        "path name base invoke prelude executable companion"
    );
    assert_eq!(layer(&p, Field::Path), Layer::Host);
    assert_eq!(layer(&p, Field::Name), Layer::Host);
    assert_eq!(layer(&p, Field::Base), Layer::Host);
    assert_eq!(layer(&p, Field::Invoke), Layer::Guest);
}

#[test]
fn a_host_at_the_root_places_without_a_leading_slash() {
    assert_eq!(placed(&ask("flake.nix", "a"), &[]).path, "flake/build.sh");
}

#[test]
fn a_placement_name_may_itself_be_a_template() {
    let mut a = ask("x/hk.pkl", "check");
    a.placement = Ok(Placement {
        name: "{host_stem}-check".to_owned(),
        dir: "scripts/hk".to_owned(),
    });
    let p = placed(&a, &[]);
    assert_eq!(p.path, "scripts/hk/hk-check.sh");
    assert_eq!(p.name, "hk-check");
}

#[test]
fn each_layout_places_under_its_own_shape() {
    let cases = [
        (Layout::Mirror, "scripts/nixos/foo/build.sh"),
        (Layout::Sibling, "nixos/foo.build.sh"),
        (Layout::Central, "scripts/shell/build.sh"),
    ];
    for (layout, want) in cases {
        let mut a = ask("nixos/foo.nix", "a");
        a.layout = layout;
        let p = placed(&a, &[]);
        assert_eq!(p.path, want, "{layout:?}");
        assert_eq!(layer(&p, Field::Path), Layer::Layout, "{layout:?}");
        assert_eq!(layer(&p, Field::Name), Layer::Host, "{layout:?}");
    }
}

#[test]
fn a_rule_path_beats_the_layout_and_names_itself() {
    let rule = ExtractRule {
        host: Some(LangId::Nix),
        path: Some("tools/{host_stem}/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let mut a = ask("nixos/foo.nix", "a");
    a.layout = Layout::Central;
    let p = placed(&a, &[rule_at("xenolith.toml", 0, 1, &rule)]);
    assert_eq!(p.path, "tools/foo/build.sh");
    assert_eq!(
        layer(&p, Field::Path),
        Layer::Rule(1, "xenolith.toml".to_owned())
    );
    assert_eq!(
        layer(&p, Field::Path).to_string(),
        "rule #1 in xenolith.toml"
    );
}

#[test]
fn fields_resolve_one_by_one_across_rules() {
    // The more specific rule sets only `invoke`; `path` still comes from
    // the less specific one, which is the only rule setting it.
    let broad = ExtractRule {
        host: Some(LangId::Nix),
        path: Some("tools/{name}.{ext}".to_owned()),
        executable: Some(false),
        ..ExtractRule::default()
    };
    let narrow = ExtractRule {
        host: Some(LangId::Nix),
        sink: Some("a.*".to_owned()),
        invoke: Some(vec!["dash".to_owned(), "{path}".to_owned()]),
        executable: Some(true),
        base: Some(Base::Root),
        ..ExtractRule::default()
    };
    let rules = [
        rule_at("xenolith.toml", 0, 1, &broad),
        rule_at("xenolith.toml", 0, 2, &narrow),
    ];
    let p = placed(&ask("nixos/foo.nix", "a.script"), &rules);
    assert_eq!(p.path, "tools/build.sh");
    assert_eq!(
        layer(&p, Field::Path),
        Layer::Rule(1, "xenolith.toml".to_owned())
    );
    assert_eq!(p.invoke, Some(vec!["dash".to_owned(), "{path}".to_owned()]));
    assert_eq!(p.executable, Some(true));
    assert_eq!(p.base, Base::Root);
    assert_eq!(
        layer(&p, Field::Executable),
        Layer::Rule(2, "xenolith.toml".to_owned())
    );
    assert_eq!(layer(&p, Field::Prelude), Layer::Guest);
}

#[test]
fn a_rule_that_does_not_match_is_not_consulted() {
    let rule = ExtractRule {
        host: Some(LangId::Pkl),
        path: Some("tools/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let p = placed(
        &ask("nixos/foo.nix", "a"),
        &[rule_at("xenolith.toml", 0, 1, &rule)],
    );
    assert_eq!(p.path, "nixos/foo/build.sh");
}

#[test]
fn on_a_specificity_tie_the_nearer_file_wins() {
    let far = ExtractRule {
        guest: Some(LangId::Shell),
        path: Some("far/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let near = ExtractRule {
        guest: Some(LangId::Shell),
        path: Some("sub/near/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let rules = [
        rule_at("sub/xenolith.toml", 1, 1, &near),
        rule_at("xenolith.toml", 0, 1, &far),
    ];
    let p = placed(&ask("sub/foo.nix", "a"), &rules);
    assert_eq!(p.path, "sub/near/build.sh");
}

#[test]
fn a_tie_inside_one_file_is_refused_naming_both_rules() {
    let one = ExtractRule {
        host: Some(LangId::Nix),
        path: Some("one/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let two = ExtractRule {
        guest: Some(LangId::Shell),
        path: Some("two/{name}.{ext}".to_owned()),
        ..ExtractRule::default()
    };
    let rules = [
        rule_at("xenolith.toml", 0, 1, &one),
        rule_at("xenolith.toml", 0, 2, &two),
    ];
    let err = refused(resolve(&ask("foo.nix", "a"), &rules));
    assert!(err.contains("rule #1 and rule #2"), "{err}");
    assert!(err.contains("`path`"), "{err}");
    assert!(err.contains("src/extract:V45"), "{err}");
}

#[test]
fn a_rule_path_that_is_absolute_climbs_out_or_names_itself_is_refused() {
    for template in ["/etc/{name}.sh", "../{name}.sh", "x/{path}"] {
        let rule = ExtractRule {
            host: Some(LangId::Nix),
            path: Some(template.to_owned()),
            ..ExtractRule::default()
        };
        let got = resolve(
            &ask("foo.nix", "a"),
            &[rule_at("xenolith.toml", 0, 1, &rule)],
        );
        assert!(got.is_err(), "{template}: {got:?}");
    }
}

#[test]
fn without_a_host_placement_only_a_rule_path_places() {
    let mut a = ask("foo.nix", "a");
    a.placement = Err("nix: does not support `placement`".to_owned());
    let err = refused(resolve(&a, &[]));
    assert!(err.contains("no place for sink `a`"), "{err}");
    let named = ExtractRule {
        host: Some(LangId::Nix),
        path: Some("x/{name}.sh".to_owned()),
        ..ExtractRule::default()
    };
    assert!(resolve(&a, &[rule_at("xenolith.toml", 0, 1, &named)]).is_err());
    let fixed = ExtractRule {
        host: Some(LangId::Nix),
        path: Some("x/{sink}.sh".to_owned()),
        ..ExtractRule::default()
    };
    let p = placed(&a, &[rule_at("xenolith.toml", 0, 1, &fixed)]);
    assert_eq!(p.path, "x/a.sh");
}

// ---------------------------------------------------------------------
// collisions (V47)
// ---------------------------------------------------------------------

#[test]
fn the_suffix_is_the_last_sink_segment_lowercased() {
    assert_eq!(suffix("systemd.services.foo.preStart"), "prestart");
    assert_eq!(suffix("script"), "script");
    assert_eq!(suffix("a.pre_start"), "pre-start");
}

#[test]
fn colliding_paths_each_take_their_sink_suffix() {
    let mut paths = vec![
        (
            "nixos/foo/foo.sh".to_owned(),
            "systemd.services.foo.script".to_owned(),
        ),
        ("nixos/foo/other.sh".to_owned(), "x".to_owned()),
        (
            "nixos/foo/foo.sh".to_owned(),
            "systemd.services.foo.preStart".to_owned(),
        ),
    ];
    assert!(disambiguate(&mut paths).is_empty());
    let got: Vec<&str> = paths.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        got,
        [
            "nixos/foo/foo-script.sh",
            "nixos/foo/other.sh",
            "nixos/foo/foo-prestart.sh"
        ]
    );
}

#[test]
fn paths_still_equal_after_the_suffix_are_returned_for_refusal() {
    let mut paths = vec![
        ("a/x".to_owned(), "one.script".to_owned()),
        ("a/x".to_owned(), "two.script".to_owned()),
        ("a/y".to_owned(), "three".to_owned()),
    ];
    assert_eq!(disambiguate(&mut paths), vec![0, 1]);
    assert_eq!(paths.first().map(|(p, _)| p.as_str()), Some("a/x-script"));
}
