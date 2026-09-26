# `packages.<sys>.default`: the `xnl` binary, built from the committed
# `Cargo.lock` (`nix` §I). Crates come through `cargoLock.lockFile`, one
# fixed-output fetch per locked crate, so the build needs no vendor hash of
# its own to keep in step with the lock -- the lock IS the pin, and a
# `cargoHash` beside it would be a second pin to forget.
#
# The closure holds rustPlatform's output and nothing from the dev shell:
# itok, microlith, sherd and hk are not arguments here, so they cannot reach
# it (`nix:C6`, `nix:V29`).
{ pkgs }:
let
  inherit (pkgs) lib;
  fs = lib.fileset;

  # Only what cargo reads. A SPEC.md edit, a bats file or a guard script
  # changes nothing cargo compiles, so it must not change the source hash
  # either -- otherwise every spec commit is a cache miss for every
  # consumer. `tests/unit/` is bats (`tests` §G), not a cargo test.
  src = fs.toSource {
    root = ../.;
    fileset =
      fs.difference
        (fs.unions [
          ../Cargo.toml
          ../Cargo.lock
          ../clippy.toml
          ../rustfmt.toml
          ../src
          ../languages
          ../tests
        ])
        (
          fs.unions [
            ../tests/unit
            (fs.fileFilter (file: file.name == "SPEC.md") ../.)
          ]
        );
  };

  manifest = lib.importTOML ../Cargo.toml;
in
pkgs.rustPlatform.buildRustPackage {
  pname = manifest.package.name;
  inherit (manifest.workspace.package) version;
  inherit src;

  cargoLock.lockFile = ../Cargo.lock;

  # The suite runs as `checks.<sys>.test` (`nix/checks.nix`), not here. A
  # package that tests itself runs the whole workspace suite on every
  # consumer build that misses the cache -- which a language-subset build
  # always does (`nix:C8`) -- to prove what CI already proved on `main`.
  doCheck = false;

  meta = {
    inherit (manifest.package) description;
    homepage = manifest.workspace.package.homepage;
    license = lib.licenses.mit;
    mainProgram = "xnl";
  };
}
