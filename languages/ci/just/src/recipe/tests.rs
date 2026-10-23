//! Unit tests for `recipe.rs` (`src:C139`): the body span a recipe's
//! lines make. Reading recipes off a justfile is `parse`'s, tested there.

use xenolith_lang_api::Span;

use super::{Line, Recipe};

fn line(start: usize, end: usize) -> Line {
    Line {
        span: Span::new(start, end),
        prefix: Span::new(start, start),
        holes: Vec::new(),
    }
}

fn recipe(shebang: Option<Line>, lines: Vec<Line>) -> Recipe {
    Recipe {
        name: "a".to_owned(),
        header: Span::new(0, 2),
        shebang,
        lines,
        params: false,
        script: false,
        positional: false,
    }
}

#[test]
fn a_body_runs_from_its_first_line_s_indent_to_its_last_line_s_end() {
    let src = "a:\n    one\n\n    two\n";
    let body = recipe(None, vec![line(7, 10), line(16, 19)]).body(src);
    assert_eq!(body.and_then(|s| s.of(src)), Some("    one\n\n    two"));
}

#[test]
fn a_shebang_opens_the_body_and_may_be_all_of_it() {
    let src = "a:\n  #!/bin/sh\n  x\n";
    let shebang = line(5, 14);
    assert_eq!(
        recipe(Some(shebang.clone()), vec![line(17, 18)])
            .body(src)
            .and_then(|s| s.of(src)),
        Some("  #!/bin/sh\n  x")
    );
    assert_eq!(
        recipe(Some(shebang), Vec::new())
            .body(src)
            .and_then(|s| s.of(src)),
        Some("  #!/bin/sh")
    );
}

#[test]
fn a_recipe_without_lines_has_no_body() {
    assert_eq!(recipe(None, Vec::new()).body("a:\n"), None);
}
