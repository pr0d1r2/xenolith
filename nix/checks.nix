# `checks.<sys>`: what `nix flake check` builds (`nix` §I), so CI's flake
# step proves the package and its gate on every tier-1 system, and on `main`
# cachix-action pushes what that step built (`nix:C7`).
#
# Each check is the package derivation with one phase swapped, rather than a
# second `buildRustPackage`: same source filter, same locked crates, so the
# vendored-crate fetches are shared and a check cannot drift onto a source
# set the package does not use.
{ pkgs, package }:
{
  # The package itself, so `nix flake check` BUILDS `packages.default`
  # instead of only evaluating it -- and so the store path cachix receives
  # from `main` is the one consumers substitute.
  build = package;

  # The workspace suite, as the gate runs it (`hk.pkl` step `test`). git is a
  # CHECK input: discovery (`src:V57`) runs `git ls-files`, and its tests
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
  # Nothing to install: the verdict is the build succeeding.
  clippy = package.overrideAttrs (old: {
    pname = "xenolith-clippy";
    nativeBuildInputs = old.nativeBuildInputs ++ [ pkgs.clippy ];
    buildPhase = ''
      runHook preBuild
      cargo clippy --workspace --all-targets --all-features --offline -- -D warnings
      runHook postBuild
    '';
    installPhase = ''
      touch $out
    '';
    dontFixup = true;
  });

  # PENDING: dogfood (`.:C19`, `.:V19`). It is `xnl check`, `xnl graph` and
  # `xnl lint` over this repo. The CLI parses all three but refuses them
  # until their engines land (`src:T153` first), and the green run itself is
  # `.:T28`. Named here rather than stubbed: a check that ran `xnl --version`
  # under the name `dogfood` would pass while proving nothing it claims.
}
