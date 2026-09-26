# SPEC

## §G GOAL

public project docs: `LICENSE`, `AGENTS.md`, `docs/{CODE_OF_CONDUCT,CONTRIBUTING,SECURITY,LLM-DISCLAIMER,THIRD-PARTY-NOTICES}.md` — the set every published sibling (sherd, microlith, itok) ships.

## §N NAV

rel|path|lens
up|.|-
self|docs|public project docs & notices
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure

## §V INVARIANTS

V106: public doc set present & non-empty: `LICENSE` (MIT, `src` C1), `AGENTS.md`, `docs/CODE_OF_CONDUCT.md`, `docs/CONTRIBUTING.md`, `docs/SECURITY.md`, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md`; guard checks presence (`scripts/guard`).
V107: `docs/SECURITY.md` names every trust boundary — `--trust-config` (`src/lint:V91`), symlink-safe writes & reads (`src/extract:V71`, `src/graph:V72`), path charset (`src/extract:V83`), fixtures synthetic (C17) — & a private reporting channel.
V108: `docs/THIRD-PARTY-NOTICES.md` GENERATED, ⊥ hand-written: ∀ crate in the dependency tree (esp. every tree-sitter grammar) w/ its license from `cargo metadata`, & ∀ tool the nix package wraps (`nix:V96`, e.g. shellcheck GPL-3 — aggregation, ⊥ linking) w/ its license from nixpkgs meta; drift = gate failure.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication | T104-T107 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T104|.|write `LICENSE` (MIT), CoC, CONTRIBUTING (gate, TDD & commit rules), LLM-DISCLAIMER, mirroring the siblings' texts|V106
T105|.|write `docs/SECURITY.md` per V107, incl. CI trust boundary (fork PRs take config from base branch)|V107
T106|.|notices generator + drift check (cargo metadata + nix eval of wrapped tools' `meta.license`)|V108
T107|.|`AGENTS.md`: the gate (`hk check --all`), never `--no-verify`, TDD & atomic commits w/ `Why:` (`scripts/guard` C11, C12), spec-first flow, PR-per-change & pause for review|V106

## §B BUGS

id|date|cause|fix
