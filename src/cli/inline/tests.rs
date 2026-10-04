//! `xnl inline`: the mirror of `src/cli/inline.rs` (`src:C139`).
//!
//! Streams and exit codes only; what goes back where is the engine's,
//! tested in `src/extract/inline/tests.rs`, and rendering and writing
//! an edit are `xnl extract`'s, tested in `src/cli/extract/tests.rs`.
//! Every run here refuses before any `git` is asked (`tests:V150`).

use std::path::PathBuf;

use super::{Flags, run};
use crate::discover::{Sandbox, write as put};

fn ran(root: &std::path::Path, extracts: &[&str], flags: Flags) -> (u8, String, String) {
    let extracts: Vec<PathBuf> = extracts.iter().map(PathBuf::from).collect();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let code = run(root, &extracts, flags, &mut out, &mut err);
    (
        code,
        String::from_utf8_lossy(&out).into_owned(),
        String::from_utf8_lossy(&err).into_owned(),
    )
}

#[test]
fn a_config_that_does_not_parse_is_refused() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "xenolith.toml", "version = [\n");
    put(&root, "x.sh", "#!/usr/bin/env bash\necho hi\n");
    let (code, out, err) = ran(&root, &["x.sh"], Flags::default());
    assert_eq!(code, 2, "{err}");
    assert!(out.is_empty());
    assert!(err.contains("xenolith.toml"), "{err}");
}

#[test]
fn a_missing_extract_is_the_engine_s_refusal_naming_the_verb() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    for flags in [
        Flags::default(),
        Flags {
            write: true,
            verbose: true,
            strict_hosts: false,
        },
    ] {
        let (code, out, err) = ran(&root, &["gone.sh"], flags);
        assert_eq!(code, 2, "{err}");
        assert!(out.is_empty());
        assert!(err.starts_with("xnl: inline: "), "{err}");
        assert!(err.contains("gone.sh"), "{err}");
    }
}
