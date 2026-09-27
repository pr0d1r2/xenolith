//! Unit tests for where a recipe's extract goes (`src:C139`,
//! `languages/api/src/site:V43`).

use xenolith_lang_api::{Delim, DelimKind, GuestEnv, LangId, Site, Span};

use super::{DIR, kebab, name, placement};

fn site(sink: &str) -> Site {
    Site {
        sink: sink.to_owned(),
        guest: LangId::Shell,
        env: GuestEnv::default(),
        delim: Delim {
            kind: DelimKind::JustRecipe,
            open: Span::new(0, 0),
            body: Span::new(0, 0),
            close: Span::new(0, 0),
        },
        holes: Vec::new(),
    }
}

#[test]
fn a_recipe_goes_to_scripts_just_beside_its_justfile() {
    let at = placement(&site("build_all"));
    assert_eq!(at.name, "build-all");
    assert_eq!(at.dir, DIR);
    assert_eq!(DIR, "{host_dir}/scripts/just");
}

#[test]
fn names_are_kebab_case_and_never_empty() {
    assert_eq!(name("test"), "test");
    assert_eq!(name("fmtCheck"), "fmt-check");
    assert_eq!(name("ci-lint_2"), "ci-lint-2");
    assert_eq!(name("_"), "{host_stem}");
    assert_eq!(kebab("--a--"), "a");
    assert_eq!(kebab(""), "");
}
