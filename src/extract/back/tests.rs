//! Reading a load back (`src/extract:V270`), over the toy host of
//! `src/extract/tests.rs`: `<sink>< <argv…>` is a load of its last
//! word, `<sink>=<guest>: <body>` a site, placed at
//! `<host dir>/<host stem>/<sink>.sh`; the `sh` guest's prelude is
//! `#!/usr/bin/env sh` alone and `&&` makes a body non-trivial.

use std::path::Path;

use xenolith_lang_api::{Host, LoadRef};

use super::{Back, again, back, loaders, trivial};
use crate::config::{self, Config, Tree};
use crate::discover::{Sandbox, write as put};
use crate::extract::toys;

fn config(toml: &str) -> Config {
    config::parse(toml).unwrap_or_else(|e| panic!("{e}"))
}

fn host() -> &'static dyn Host {
    toys()
        .hosts
        .first()
        .copied()
        .unwrap_or_else(|| panic!("no toy host"))
}

/// The one load in `name`.
fn load_of(root: &Path, name: &str) -> (String, LoadRef) {
    let src = std::fs::read_to_string(root.join(name)).unwrap_or_else(|e| panic!("{e}"));
    let loads = host().loads(&src).unwrap_or_else(|e| panic!("{e}"));
    let [load] = loads.as_slice() else {
        panic!("{loads:?}")
    };
    (src.clone(), load.clone())
}

fn read_back<'a>(root: &Path, tree: &Tree, extract: &str) -> Result<Back<'a>, String> {
    let (src, load) = load_of(root, "a.toy");
    back(root, tree, &toys(), "a.toy", host(), &src, &load, extract)
}

fn sandbox_with(extract: &str, text: &str) -> (Sandbox, std::path::PathBuf) {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", &format!("build< sh ./{extract}\n"));
    put(&root, extract, text);
    (sandbox, root)
}

#[test]
fn a_load_reads_back_to_its_body_its_site_and_its_placement() {
    let (_sandbox, root) = sandbox_with("a/build.sh", "#!/usr/bin/env sh\nmake && make test\n");
    let tree = Tree::new(Config::default());
    let read = read_back(&root, &tree, "a/build.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read.body, "make && make test\n", "the prelude is not body");
    assert_eq!(read.inlined, "build=shell: make && make test\n");
    assert_eq!(read.planned.site.sink, "build");
    assert_eq!(read.planned.placed.path, "a/build.sh");
    assert_eq!(read.misplaced(), None, "it is where the config puts it");
    assert!(!read.trivial(&Config::default()));
}

#[test]
fn a_layout_change_makes_it_misplaced_naming_the_new_path() {
    let (_sandbox, root) = sandbox_with("a/build.sh", "#!/usr/bin/env sh\nmake && make test\n");
    let tree = Tree::new(config("version = 1\n[extract]\nlayout = \"central\"\n"));
    let read = read_back(&root, &tree, "a/build.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read.misplaced(), Some("scripts/shell/build.sh"));
}

#[test]
fn the_collision_suffixed_path_counts_as_placed() {
    // `src/extract:V47`: a run where two sites met gave `build-build.sh`.
    let (_sandbox, root) = sandbox_with("a/build-build.sh", "#!/usr/bin/env sh\nmake && b\n");
    let tree = Tree::new(Config::default());
    let read = read_back(&root, &tree, "a/build-build.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read.misplaced(), None);
}

#[test]
fn a_load_that_cannot_be_read_back_says_why() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    put(&root, "a.toy", "build< sh ./a/build.sh\n");
    let tree = Tree::new(Config::default());
    let err = read_back(&root, &tree, "a/build.sh")
        .err()
        .unwrap_or_default();
    assert!(err.contains("a/build.sh cannot be read"), "{err}");
}

#[test]
fn trivial_is_check_s_verdict_threshold_included() {
    let sh = toys()
        .guests
        .first()
        .copied()
        .unwrap_or_else(|| panic!("no guest"));
    let env = xenolith_lang_api::GuestEnv::default();
    let plain = Config::default();
    assert!(trivial(sh, "echo hi\n", &env, &plain));
    assert!(!trivial(sh, "a && b\n", &env, &plain));
    let relaxed = config("version = 1\n[threshold.shell]\nallow = [\"and-or\"]\n");
    assert!(trivial(sh, "a && b\n", &env, &relaxed), "src/config:V55");
    assert!(
        !trivial(sh, "a && b | c\n", &env, &relaxed),
        "pipeline is not allowed"
    );
}

/// The real languages, as the registry of this build has them.
#[cfg(all(feature = "lang-just", feature = "lang-shell"))]
fn registry_langs() -> crate::check::Langs<'static> {
    crate::check::Langs {
        hosts: crate::registry::hosts(),
        guests: crate::registry::guests(),
    }
}

/// The just host of this build.
#[cfg(all(feature = "lang-just", feature = "lang-shell"))]
fn just() -> &'static dyn Host {
    crate::registry::hosts()
        .iter()
        .copied()
        .find(|h| h.id() == xenolith_lang_api::LangId::Just)
        .unwrap_or_else(|| panic!("no just host"))
}

/// `src/extract:B1`: a just site opens at its recipe's header, before
/// the load line its body goes back into; the site whose body holds the
/// put-back lines is the one the load was.
#[cfg(all(feature = "lang-just", feature = "lang-shell"))]
#[test]
fn a_just_load_reads_back_to_its_recipe() {
    let sandbox = Sandbox::new();
    let root = sandbox.plain("r");
    let src = "build:\n    bash scripts/build.sh\n";
    put(&root, "justfile", src);
    put(&root, "scripts/build.sh", "make && make test\n");
    let tree = Tree::new(Config::default());
    let loads = just().loads(src).unwrap_or_else(|e| panic!("{e}"));
    let [load] = loads.as_slice() else {
        panic!("{loads:?}")
    };
    let read = back(
        &root,
        &tree,
        &registry_langs(),
        "justfile",
        just(),
        src,
        load,
        "scripts/build.sh",
    )
    .unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(read.inlined, "build:\n    make && make test\n");
}

#[test]
fn again_proves_the_bytes_and_refuses_a_file_it_would_rewrite() {
    let (_sandbox, root) = sandbox_with("a/build.sh", "#!/usr/bin/env sh\nmake && make test\n");
    let tree = Tree::new(Config::default());
    let mut read = read_back(&root, &tree, "a/build.sh").unwrap_or_else(|e| panic!("{e}"));
    let edit = again(&root, "a.toy", &mut read, "a/build.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        edit.after, "build< sh ./a/build.sh\n",
        "the host comes back"
    );
    let moved = again(&root, "a.toy", &mut read, "b/build.sh").unwrap_or_else(|e| panic!("{e}"));
    assert_eq!(
        moved.after, "build< sh ./b/build.sh\n",
        "the load follows the path"
    );
    // A shebang the prelude does not write: extracting again would
    // rewrite it, so neither a move nor an inline is exact.
    let (_sandbox, root) = sandbox_with("a/build.sh", "#!/bin/dash\nmake && make test\n");
    let mut read = read_back(&root, &tree, "a/build.sh").unwrap_or_else(|e| panic!("{e}"));
    let err = again(&root, "a.toy", &mut read, "b/build.sh")
        .err()
        .unwrap_or_default();
    assert!(err.contains("src/extract:V270"), "{err}");
}

#[test]
fn loaders_counts_every_load_of_an_extract_in_the_tree() {
    let sandbox = Sandbox::new();
    let root = sandbox.repo("r");
    put(&root, "a.toy", "build< sh ./x.sh\n");
    put(&root, "b.toy", "one< sh ./x.sh\ntwo< sh ./y.sh\n");
    put(&root, "x.sh", "#!/usr/bin/env sh\na && b\n");
    put(&root, "y.sh", "#!/usr/bin/env sh\na && b\n");
    sandbox.run_git(&root, &["add", "."]);
    let found = loaders(&root, &Config::default(), false, &toys(), &|| sandbox.git())
        .unwrap_or_else(|e| panic!("{e}"));
    let hosts = |extract: &str| -> Vec<String> {
        found
            .get(extract)
            .map(|v| v.iter().map(|(h, l, _)| format!("{h}:{l}")).collect())
            .unwrap_or_default()
    };
    assert_eq!(hosts("x.sh"), ["a.toy:1", "b.toy:1"]);
    assert_eq!(hosts("y.sh"), ["b.toy:2"]);
}
