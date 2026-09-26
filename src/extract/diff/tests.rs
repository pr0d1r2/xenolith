use super::unified;

fn lines(n: usize) -> String {
    (1..=n)
        .flat_map(|i| ["l".to_owned(), i.to_string(), "\n".to_owned()])
        .collect()
}

#[test]
fn equal_texts_give_no_diff() {
    assert_eq!(unified("a/x", "b/x", "same\n", "same\n"), "");
}

#[test]
fn a_new_file_is_one_addition_hunk() {
    assert_eq!(
        unified("/dev/null", "b/x.sh", "", "#!/bin/sh\necho\n"),
        "--- /dev/null\n+++ b/x.sh\n@@ -0,0 +1,2 @@\n+#!/bin/sh\n+echo\n"
    );
}

#[test]
fn a_one_line_change_keeps_three_lines_of_context() {
    let old = lines(9);
    let new = old.replace("l5\n", "L5\n");
    assert_eq!(
        unified("a/f", "b/f", &old, &new),
        "--- a/f\n+++ b/f\n@@ -2,7 +2,7 @@\n l2\n l3\n l4\n-l5\n+L5\n l6\n l7\n l8\n"
    );
}

#[test]
fn a_single_line_file_leaves_out_the_count() {
    assert_eq!(
        unified("a/f", "b/f", "x\n", "y\n"),
        "--- a/f\n+++ b/f\n@@ -1 +1 @@\n-x\n+y\n"
    );
}

#[test]
fn distant_changes_are_separate_hunks_and_near_ones_merge() {
    let old = lines(20);
    let far = old.replace("l2\n", "L2\n").replace("l18\n", "L18\n");
    let diff = unified("a/f", "b/f", &old, &far);
    assert_eq!(diff.matches("@@ -").count(), 2, "{diff}");
    let near = old.replace("l2\n", "L2\n").replace("l8\n", "L8\n");
    let diff = unified("a/f", "b/f", &old, &near);
    assert_eq!(diff.matches("@@ -").count(), 1, "{diff}");
}

#[test]
fn a_missing_final_newline_is_marked() {
    assert_eq!(
        unified("a/f", "b/f", "a\nb", "a\nc"),
        "--- a/f\n+++ b/f\n@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+c\n\
         \\ No newline at end of file\n"
    );
}

#[test]
fn insertions_and_removals_in_the_middle_align_on_common_lines() {
    let old = "a\nb\nc\nd\n";
    let new = "a\nx\nc\nd\ny\n";
    assert_eq!(
        unified("a/f", "b/f", old, new),
        "--- a/f\n+++ b/f\n@@ -1,4 +1,5 @@\n a\n-b\n+x\n c\n d\n+y\n"
    );
}
