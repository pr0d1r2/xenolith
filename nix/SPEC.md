# SPEC

## §G GOAL

nix packaging: flake inputs & follows, `packages`, `checks`, devShell, cachix, language-subset override, closure hygiene. `flake.nix` stays at root & imports `./nix/*.nix`.

## §N NAV

rel|path|lens
up|.|-
self|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`

## §V INVARIANTS

V17: flake inputs ! exactly `nixpkgs-lock`, `nix-hk`, `itok`, `microlith`, `sherd` (+ follows per C6); `flake.lock` holds exactly 1 nixpkgs node, rev ≡ nixpkgs-lock rev; check fails otherwise.
V29: `packages.default` closure ∌ itok, microlith, sherd, hk (dev-only inputs, C6).
V31: nix `languages` subset exact: `xnl langs` of subset build lists exactly subset as compiled in; unknown name → eval error listing supported names, ⊥ silent drop; empty list = eval error.

## §T TASKS

id|status|task|cites
T1|.|scaffold flake: inputs nixpkgs-lock + nix-hk w/ follows, devShell (rustc, cargo, clippy, rustfmt, cargo-deny, cargo-llvm-cov ?, hk, bats, shellcheck, shfmt, nixfmt, statix, deadnix), `.gitignore`, `flake.lock`|V17,C6
T26|.|nix package `packages.default`, `checks` (test, clippy, dogfood); cachix push from CI `main`|C7,C19,`.:V19`
T38|.|closure check: `nix path-info -r` of `packages.default` ∌ dev tools|V29,C6
T39|.|resolve C20 cycle policy before itok/microlith/sherd adopt xenolith|C20
T41|.|nix `languages` override arg → cargo features; flake check builds subset `[ "nix" ]` & asserts `xnl langs`; README consumer snippet w/ subset|V31,C8

## §B BUGS

id|date|cause|fix
