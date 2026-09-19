# SPEC

## §G GOAL

repo guardrail scripts hk calls: commit-msg, bats mirror, TDD order, private-name denylist.

## §N NAV

rel|path|lens
up|.|-
up|scripts|∀ shell in repo: dev shell hook, guardrail scripts
self|scripts/guard|repo guardrail scripts hk calls

## §V INVARIANTS

V20: ∀ commit msg Conventional Commits & body contains `Why:`; hk `commit-msg` enforces (via `ldt`-free script ∵ bootstrap).
V21: ∀ `scripts/**/*.sh` ∃ bats at mirrored path & vice versa (⊥ orphan test).
V23: ⊥ private repo name in tracked files | commit msgs (C17); hk check against denylist in gitignored file ?.

## §T TASKS

id|status|task|cites
T5|.|`scripts/guard/commit-msg.sh` + bats: Conventional Commits + `Why:`|V20,C12
T6|.|`scripts/guard/bats-mirror.sh` + bats: 1-to-1 `.sh` ↔ `.bats`|V21,C13
T7|.|`scripts/guard/tdd-order.sh` + bats: test commit precedes impl commit (`.rs`, `.sh`)|`.:V16`,C11
T29|.|private-name denylist guard (gitignored list) + bats|V23,C17
T47|.|`scripts/guard/crate-deps.sh` + bats: from `cargo metadata`, api ⊥ grammar dep & ⊥ features, language crate ⊥ depends on root | other language crate; hk pre-push|`languages/api:V32`,C13

## §B BUGS

id|date|cause|fix
