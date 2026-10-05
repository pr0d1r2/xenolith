# The two tool lists, drawn ONCE (`nix:V250`): the runtime tools `xnl`
# calls through a compiled-in language's checks and fixers, and the dev
# tools only this repo's own gate uses. The dev shell holds both; the
# package holds the runtime half and must hold none of the other
# (`nix:V29`, `nix:T38`). A tool in both lists would be a contradiction, so
# a tool the package ships is runtime even though the shell has it too.
{ pkgs }:
let
  # sherd, pinned to a crates.io release on purpose (`nix:V351`). A fixed
  # version and two fixed hashes make a fixed derivation, so the cachix
  # push from `main` serves it everywhere the shell is entered. Bumping it
  # is the version and both hashes: set them to `lib.fakeHash`, build, and
  # copy what nix reports. The crate ships its own `Cargo.lock`, which is
  # what `cargoHash` vendors.
  sherd = pkgs.rustPlatform.buildRustPackage rec {
    pname = "sherd";
    version = "0.5.2";
    src = pkgs.fetchCrate {
      inherit pname version;
      hash = "sha256-SmiXquF6oYWWozK/QwqbEPsHzkCbrT2omXRtFwWWwc0=";
    };
    cargoHash = "sha256-693lm6urHneBZjb6hZgfWNy17xbkiXdZXgEEDLvfYAs=";
    # sherd's tests build fixture repositories with `git init`, which the
    # sandbox lacks (`nix:B1` is the same trap in this repo's own checks).
    nativeCheckInputs = [ pkgs.git ];
  };
in
{
  # Keyed by language (the `lang-<l>` feature), then by the command the
  # crate's `checks()` / `fixers()` put in argv[0] -- the name `xnl lint`
  # reports and the fixture check looks for (`nix:V251`). tcl's one tool,
  # `xenolith-tcl-syntax`, is a binary of this repo's own tcl crate, so
  # `nix/package.nix` adds it where the crate is built.
  runtime = {
    # `just --fmt --check`, the just host's check and fixer
    # (`languages/ci/just` §I).
    just = {
      inherit (pkgs) just;
    };
    # The nix host's three, in check and fix modes (`languages/ci/nix`).
    # They gate this repo's own nix files too, which is why the dev shell
    # has always carried them.
    nix = {
      inherit (pkgs) statix deadnix nixfmt;
    };
    # None yet: `pkl format --diff` is still `?` (`languages/api` §I).
    pkl = { };
    # Host checks shellcheck and shfmt; the guest adds checkbashisms for an
    # sh-family extract and `zsh -n` for a zsh one (`src/lint` §I).
    shell = {
      inherit (pkgs)
        shellcheck
        shfmt
        checkbashisms
        zsh
        ;
    };
    # `xmllint --noout` (`languages/data/xml` §I). The `bin` output, so the
    # closure carries the tool and not libxml2's headers.
    xml = {
      xmllint = pkgs.libxml2.bin;
    };
  };

  # Dev shell only. Every name here is one the closure check forbids.
  dev = [
    # The Rust toolchain. The package is BUILT with it, which is exactly why
    # it must not be a runtime reference of what was built.
    pkgs.rustc
    pkgs.cargo
    pkgs.clippy
    pkgs.rustfmt
    # Supply chain in ONE tool: cargo-audit and osv-scanner ask the advisory
    # question this already answers.
    pkgs.cargo-deny
    # Coverage, with the floor recorded in `.coverage` and ratcheted by
    # `sherd coverage` (`scripts:C14`, `scripts:V28`).
    pkgs.cargo-llvm-cov
    pkgs.llvmPackages.llvm
    # Discovery asks git first (`src/discover:V57`), but no check or fixer
    # runs it, so V96 does not ship it: a consumer's repo has its own.
    pkgs.git

    # The gate RUNNER; the ops it runs live in `hk.pkl`, which is the gate of
    # record (`scripts:V122`). From the `nix-hk` overlay rather than nixpkgs:
    # nixpkgs' hk trails the releases `hk.pkl` is written against.
    pkgs.hk

    # The official evaluator for the Pkl gate configuration. The gate must
    # reject syntax that hk's built-in parser accepts only leniently.
    pkgs.pkl

    # bats runs the mirrored tests (`scripts:C13`).
    pkgs.bats

    # The guards that read JSON do it in jq, from their own `.jq` files
    # (`scripts/guard:T47`, `scripts:T114`): `cargo metadata` and `gh api`
    # both answer in JSON, and a hand-rolled parser in shell is a second
    # place for the verdict to be wrong. Pinned here, so the gate never runs
    # whatever jq the machine happens to carry.
    pkgs.jq

    # Every language is a cargo feature (`nix:C8`), so the feature powerset
    # is a real build surface: `cargo hack --each-feature` is what keeps a
    # subset build from breaking in a consumer's tree and nowhere else.
    pkgs.cargo-hack

    # The release runner `release.toml` configures (`nix:V109`), as in every
    # published sibling. It lives here so the runbook runs from the one
    # pinned toolchain; the release is still cut by hand from `main`.
    pkgs.cargo-release

    # Tools for steps that touch files no compiler reads.
    pkgs.editorconfig-checker
    pkgs.taplo
    pkgs.typos
    pkgs.actionlint
    # zizmor AUDITS the workflow actionlint CHECKS -- neither replaces the
    # other: actionlint knows nothing about token scope, zizmor does no
    # syntax checking at all.
    pkgs.zizmor
    # `--offline`, so the gate never reaches the network (`.:C3`): someone
    # else's 404 must not fail a commit, but a relative link that stopped
    # resolving after a file moved must.
    pkgs.lychee
    # Secret shapes `hk util detect-private-key` does not cover -- that
    # builtin does exactly what its name says, and an `AWS_SECRET_ACCESS_KEY=`
    # line walks straight past it.
    pkgs.ripsecrets
  ];

  # The spec toolchain built here rather than taken from a flake input
  # (`nix:V351`). Dev shell only, like `dev`.
  spec = [ sherd ];

  # The spec toolchain's names (`scripts:C21`), for the closure check: itok
  # and microlith arrive from flake inputs (`nix:V17`), sherd from `spec`.
  specNames = [
    "itok"
    "microlith"
    "sherd"
  ];
}
