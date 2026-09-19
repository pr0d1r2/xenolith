# SPEC

## §G GOAL

repo guardrail scripts hk calls: commit-msg, bats mirror, TDD order, private-name denylist.

## §N NAV

rel|path|lens
up|.|-
up|scripts|∀ shell in repo: dev shell hook, guardrail scripts
self|scripts/guard|repo guardrail scripts hk calls

## §C CONSTRAINTS

- C11: TDD. RED commit (`test:` failing `#[test]` | fixture | bats) → GREEN commit (`feat:`|`fix:`) → REFACTOR commit (`refactor:`) ?. test commit ! precede impl commit.
- C12: atomic commits. 1 logical change / commit. Conventional Commits. body ! carry agent reasoning (`Why:` + cite `§V.n`|`§T.n`) → audit trail.
- C17: ⊥ private repo named in source, fixtures, docs, commit msgs. public repos (`nix-hk`, `nixpkgs-lock`, `itok`, `microlith`, `sherd` — verified PUBLIC 2026-09-18; `nix-shebang` — verified PUBLIC 2026-09-19) may be named. unknown = private. fixtures synthetic | anonymised.

## §V INVARIANTS

V16: rule & its checker & its fixtures land in ONE commit; RED test commit precedes (C11).
V20: ∀ commit msg Conventional Commits & body contains `Why:`; hk `commit-msg` enforces (via `xnl`-free script ∵ bootstrap).
V21: ∀ `scripts/**/*.sh` & `.github/scripts/**/*.sh` ∃ bats at mirrored path & vice versa (⊥ orphan test).
V23: ⊥ private repo name in tracked files | commit msgs (C17); hk check against denylist in gitignored file ?.
V117: before the FIRST public push: ∀ commit message & blob in every ref to be pushed scanned against the private denylist (V23); only `main` & release tags pushed; local `backup/*` branches ⊥ pushed.

## §T TASKS

id|status|task|cites
T5|.|`scripts/guard/commit-msg.sh` + bats: Conventional Commits + `Why:`|V20,C12
T6|.|`scripts/guard/bats-mirror.sh` + bats: 1-to-1 `.sh` ↔ `.bats`|V21,C13
T7|.|`scripts/guard/tdd-order.sh` + bats: test commit precedes impl commit (`.rs`, `.sh`)|V16,C11
T29|.|private-name denylist guard (gitignored list) + bats|V23,C17
T47|.|`scripts/guard/crate-deps.sh` + bats: from `cargo metadata`, api ⊥ grammar dep & ⊥ features, language crate ⊥ depends on root \| other language crate; hk pre-push|`languages/api:V32`,C13
T115|.|history audit script (`git log -p` over refs to push vs denylist) + bats; run once before first push|V117,V23

## §B BUGS

id|date|cause|fix
