//! One graph run: the mirror of `src/graph/scan.rs` (`src:C139`).
//!
//! The run itself is driven end to end by `src/graph/tests.rs`; what is
//! pinned here are the small readers it stands on: which guest reads a
//! file, where a byte offset is, and what the first line of a file is.

use std::path::Path;

use xenolith_lang_api::{Guest, GuestEnv, Invoke, LangId, LintCmd, Prelude, Result};

use super::{head, position, reader};
use crate::discover::{Sandbox, write};

struct Sh;

impl Guest for Sh {
    fn id(&self) -> LangId {
        LangId::Shell
    }
    fn extension(&self, _: &GuestEnv) -> &'static str {
        "sh"
    }
    fn invoke(&self, path: &Path) -> Invoke {
        Invoke {
            argv: vec![path.display().to_string()],
        }
    }
    fn trivial(&self, _: &str) -> Result<bool> {
        Ok(false)
    }
    fn prelude(&self, _: &GuestEnv) -> Prelude {
        Prelude {
            shebang: None,
            strict: None,
        }
    }
    fn executable(&self) -> bool {
        true
    }
    fn checks(&self, _: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
    fn fixers(&self, _: &GuestEnv) -> Vec<LintCmd> {
        Vec::new()
    }
}

const GUESTS: &[&dyn Guest] = &[&Sh];

#[test]
fn the_extension_names_the_reader_without_a_shebang() {
    assert_eq!(reader(GUESTS, Path::new("a/b.sh"), ""), Some(LangId::Shell));
    assert_eq!(reader(GUESTS, Path::new("a/SPEC.md"), "# SPEC"), None);
    assert_eq!(reader(GUESTS, Path::new("Makefile"), ""), None);
}

#[test]
fn a_shebang_decides_over_the_extension() {
    let bash = "#!/usr/bin/env bash";
    assert_eq!(reader(GUESTS, Path::new("tool"), bash), Some(LangId::Shell));
    // A shebang naming a guest this build lacks is no extract, whatever
    // the extension says.
    let python = "#!/usr/bin/env python3";
    assert_eq!(reader(GUESTS, Path::new("x.sh"), python), None);
}

#[test]
fn a_position_is_one_based_and_counts_characters() {
    assert_eq!(position("abc", 0), (1, 1));
    assert_eq!(position("a\nbc", 3), (2, 2));
    assert_eq!(position("é\nxé!", 6), (2, 3));
    // An offset past the end, or inside a character, still answers.
    assert_eq!(position("ab", 99), (1, 3));
}

#[test]
fn head_is_the_first_line_only() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("t");
    write(&root, "a", "#!/bin/sh\necho\n");
    assert_eq!(head(&root.join("a")), "#!/bin/sh");
    assert_eq!(head(&root.join("missing")), "");
}
