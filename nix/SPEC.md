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


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
