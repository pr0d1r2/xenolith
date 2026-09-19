# SPEC

## §G GOAL

public project docs: `LICENSE`, `AGENTS.md`, `docs/{CODE_OF_CONDUCT,CONTRIBUTING,SECURITY,LLM-DISCLAIMER,THIRD-PARTY-NOTICES}.md` — the set every published sibling (sherd, microlith, itok) ships.

## §N NAV

rel|path|lens
up|.|-
self|docs|public project docs & notices
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure

## §V INVARIANTS

V106: public doc set present & non-empty: `LICENSE` (MIT, `src` C1), `AGENTS.md`, `docs/CODE_OF_CONDUCT.md`, `docs/CONTRIBUTING.md`, `docs/SECURITY.md`, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md`; guard checks presence (`scripts/guard`).

## §T TASKS

id|status|task|cites
T104|.|write `LICENSE` (MIT), CoC, CONTRIBUTING (gate, TDD & commit rules), LLM-DISCLAIMER, mirroring the siblings' texts|V106

## §B BUGS

id|date|cause|fix
