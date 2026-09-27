# The two tool lists, drawn ONCE (`nix:V250`): the runtime tools `xnl`
# calls through a compiled-in language's checks and fixers, and the dev
# tools only this repo's own gate uses. The dev shell holds both; the
# package holds the runtime half and must hold none of the other
# (`nix:V29`, `nix:T38`). A tool in both lists would be a contradiction, so
# a tool the package ships is runtime even though the shell has it too.
{ pkgs }:
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

    # bats runs the mirrored tests (`scripts:C13`).
    pkgs.bats

    # Every language is a cargo feature (`nix:C8`), so the feature powerset
    # is a real build surface: `cargo hack --each-feature` is what keeps a
    # subset build from breaking in a consumer's tree and nowhere else.
    pkgs.cargo-hack

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

  # The spec toolchain arrives from flake inputs (`scripts:C21`), so its
  # derivations are not in `pkgs`; its names are fixed by `nix:V17`, which
  # pins exactly these inputs.
  specNames = [
    "itok"
    "microlith"
    "sherd"
  ];
}
