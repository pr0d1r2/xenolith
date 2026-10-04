# SPEC

## §G GOAL

repo guardrail scripts hk calls: commit-msg, bats mirror, rust mirror, TDD order, private-name denylist, node config (`.:V90`).

## §N NAV

rel|path|lens
up|.|-
up|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
self|scripts/guard|repo guardrail scripts hk calls

## §C CONSTRAINTS

- C11: TDD. RED commit (`test:` failing `#[test]` | fixture | bats) → GREEN commit (`feat:`|`fix:`) → REFACTOR commit (`refactor:`) ?. test commit ! precede impl commit.
- C12: atomic commits. 1 logical change / commit. Conventional Commits. body ! carry agent reasoning (`Why:` + cite `§V.n`|`§T.n`) → audit trail.
- C17: ⊥ private repo named in source, fixtures, docs, commit msgs. public repos (`nix-hk`, `nixpkgs-lock`, `itok`, `microlith`, `sherd` — verified PUBLIC 2026-09-18; `nix-shebang` — verified PUBLIC 2026-09-19) may be named. unknown = private. fixtures synthetic | anonymised.

## §V INVARIANTS

V16: rule & its checker & its fixtures land in ONE commit; RED test commit precedes (C11).
V20: ∀ commit msg Conventional Commits & body contains `Why:`; hk `commit-msg` enforces (via `xnl`-free script ∵ bootstrap).
V21: ∀ `scripts/**/*.sh` & `.github/scripts/**/*.sh` ∃ bats at mirrored path & vice versa (⊥ orphan test).
V140: ∀ tracked `.rs` w/ logic (∃ fn body, `src:C139`) ∃ sibling `tests.rs` wired by `#[cfg(test)] mod tests;` & vice versa (⊥ orphan, ⊥ unwired). checked per branch ⊥ per commit (B1). exempt, closed list (new entry = row here w/ reason): `**/build.rs` (build script, ⊥ crate code); `languages/ci/pkl/src/grammar.rs` (FFI shim over vendored C, `languages:V121`; covered by crate `tests/`); `**/tests.rs` (the mirrors); `tests/**` & `<crate>/tests/**` (integration: allowed, ⊥ satisfy V140); `**/vendor/**` (verbatim upstream); `lib.rs` of pure wiring (mod decls, re-exports, ⊥ fn body); `main.rs` shim (`fn main` → lib only, `src:C139`).
V23: ⊥ private repo name in tracked files | commit msgs (C17); hk check against denylist in gitignored file ?. entry matches as WHOLE word: fixed string, ignore case, ⊥ `[A-Za-z0-9_]` adjacent either side (`git grep -w`), in content & path; ⊥ regex from list.
V117: before the FIRST public push: ∀ commit message & blob in every ref to be pushed scanned against the private denylist, matched as V23 says; only `main` & release tags pushed; local `backup/*` branches ⊥ pushed.
V353: `tdd-order` exemption = closed list `scripts/guard/tdd-order.exempt`: full commit id + reason naming where its RED-first history lives; entry only for a commit already on `main` that cannot be rewritten; new entry = §B row here.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T115, T350 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T115|.|`scripts/guard/history-audit.sh [REF…]` + bats: default refs `main` + ∀ tag, any other ref refused (V117); scans ref names, annotated-tag msgs, `git log -p -m --text --format=fuller` (∀ msg, path, blob, merge) vs `.private-names` (`--denylist FILE`); report = denylist line, hit count, commit ids, ⊥ name; ∄ list \| ∄ pattern → FAIL (⊥ vacuous pass); run once before first push|V117,V23
T350|.|`scripts/guard/tdd-order.sh [--exempt FILE] [RANGE]` + bats: commit in the list (default `tdd-order.exempt` beside the script) skipped; malformed line, short id, ∄ list ⇒ fail (⊥ vacuous pass); `6783bb9` listed (B4)|V353,C11

## §B BUGS

id|date|cause|fix
B1|2026-09-20|`bats-mirror` ran on pre-commit ∴ ⊥ RED commit possible: C11 test commit precedes its script, so orphan test = method working, ⊥ defect. V21 read as per-commit ⊥ per-branch|step moved to `all` (pre-push, `hk check`); mirror = property of branch, checked before push when GREEN exists
B2|2026-09-27|`--no-renames` listed a moved file as added ∴ moving a crate under a hub (`languages/shells/shell:V137`) read as Rust w/o a prior `test:` commit|`git show -M`: rename (≥50% similar) ≠ added; rewritten beyond that = new code
B3|2026-09-27|denylist entry matched as substring (tree guard & history audit) ∴ short name hit inside unrelated words & paths, guard unusable|whole-word match (V23): `git grep -w` for content, word check for paths & history; paths read unquoted (`ls-files -z`, `log` w/ `core.quotepath=off`) ∵ octal escape puts digit before name
B4|2026-10-04|squash-only merge (#39 → `6783bb9`) collapsed RED & GREEN into one commit ∴ `tdd-order` over `main` history fails; RED-first commits survive only in PR #39|rebase merge only (`scripts:V115`); `6783bb9` in the closed exemption list (V353, T350)
