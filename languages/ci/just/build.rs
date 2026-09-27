//! Builds the vendored `tree-sitter-just` grammar (`languages:V121`).
//!
//! The C is the published crate's generated output, copied verbatim at
//! the version and rev named in `vendor/tree-sitter-just/UPSTREAM`.
//! Nothing here generates it: a build that needed the tree-sitter CLI and
//! node would need a network, and the build is offline (`.:C3`).

fn main() {
    let src = std::path::Path::new("vendor/tree-sitter-just/src");
    let parser = src.join("parser.c");
    let scanner = src.join("scanner.c");

    cc::Build::new()
        .std("c11")
        .include(src)
        .file(&parser)
        .file(&scanner)
        // Generated tables trip warnings that are upstream's to fix, and a
        // warning printed on every build is one nobody reads.
        .warnings(false)
        .compile("tree-sitter-just");

    println!("cargo:rerun-if-changed={}", parser.display());
    println!("cargo:rerun-if-changed={}", scanner.display());
    println!(
        "cargo:rerun-if-changed={}",
        src.join("tree_sitter").display()
    );
}
