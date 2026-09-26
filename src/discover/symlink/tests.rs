//! Symlinks in discovery: the mirror of `src/discover/symlink.rs`
//! (`src:C139`), and the three fixtures `src:T127` names -- a symlinked
//! file, a symlinked directory, an explicitly named symlink.
//!
//! Every tree is built in the discovery tests' [`Sandbox`], and every git
//! call goes through it (`tests:V150`).

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};

use super::super::tests::{Sandbox, found, names, refused, write};
use super::super::{DiscoverError, SYMLINK_SKIPPED};
use super::symlinked_component;

fn link(root: &Path, target: &str, at: &str) {
    symlink(target, root.join(at)).unwrap_or_else(|e| panic!("ln -s {target} {at}: {e}"));
}

/// The (code, file) of every warning, the part a consumer matches on.
fn skipped(warnings: &[crate::model::Warning]) -> Vec<(String, String)> {
    warnings
        .iter()
        .map(|w| {
            let file = w.file.as_ref().map(|f| f.display().to_string());
            (w.code.clone(), file.unwrap_or_default())
        })
        .collect()
}

fn pair(code: &str, file: &str) -> (String, String) {
    (code.to_owned(), file.to_owned())
}

// ---------------------------------------------------------------------
// tracked symlinks (`src:V128`)
// ---------------------------------------------------------------------

#[test]
fn a_tracked_symlinked_file_is_skipped_with_a_warning() {
    // Fixture one. The target is scanned under its own name; the link
    // would be the same bytes reported twice, or bytes from outside the
    // repository reported as if they were in it.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "real.sh", "echo real\n");
    link(&root, "real.sh", "alias.sh");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    assert_eq!(names(&got.files), ["real.sh"]);
    assert_eq!(
        skipped(&got.warnings),
        [pair("symlink-skipped", "alias.sh")]
    );
    let message = got.warnings.first().map(|w| w.message.clone());
    let message = message.unwrap_or_default();
    assert!(message.contains("symlink"), "{message}");
    assert!(message.contains("src:V128"), "{message}");
}

#[test]
fn the_warning_code_is_the_stable_kebab_case_id() {
    assert_eq!(SYMLINK_SKIPPED, "symlink-skipped");
}

#[test]
fn a_tracked_symlinked_directory_is_skipped_and_not_followed() {
    // Fixture two. git records a symlinked directory as ONE link entry,
    // so what is under the target is listed once, under its real path.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "lib/a.sh", "echo a\n");
    link(&root, "lib", "vendor");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    assert_eq!(names(&got.files), ["lib/a.sh"]);
    assert_eq!(skipped(&got.warnings), [pair("symlink-skipped", "vendor")]);
}

#[test]
fn a_tracked_symlink_pointing_outside_the_repository_is_skipped() {
    // The case `src:V128` exists for: bytes the repository does not own.
    let sandbox = Sandbox::new();
    let outside = sandbox.plain("elsewhere");
    write(&outside, "secret.sh", "echo outside\n");
    let root = sandbox.repo("r");
    let target = outside.join("secret.sh");
    symlink(&target, root.join("borrowed.sh")).unwrap_or_else(|e| panic!("ln -s: {e}"));
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    assert!(got.files.is_empty(), "{:?}", got.files);
    assert_eq!(
        skipped(&got.warnings),
        [pair("symlink-skipped", "borrowed.sh")]
    );
}

#[test]
fn a_tracked_file_whose_directory_became_a_symlink_is_skipped() {
    // git tracked `d/f.sh` when `d` was a directory; the work tree now has
    // `d` as a link. Reading `d/f.sh` would read through it.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "d/f.sh", "echo f\n");
    write(&root, "real/f.sh", "echo real\n");
    sandbox.run_git(&root, &["add", "d/f.sh"]);
    fs::remove_dir_all(root.join("d")).unwrap_or_else(|e| panic!("rm d: {e}"));
    link(&root, "real", "d");
    let got = found(&sandbox, &root, &[]);
    assert!(got.files.is_empty(), "{:?}", got.files);
    assert_eq!(skipped(&got.warnings), [pair("symlink-skipped", "d/f.sh")]);
    let message = got.warnings.first().map(|w| w.message.clone());
    let message = message.unwrap_or_default();
    assert!(message.contains("`d`"), "names the link: {message}");
}

#[test]
fn a_dangling_tracked_symlink_is_skipped_not_an_error() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    link(&root, "nowhere.sh", "dangling.sh");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &[]);
    assert!(got.files.is_empty(), "{:?}", got.files);
    assert_eq!(
        skipped(&got.warnings),
        [pair("symlink-skipped", "dangling.sh")]
    );
}

// ---------------------------------------------------------------------
// walked directories (outside git)
// ---------------------------------------------------------------------

#[test]
fn a_walked_directory_skips_its_symlinks_and_does_not_enter_linked_dirs() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "d/a.sh", "echo a\n");
    write(&root, "other/b.sh", "echo b\n");
    link(&root, "a.sh", "d/alias.sh");
    link(&root, "../other", "d/linked");
    let got = found(&sandbox, &root, &["d"]);
    assert_eq!(names(&got.files), ["d/a.sh"]);
    assert_eq!(
        skipped(&got.warnings),
        [
            pair("symlink-skipped", "d/alias.sh"),
            pair("symlink-skipped", "d/linked"),
        ]
    );
}

#[test]
fn an_explicit_directory_in_a_repository_skips_its_tracked_symlinks() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "d/a.sh", "echo a\n");
    link(&root, "a.sh", "d/alias.sh");
    sandbox.run_git(&root, &["add", "."]);
    let got = found(&sandbox, &root, &["d"]);
    assert_eq!(names(&got.files), ["d/a.sh"]);
    assert_eq!(
        skipped(&got.warnings),
        [pair("symlink-skipped", "d/alias.sh")]
    );
}

// ---------------------------------------------------------------------
// named explicitly: exit 2 (`src:V128`)
// ---------------------------------------------------------------------

#[test]
fn an_explicitly_named_symlink_is_refused_with_exit_2() {
    // Fixture three. Skipping with a warning would scan nothing and exit
    // 0 on the one file the caller asked about.
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "real.sh", "echo real\n");
    link(&root, "real.sh", "alias.sh");
    let e = refused(&sandbox, &root, &["alias.sh"]);
    assert_eq!(
        e,
        DiscoverError::Symlink {
            path: PathBuf::from("alias.sh"),
            link: PathBuf::from("alias.sh"),
        }
    );
    assert_eq!(e.exit_code(), 2);
    let text = e.to_string();
    assert!(text.contains("alias.sh"), "{text}");
    assert!(text.contains("symlink"), "{text}");
    assert!(text.contains("src:V128"), "{text}");
}

#[test]
fn a_symlink_among_other_named_files_still_refuses_the_run() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "a.sh", "echo a\n");
    link(&root, "a.sh", "b.sh");
    let e = refused(&sandbox, &root, &["a.sh", "b.sh"]);
    assert!(matches!(e, DiscoverError::Symlink { .. }), "{e:?}");
}

#[test]
fn an_explicitly_named_symlinked_directory_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "lib/a.sh", "echo a\n");
    link(&root, "lib", "vendor");
    let e = refused(&sandbox, &root, &["vendor"]);
    assert_eq!(
        e,
        DiscoverError::Symlink {
            path: PathBuf::from("vendor"),
            link: PathBuf::from("vendor"),
        }
    );
}

#[test]
fn an_explicit_path_through_a_symlinked_directory_is_refused_naming_the_link() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    write(&root, "lib/a.sh", "echo a\n");
    link(&root, "lib", "vendor");
    let e = refused(&sandbox, &root, &["vendor/a.sh"]);
    assert_eq!(
        e,
        DiscoverError::Symlink {
            path: PathBuf::from("vendor/a.sh"),
            link: PathBuf::from("vendor"),
        }
    );
    let text = e.to_string();
    assert!(text.contains("vendor/a.sh"), "{text}");
    assert!(text.contains("`vendor`"), "{text}");
}

#[test]
fn an_explicitly_named_dangling_symlink_is_a_symlink_refusal_not_missing() {
    // The link exists; what it points at does not. Saying "no such file"
    // would send the user looking for a file that is plainly there.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    link(&root, "nowhere.sh", "dangling.sh");
    let e = refused(&sandbox, &root, &["dangling.sh"]);
    assert!(matches!(e, DiscoverError::Symlink { .. }), "{e:?}");
}

// ---------------------------------------------------------------------
// `symlinked_component`
// ---------------------------------------------------------------------

#[test]
fn a_plain_path_has_no_symlinked_component() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "a/b/c.sh", "echo c\n");
    assert_eq!(symlinked_component(&root, Path::new("a/b/c.sh")), None);
}

#[test]
fn the_first_symlinked_component_is_the_one_named() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "real/inner/c.sh", "echo c\n");
    link(&root, "real", "outer");
    link(&root, "c.sh", "real/inner/alias.sh");
    assert_eq!(
        symlinked_component(&root, Path::new("outer/inner/alias.sh")),
        Some(PathBuf::from("outer"))
    );
    assert_eq!(
        symlinked_component(&root, Path::new("real/inner/alias.sh")),
        Some(PathBuf::from("real/inner/alias.sh"))
    );
}

#[test]
fn a_path_that_does_not_exist_has_no_symlinked_component() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    assert_eq!(symlinked_component(&root, Path::new("no/such.sh")), None);
}

#[test]
fn an_absolute_path_under_the_root_is_judged_below_the_root_only() {
    // The sandbox itself may sit under a symlink (macOS `/var`); only the
    // components the repository owns are the repository's business.
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    write(&root, "real/c.sh", "echo c\n");
    link(&root, "real", "linked");
    assert_eq!(symlinked_component(&root, &root.join("real/c.sh")), None);
    assert_eq!(
        symlinked_component(&root, &root.join("linked/c.sh")),
        Some(root.join("linked"))
    );
}

#[test]
fn an_absolute_path_outside_the_root_is_judged_by_its_last_component() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("p");
    let elsewhere = sandbox.plain("elsewhere");
    write(&elsewhere, "c.sh", "echo c\n");
    link(&elsewhere, "c.sh", "alias.sh");
    assert_eq!(symlinked_component(&root, &elsewhere.join("c.sh")), None);
    assert_eq!(
        symlinked_component(&root, &elsewhere.join("alias.sh")),
        Some(elsewhere.join("alias.sh"))
    );
}
