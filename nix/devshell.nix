# The dev shell: the tools CI gets, so a local verdict and a CI verdict are
# the same verdict. Every entry here is PINNED by `flake.lock` -- a tool
# installed by hand is a tool at whatever version that machine happened to
# have, and a gate that disagrees between machines is not a gate.
{
  pkgs,
  # itok, microlith and sherd, built from their own flakes (`scripts:C21`).
  # Dev-shell only: they must never appear in `packages.default`'s closure
  # (`nix:V29`), which is why they arrive as an argument instead of as an
  # input this file resolves for itself.
  specTools,
}:
pkgs.mkShell {
  packages = [
    # The Rust toolchain. No crate exists yet -- the spec is federated before
    # the code (`.:C22`) -- but the gate that will compile it is built first,
    # so the first `src/` commit lands into a shell that can already check it.
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
    pkgs.git

    # The gate RUNNER; the ops it runs live in `hk.pkl`, which is the gate of
    # record (`scripts:V122`). From the `nix-hk` overlay rather than nixpkgs:
    # nixpkgs' hk trails the releases `hk.pkl` is written against.
    pkgs.hk

    # Shell is a first-class language here -- every guardrail is one, and the
    # tool's own subject is embedded shell -- so its three tools are not
    # optional extras: bats runs the mirrored tests (`scripts:C13`),
    # shellcheck and shfmt gate every `.sh` in the repo.
    pkgs.bats
    pkgs.shellcheck
    pkgs.shfmt

    # Nix gates this very file. The flake decides what every other step runs
    # with, so drift here is drift everywhere. statix and deadnix catch what
    # a formatter cannot: a dead `let` binding evaluates fine forever.
    pkgs.nixfmt
    pkgs.statix
    pkgs.deadnix

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
  ]
  ++ specTools;

  # Pin locale so tool output is byte-identical across machines (`.:C3`).
  LANG = "C.UTF-8";
  LLVM_COV = "${pkgs.llvmPackages.llvm}/bin/llvm-cov";
  LLVM_PROFDATA = "${pkgs.llvmPackages.llvm}/bin/llvm-profdata";

  # The hook is READ from `scripts/dev/shell-hook.sh`, never inlined
  # (`scripts:C10`): shell written into this file would be shell that
  # shellcheck, shfmt and bats never see, and the one script that installs
  # the gate is the last place to keep an unchecked rule.
  #
  # Materialized as its own script rather than pasted into the shellHook
  # body, so its `set -euo pipefail` governs that process alone. Pasted, an
  # `-e` would leak into the interactive shell and a typo at the prompt
  # would close the terminal.
  shellHook = "${pkgs.writeShellScript "xenolith-shell-hook" (
    builtins.readFile ../scripts/dev/shell-hook.sh
  )}";
}
