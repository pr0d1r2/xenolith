# SPEC

## §G GOAL

∀ shell in repo: dev shell hook & guardrail scripts, each w/ mirrored bats. + gate config they run under: `hk.pkl` (T4), vendored hk schema `pkl/Config.pkl` it amends, CI `.github/workflows/` (T27), zizmor ledger `.github/zizmor.yml` (T113).

## §F FEDERATION

dir|owns|⊥owns|tokens
guard|repo guardrail scripts hk calls|product rules (`src`), bats (`tests`)|-

## §N NAV

rel|path|lens
up|.|-
self|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices, README & root doc files in the fleet's shape
sib|dev|repo-maintaining tooling, `publish = false`: README generated blocks, third-party notices

## §C CONSTRAINTS

- C9: guardrails = `hk` (from `nix-hk`), `hk.pkl`. ∀ hk step = one plain command a human can paste (`cargo fmt --check`, `xnl check {{files}}`); ⊥ inline shell logic (dogfood `languages/shells/shell:V3`).
- C10: `nix develop` shellHook runs `scripts/dev/shell-hook.sh` → `hk install` idempotent ∴ hooks current ∀ shell enter. shellHook wired via `builtins.readFile`, ⊥ inline.
- C13: any shell in repo ∈ `scripts/` \| `.github/scripts/` w/ 1-to-1 bats at mirrored path (`scripts/a/b.sh` ↔ `tests/unit/scripts/a/b.bats`, `.github/scripts/ci/x.sh` ↔ `tests/unit/.github/scripts/ci/x.bats`); `set -euo pipefail`, shellcheck, shfmt clean.
- C14: Rust coverage via `cargo llvm-cov`, floor in `.coverage`, gated by `sherd coverage --check`, ratchets up only (`--record` refuses drop). lint debt ratchet via `sherd debt --check` vs `.lint-debt`.
- C21: spec toolchain in guardrails: `microlith` (`mth fmt --check`, `mth check` ∀ `SPEC.md`), `itok` (`itok check` vs `.context-limits`), `sherd` (`sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget`, `sherd coverage --check` vs `.coverage`, `sherd debt --check` vs `.lint-debt`; `sherd review` advisory ?). ∀ hk step one plain command (C9) ∴ remediation text in tool output | `scripts/hk/*.sh`, ⊥ inline `\|\| { echo ...; }`.

## §I INTERFACES

- file (this repo): `.context-limits` (itok ceilings), `.coverage` (floor), `.lint-debt` (sherd debt baseline), `hk.pkl` (gate of record, V122), `pkl/Config.pkl` (upstream hk schema, vendored verbatim ∵ `.:C3`; ⊥ hand-edit, bump only w/ pinned hk), `.github/workflows/ci.yml` (calls `hk`, V122), `.github/zizmor.yml` (declines ledger, V114).

## §V INVARIANTS

V22: `cargo fmt --check`, `clippy -D warnings`, `cargo deny check`, `cargo test` green before push; hk `pre-push` enforces.
V25: ∀ `SPEC.md` (root & nodes) pass `mth fmt --check` & `mth check`.
V26: ∀ path ∈ `.context-limits` ≤ ceiling via `itok check`; ceiling raise only in own commit w/ `Why:`.
V27: federation consistent: `sherd validate`, `sherd sync --check`, `sherd check`, `sherd budget` green; §N ⊥ hand-edited.
V28: coverage ≥ `.coverage` floor & lint debt ≤ `.lint-debt` (`sherd coverage --check`, `sherd debt --check`); both ratchet one way.
V114: workflows audited in the gate: `zizmor --persona=pedantic` (declines recorded in `.github/zizmor.yml` w/ reason & exit condition), `actionlint`, link check (`lychee --offline`).
V115: GitHub settings stated & checked, ⊥ assumed: `main` protected, CI jobs required, admins included, Actions may open PRs only if a bot needs it; PRs land by REBASE merge only (squash ⊥, merge commit ⊥) ∴ each RED & GREEN commit reaches `main` (`scripts/guard:B4`); `scripts/guard/github-settings.sh` compares `gh api` output (advisory offline).
V116: ⊥ silent automation: a workflow that pushes a branch ! confirm its PR exists & exit non-zero otherwise; "branch exists → nothing to do" is ⊥ success.
V122: `hk.pkl` = the gate of RECORD & its single definition: CI calls the same `hk` set, ⊥ restates steps; a step exists once. hooks installed by shell entry (C10) & REFUSE when `hk` is off PATH, ⊥ skip silently.
V352: pre-commit `bats` runs only the bats files whose mirrored script (C13) exists (`scripts/hk/bats-ready.sh`) ∴ RED bats commit (C11) possible & ∀ other bats still run per commit; push & `hk check` run the whole suite ∴ a held-back test ⊥ leaves the branch failing. also held back: a bats file staged while its script is ⊥ staged (`git diff --cached`; RED for a change to an EXISTING script, B4); ∄ git ⇒ that rule off (worst case refuses a RED, ⊥ false pass).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication |  | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`.envrc` ⊥ shebang ∴ shellcheck w/o `--shell` (xnl lint via shell host, `src/registry:B11`) → SC2148; only hk passed `--shell=bash`|`# shellcheck shell=bash` directive in `.envrc`
B2|2026-09-28|nix-direnv rebuilds the dev shell only on change to `flake.nix`, `flake.lock`, `.envrc`; devShell lives in `nix/devshell.nix` & `nix/tools.nix` ∴ tool-list change (#3 added `cargo-release`) reached `main` ⊥ any direnv shell → `no such command: release`|`watch_file nix/*.nix` in `.envrc` (direnv directive, ⊥ shell logic ∴ C13 holds)
B3|2026-10-04|`bats` on pre-commit ran the whole suite ∴ a RED bats commit (C11) was refused -- same shape as `scripts/guard:B1`, on the suite rather than the mirror|V352: pre-commit runs only bats whose script exists (T347); push & `hk check` run all
B4|2026-10-04|V352 held back only a bats file whose script ∄ ∴ a RED commit for a change to an EXISTING script was still refused|V352: also hold back a staged bats file whose script is ⊥ staged (T348)
