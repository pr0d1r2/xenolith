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
let
  tools = import ./tools.nix { inherit pkgs; };
in
pkgs.mkShell {
  # Dev tools, then every language's runtime tools (`nix:V250`): the gate
  # lints this repo's own shell, nix, just and xml with the same pinned
  # binaries the package ships, and a missing tool is `xnl lint` exit 2,
  # not a pass (`src/lint:V8`). Both lists live in `nix/tools.nix`, so the
  # shell and the package cannot disagree on what a tool is.
  packages =
    tools.dev
    ++ builtins.concatMap builtins.attrValues (builtins.attrValues tools.runtime)
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
