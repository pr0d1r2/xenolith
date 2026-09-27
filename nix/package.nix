# `packages.<sys>.default`: the `xnl` binary, built from the committed
# `Cargo.lock` (`nix` §I). Crates come through `cargoLock.lockFile`, one
# fixed-output fetch per locked crate, so the build needs no vendor hash of
# its own to keep in step with the lock -- the lock IS the pin, and a
# `cargoHash` beside it would be a second pin to forget.
#
# The closure holds rustPlatform's output and nothing from the dev shell:
# itok, microlith, sherd and hk are not arguments here, so they cannot reach
# it (`nix:C6`, `nix:V29`); `checks.closure` proves it (`nix:T38`).
#
# `.override { languages = [ "nix" "pkl" ]; }` builds a subset
# (`nix:C8`, `nix:V31`): only those `lang-*` features, so only those
# grammars and sinks are in the binary.
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

  # The names `languages` accepts: the root crate's `lang-*` features, read
  # from the manifest (`nix` §I), so the list cannot drift from what cargo
  # can build. Sorted, as attribute names always are.
  supported = map (lib.removePrefix "lang-") (
    lib.filter (lib.hasPrefix "lang-") (lib.attrNames manifest.features)
  );
  named = names: lib.concatStringsSep " " names;

  # V31: an unknown name or an empty list is an EVAL error naming every
  # supported language -- never a silent drop, never a binary that claims
  # nothing. Sorted and deduplicated, so `[ "pkl" "nix" ]` and
  # `[ "nix" "pkl" ]` are one derivation and one cache entry.
  choose =
    languages:
    let
      unknown = lib.subtractLists supported languages;
    in
    if languages == [ ] then
      throw "xenolith: `languages` is empty; name at least one of: ${named supported} (nix:V31)"
    else if unknown != [ ] then
      throw "xenolith: unknown language(s) ${named unknown}; supported: ${named supported} (nix:V31)"
    else
      lib.sort lib.lessThan (lib.unique languages);

  build =
    {
      languages ? supported,
    }:
    let
      chosen = choose languages;
    in
    pkgs.rustPlatform.buildRustPackage {
      pname = manifest.package.name;
      inherit (manifest.workspace.package) version;
      inherit src;

      cargoLock.lockFile = ../Cargo.lock;

      # Every build names its features, the default included: `default`
      # in Cargo.toml and `supported` here are then one list read twice,
      # not two lists that agree today.
      buildNoDefaultFeatures = true;
      buildFeatures = map (l: "lang-${l}") chosen;

      # The suite runs as `checks.<sys>.test` (`nix/checks.nix`), not here. A
      # package that tests itself runs the whole workspace suite on every
      # consumer build that misses the cache -- which a language-subset build
      # always does (`nix:C8`) -- to prove what CI already proved on `main`.
      doCheck = false;

      # What this build carries, for the checks that prove it (`nix:V251`).
      passthru.languages = chosen;

      meta = {
        inherit (manifest.package) description;
        homepage = manifest.workspace.package.homepage;
        license = lib.licenses.mit;
        mainProgram = "xnl";
      };
    };
in
lib.makeOverridable build { }
