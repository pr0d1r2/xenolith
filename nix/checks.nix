# `checks.<sys>`: what `nix flake check` builds (`nix` §I), so CI's flake
# step proves the package and its gate on every tier-1 system, and on `main`
# cachix-action pushes what that step built (`nix:C7`).
#
# Each check is the package derivation with one phase swapped, rather than a
# second `buildRustPackage`: same source filter, same locked crates, so the
# vendored-crate fetches are shared and a check cannot drift onto a source
# set the package does not use.
{ pkgs, package }:
let
  inherit (pkgs) lib;
  tools = import ./tools.nix { inherit pkgs; };

  # Both names a derivation goes by in the store: its own name without the
  # version (`rustc-wrapper`) and its pname (`rustc`, the compiler the
  # wrapper holds). Names, not paths: a second build of a dev tool at
  # another path is still a dev tool in the closure.
  names =
    drv:
    lib.unique [
      (builtins.parseDrvName drv.name).name
      (lib.getName drv)
    ];
  devNames = lib.concatMap names tools.dev ++ tools.specNames;

  # `nix path-info -r`, where a sandboxed check can read it: closureInfo's
  # `store-paths`, matched against `forbidden` by `scripts/nix/closure.sh`
  # -- one command, so this file passes its own `xnl check` (`nix:B2`).
  closureCheck =
    name: drv: forbidden:
    pkgs.runCommand name { }
      "bash ${../scripts/nix/closure.sh} ${
        pkgs.closureInfo { rootPaths = [ drv ]; }
      }/store-paths $out ${lib.escapeShellArgs forbidden}";

  # `xnl langs` of a build lists exactly its languages as compiled in
  # (`nix:V31`), through `scripts/nix/langs.sh`.
  langsCheck =
    name: drv:
    pkgs.runCommand name {
      nativeBuildInputs = [ drv ];
    } "bash ${../scripts/nix/langs.sh} $out ${lib.escapeShellArgs drv.languages}";

  # The consumer's subset (`nix:C8`, `nix:T41`): one language, so every
  # other one is provably left out.
  subset = package.override { languages = [ "nix" ]; };

  # V31's refusals are EVAL errors, so they are proven at eval time:
  # `tryEval` catches the `throw`, and forcing `drvPath` forces the list.
  refused = languages: !(builtins.tryEval (package.override { inherit languages; }).drvPath).success;
in
{
  # The package itself, so `nix flake check` BUILDS `packages.default`
  # instead of only evaluating it -- and so the store path cachix receives
  # from `main` is the one consumers substitute.
  build = package;

  # The workspace suite, as the gate runs it (`hk.pkl` step `test`). git is a
  # CHECK input: discovery (`src/discover:V57`) runs `git ls-files`, and its tests
  # build scratch repos, so a sandbox without git fails them (`nix:B1`).
  test = package.overrideAttrs (old: {
    pname = "xenolith-test";
    doCheck = true;
    nativeCheckInputs = (old.nativeCheckInputs or [ ]) ++ [ pkgs.git ];
    cargoTestFlags = [
      "--workspace"
      "--all-features"
    ];
  });

  # Clippy with `-D warnings`, as the gate runs it (`hk.pkl` step `clippy`).
  # Nothing to install: the verdict is the build succeeding. Each phase is
  # ONE command, so this file passes its own `xnl check` (`.:V19`, `nix:B2`):
  # the package sets no pre/postBuild hooks for clippy to honour.
  clippy = package.overrideAttrs (old: {
    pname = "xenolith-clippy";
    nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
    buildPhase = "cargo clippy --workspace --all-targets --all-features --offline -- -D warnings";
    installPhase = "touch $out";
    dontFixup = true;
  });

  # Dogfood (`.:V19`, `.:C19`, `nix:T26`): the PACKAGED `xnl check` over the
  # flake's whole source tree, through `scripts/nix/dogfood.sh` -- one
  # command here, so this file passes the check it runs. The hk `dogfood`
  # step proves the tree's own build; this proves the binary cachix ships.
  # git is an input because discovery asks git first even for an explicit
  # directory, then walks it when the tree is no repository (`src/discover:V57`).
  # `graph` and `lint` join when their engines land (`src/graph:T21`,
  # `src/lint:T24`).
  dogfood = pkgs.runCommand "xenolith-dogfood" {
    nativeBuildInputs = [
      package
      pkgs.git
    ];
  } "bash ${../scripts/nix/dogfood.sh} ${../.} $out";

  # `nix:T38`: the package's closure holds no dev tool (`nix:V29`,
  # `nix:V250`) -- the toolchain it was built with, the gate's runner and
  # linters, the spec tools.
  closure = closureCheck "xenolith-closure" package devNames;

  # `nix:T41`: the default build compiles in every supported language, the
  # `[ "nix" ]` override exactly nix (`nix:V31`, `nix:V251`).
  langs = langsCheck "xenolith-langs" package;
  subset-langs = langsCheck "xenolith-subset-langs" subset;

  # An empty list and an unknown name, alone or beside a good one, are
  # refused rather than built (`nix:V31`).
  subset-refusals =
    assert refused [ ];
    assert refused [ "cobol" ];
    assert refused [
      "nix"
      "cobol"
    ];
    pkgs.runCommand "xenolith-subset-refusals" { } "touch $out";
}
