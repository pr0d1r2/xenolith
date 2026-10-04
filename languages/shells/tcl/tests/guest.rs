//! Tcl as a GUEST (`languages/shells/tcl:V197`): the extract file an
//! engine writes from its prelude reads back to the body it held
//! (`languages/api/src/lens:V63`), in both dialects.

use xenolith_lang_api::{Guest, GuestEnv, shebang};
use xenolith_lang_tcl::TclGuest;

#[test]
fn the_prelude_wraps_and_strips_back_to_the_body() {
    let expect = GuestEnv {
        dialect: Some("expect".to_owned()),
        options: Vec::new(),
    };
    for env in [GuestEnv::default(), expect] {
        let prelude = TclGuest.prelude(&env);
        for body in ["puts hi\n", "set x 1\nputs $x\n", ""] {
            let file = shebang::wrap(body, &prelude);
            assert!(file.starts_with("#!/usr/bin/env "), "{file:?}");
            assert_eq!(shebang::strip_strict(&file, &prelude), body, "{env:?}");
        }
    }
}

#[test]
fn an_expect_extract_is_run_by_expect() {
    let expect = GuestEnv {
        dialect: Some("expect".to_owned()),
        options: Vec::new(),
    };
    let ext = TclGuest.extension(&expect);
    let path = format!("scripts/login.{ext}");
    assert_eq!(
        TclGuest.invoke(std::path::Path::new(&path)).argv,
        ["expect", "scripts/login.exp"]
    );
}
