//! Named blocks: the mirror of `dev/src/splice.rs` (`src:C139`).

use super::{Blocks, Outcome, apply, current, markers, splice};

fn doc(badges: &str, langs: &str) -> String {
    format!(
        "# t\n\n<!-- BEGIN badges -->\n{badges}<!-- END badges -->\n\ntext\n\n\
         <!-- BEGIN langs -->\n{langs}<!-- END langs -->\n"
    )
}

fn blocks(badges: &str, langs: &str) -> Blocks {
    vec![
        ("badges".to_string(), badges.to_string()),
        ("langs".to_string(), langs.to_string()),
    ]
}

#[test]
fn markers_are_named() {
    assert_eq!(
        markers("langs"),
        (
            "<!-- BEGIN langs -->".to_string(),
            "<!-- END langs -->".to_string()
        )
    );
}

#[test]
fn splice_replaces_only_between_one_blocks_markers() {
    let d = doc("old\n", "keep\n");
    let out = splice(&d, "badges", "new\n").unwrap_or_default();
    assert_eq!(out, doc("new\n", "keep\n"));
    assert_eq!(current(&out, "badges").as_deref(), Some("new\n"));
    assert_eq!(current(&out, "langs").as_deref(), Some("keep\n"));
}

/// Idempotent (`dev:V343`): a second splice of the same block is a no-op.
#[test]
fn splicing_twice_changes_nothing_the_second_time() {
    let once = splice(&doc("old\n", ""), "badges", "new\n").unwrap_or_default();
    assert_eq!(
        splice(&once, "badges", "new\n").as_deref(),
        Some(once.as_str())
    );
}

#[test]
fn a_block_without_both_markers_is_absent() {
    let d = "# t\n<!-- BEGIN badges -->\nx\n";
    assert_eq!(current(d, "badges"), None);
    assert_eq!(splice(d, "badges", "y\n"), None);
    assert_eq!(current("# t\n", "langs"), None);
}

#[test]
fn a_document_where_every_block_matches_is_fresh() {
    let d = doc("A\n", "B\n");
    assert_eq!(apply(&d, &blocks("A\n", "B\n"), true), Outcome::Fresh);
    assert_eq!(apply(&d, &blocks("A\n", "B\n"), false), Outcome::Fresh);
}

/// One block current, the other not: the stale one is reported, and only
/// it, with what it should say and what it says.
#[test]
fn a_stale_second_block_is_reported_even_when_the_first_is_fresh() {
    let d = doc("A\n", "OLD\n");
    let Outcome::Stale(diff) = apply(&d, &blocks("A\n", "NEW\n"), true) else {
        panic!("a stale block must be reported")
    };
    assert_eq!(diff, vec!["stale: langs", "  want: NEW", "  have: OLD"]);
}

#[test]
fn writing_replaces_every_stale_block_in_one_pass() {
    let d = doc("OLD\n", "OLD\n");
    let Outcome::Wrote(next) = apply(&d, &blocks("A\n", "B\n"), false) else {
        panic!("two stale blocks must be rewritten")
    };
    assert_eq!(next, doc("A\n", "B\n"));
    assert_eq!(apply(&next, &blocks("A\n", "B\n"), true), Outcome::Fresh);
}

/// A block the document does not carry is named, never written in.
#[test]
fn a_block_whose_markers_are_absent_is_named() {
    let d = "# t\n\n<!-- BEGIN badges -->\nA\n<!-- END badges -->\n";
    assert_eq!(
        apply(d, &blocks("A\n", "B\n"), false),
        Outcome::NoMarkers(vec!["langs".to_string()])
    );
}
