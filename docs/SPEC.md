# SPEC

## §G GOAL

public project docs: `README.md`, `LICENSE`, `AGENTS.md`, `CHANGELOG.md`, `docs/{CODE_OF_CONDUCT,CONTRIBUTING,SECURITY,LLM-DISCLAIMER,THIRD-PARTY-NOTICES,INTEGRATION}.md` -- the set & shape every published sibling (sherd, microlith, itok) ships (`dev:R340`). `docs/MIGRATION.md` is xenolith's own.

## §N NAV

rel|path|lens
up|.|-
self|docs|public project docs & notices, README & root doc files in the fleet's shape
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|dev|repo-maintaining tooling, `publish = false`: README generated blocks, third-party notices

## §V INVARIANTS

V106: public doc set present & non-empty: `LICENSE` (MIT, `src` C1), `AGENTS.md`, `docs/CODE_OF_CONDUCT.md`, `docs/CONTRIBUTING.md`, `docs/SECURITY.md`, `docs/LLM-DISCLAIMER.md`, `docs/THIRD-PARTY-NOTICES.md`; guard checks presence (`scripts/guard`).
V107: `docs/SECURITY.md` names every trust boundary -- `--trust-config` (`src/lint:V91`), symlink-safe writes & reads (`src/extract:V71`, `src/graph:V72`), path charset (`src/extract:V83`), fixtures synthetic (C17) -- & a private reporting channel.
V108: `docs/THIRD-PARTY-NOTICES.md` GENERATED, ⊥ hand-written: ∀ crate in the dependency tree (esp. every tree-sitter grammar) w/ its license from `cargo metadata`, & ∀ tool the nix package wraps (`nix:V96`, e.g. shellcheck GPL-3 -- aggregation, ⊥ linking) w/ its license from nixpkgs meta; drift = gate failure.
V280: docs claim only the tree: ∀ `xnl` verb, flag, `xenolith.toml` key, flake output & file a doc names exists at the doc's commit (read from code \| run, ⊥ from spec); planned = marked planned w/ its task id. prose ⊥ RED (⊥ checker) ∴ verified by e2e of each shown command at write time + `lychee --offline`.
V281: consumer migration doc (`.:T32`) = `docs/MIGRATION.md`: legacy `.<lang>-embedded-shell-allowlist` → `xenolith.toml` via `xnl migrate`; fleet hooks `xnl` replaces (justfile allowlist `languages/ci/just:R178`, `xmllint` `languages/data/xml:R186`, tcl `languages/shells/tcl:R193`); just delta named (`languages/ci/just:V181`, `languages/ci/just:R207`).
V348: README & docs in the fleet shape (`dev:R340`). README: `# name` → generated `badges` block → "Read LLM-DISCLAIMER first." (linking `docs/LLM-DISCLAIMER.md`) → the problem in one measured number (counts from §R rows only, ⊥ repo names, `scripts/guard` C17) → what `xnl` does → the siblings' sections, order & tone. `docs/` headings = the siblings' (CoC, CONTRIBUTING, LLM-DISCLAIMER, SECURITY), adapted ⊥ copied. `docs/INTEGRATION.md` = the gate (one definition, three callers, what runs on which files, reproduce w/o hk, release runbook `nix:V109`, known gaps) + consumer wiring (`xnl` in hk, lefthook, a flake). ∀ number either GENERATED (`dev:V340`) or a cited §R count, ⊥ typed from memory (V280).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication |  | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
