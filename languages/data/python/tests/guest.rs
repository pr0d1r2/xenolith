//! Python as a GUEST (`languages/data/python:V59`): the extract file an
//! engine writes from its prelude reads back to the body it held
//! (`languages/api/src/lens:V63`).

use xenolith_lang_api::{Guest, GuestEnv, shebang};
use xenolith_lang_python::PythonGuest;

#[test]
fn the_prelude_wraps_and_strips_back_to_the_body() {
    let prelude = PythonGuest.prelude(&GuestEnv::default());
    for body in ["print(1)\n", "import sys\nprint(sys.argv)\n", ""] {
        let file = shebang::wrap(body, &prelude);
        assert!(file.starts_with("#!/usr/bin/env python3\n"), "{file:?}");
        assert_eq!(shebang::strip_strict(&file, &prelude), body);
    }
}

#[test]
fn an_extract_is_run_by_python3() {
    let ext = PythonGuest.extension(&GuestEnv::default());
    let path = format!("scripts/probe.{ext}");
    assert_eq!(
        PythonGuest.invoke(std::path::Path::new(&path)).argv,
        ["python3", "scripts/probe.py"]
    );
}
