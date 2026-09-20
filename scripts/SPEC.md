# SPEC

## §G GOAL

∀ shell in repo: dev shell hook & guardrail scripts, each w/ mirrored bats.

## §F FEDERATION

dir|owns|⊥owns|tokens
guard|repo guardrail scripts hk calls|product rules (`src`), bats (`tests`)|-

## §N NAV

rel|path|lens
up|.|-
self|scripts|∀ shell in repo: dev shell hook, guardrail scripts
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices

## §C CONSTRAINTS

- C9: guardrails = `hk` (from `nix-hk`), `hk.pkl`. ∀ hk step = one plain command a human can paste (`cargo fmt --check`, `xnl check {{files}}`); ⊥ inline shell logic (dogfood `languages/shell:V3`).
- C10: `nix develop` shellHook runs `scripts/dev/shell-hook.sh` → `hk install` idempotent ∴ hooks current ∀ shell enter. shellHook wired via `builtins.readFile`, ⊥ inline.
- C13: any shell in repo ∈ `scripts/` \| `.github/scripts/` w/ 1-to-1 bats at mirrored path (`scripts/a/b.sh` ↔ `tests/unit/scripts/a/b.bats`, `.github/scripts/ci/x.sh` ↔ `tests/unit/.github/scripts/ci/x.bats`); `set -euo pipefail`, shellcheck, shfmt clean.
- C14: Rust coverage via `cargo llvm-cov`, floor in `.coverage`, gated by `sherd coverage --check`, ratchets up only (`--record` refuses drop). lint debt ratchet via `sherd debt --check` vs `.lint-debt`.
- C21: spec toolchain in guardrails: `microlith` (`mth fmt --check`, `mth check` ∀ `SPEC.md`), `itok` (`itok check` vs `.context-limits`), `sherd` (`sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget`, `sherd coverage --check` vs `.coverage`, `sherd debt --check` vs `.lint-debt`; `sherd review` advisory ?). ∀ hk step one plain command (C9) ∴ remediation text in tool output | `scripts/hk/*.sh`, ⊥ inline `\|\| { echo …; }`.

## §I INTERFACES

- file (this repo): `.context-limits` (itok ceilings), `.coverage` (floor), `.lint-debt` (sherd debt baseline).

## §V INVARIANTS

V22: `cargo fmt --check`, `clippy -D warnings`, `cargo deny check`, `cargo test` green before push; hk `pre-push` enforces.
V25: ∀ `SPEC.md` (root & nodes) pass `mth fmt --check` & `mth check`.
V26: ∀ path ∈ `.context-limits` ≤ ceiling via `itok check`; ceiling raise only in own commit w/ `Why:`.
V27: federation consistent: `sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget` green; §N ⊥ hand-edited.
V28: coverage ≥ `.coverage` floor & lint debt ≤ `.lint-debt` (`sherd coverage --check`, `sherd debt --check`); both ratchet one way.
V114: workflows audited in the gate: `zizmor --persona=pedantic` (declines recorded in `.github/zizmor.yml` w/ reason & exit condition), `actionlint`, link check (`lychee --offline`).
V115: GitHub settings stated & checked, ⊥ assumed: `main` protected, CI jobs required, admins included, Actions may open PRs only if a bot needs it; `scripts/guard/github-settings.sh` compares `gh api` output (advisory offline).
V116: ⊥ silent automation: a workflow that pushes a branch ! confirm its PR exists & exit non-zero otherwise; "branch exists → nothing to do" is ⊥ success.
V122: `hk.pkl` = the gate of RECORD & its single definition: CI calls the same `hk` set, ⊥ restates steps; a step exists once. hooks installed by shell entry (C10) & REFUSE when `hk` is off PATH, ⊥ skip silently.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T2, T4, T27, T33-T36, T121 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T113, T114 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T2|x|`scripts/dev/shell-hook.sh` + bats (RED→GREEN): idempotent `hk install`, wired via `builtins.readFile`|C10,`scripts/guard:V21`
T4|x|`hk.pkl`: fmt, clippy, deny, test, bats, cargo-hack, shellcheck, shfmt, nixfmt, statix, deadnix, taplo, typos, editorconfig, trailing-whitespace, final-newline, no-large-files, secret scan, link check; commit-msg & pre-push hooks|C9,V22,V122
T27|.|CI workflow: tier-1 matrix, `hk check --all`, bats, `nix flake check`, cachix|C7,V22
T33|x|hk steps `mth fmt --check` & `mth check` ∀ `SPEC.md`; `mth fmt` as fix|V25,C21
T34|x|`.context-limits` ceilings + hk `itok check`|V26,C21
T35|x|hk steps `sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget`; `sherd review` advisory ?|V27,C21
T36|.|`.coverage` floor + `.lint-debt` baseline; hk pre-push `sherd coverage --check`, `sherd debt --check` (supersedes C14 llvm-cov wiring in T1)|V28,C14
T113|.|hk steps zizmor, actionlint, lychee + `.github/zizmor.yml` ledger|V114
T114|.|`github-settings.sh` + bats; runbook lists the intended settings|V115
T121|x|repo hygiene files: `.envrc` (`use flake`), `.editorconfig`, `.typos.toml`, `.taplo.toml`, `.gitignore`|V122,C10

## §B BUGS

id|date|cause|fix
