# SPEC

## §G GOAL

tooling that maintains THIS repository & ships to nobody: `xenolith-dev`, a `publish = false` workspace member. regenerates the README's generated blocks (badges, languages) & `docs/THIRD-PARTY-NOTICES.md` from the files that OWN each fact; `--check` refuses a stale one. shape & conventions = sherd's `dev/` (`sherd-dev`), the one dev crate in the fleet (R340).

## §N NAV

rel|path|lens
up|.|-
self|dev|repo-maintaining tooling, `publish = false`: README generated blocks, third-party notices
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|docs|public project docs & notices, README & root doc files in the fleet's shape

## §C CONSTRAINTS

- ⊥ published, ⊥ packaged: `publish = false`; `packages.default` builds `-p xenolith` only (`nix:V349`) ∴ a consumer ⊥ ever receives these verbs.
- lib + shim (`src:C139`): `main.rs` = `fn main` only; ∀ rule = pure fn over `&str` in the lib, inputs read by the CALLER & handed in ∴ ∀ rule testable from a string literal w/o a repository.
- exit codes = `src/cli:V24`: 0 clean · 1 stale | violation · 2 usage. a 2nd convention for one thing = duplication.
- deps: workspace crates + `serde_json` only (already in `Cargo.lock`) ∴ ⊥ new crate in the lock for a tool nobody installs.
- ⊥ reimpl an owner: language ids = `xenolith_lang_api::LangId::ALL`, features & default set = root `Cargo.toml`, runtime tools & their licenses = the nix package's `passthru.toolLicenses` (`nix eval`), crate licenses = `cargo metadata`.

## §I INTERFACES

- cmd: `xenolith-dev --check [<path>...]` → ∀ check (readme, notices) concurrently; paths narrow to what they can have invalidated (V345); ⊥ paths = compare ∀.
- cmd: `xenolith-dev --fix [<path>...]` → regenerate what `--check` would refuse; hk `fix`.
- cmd: `xenolith-dev readme [--check] [<path>...]` → rewrite ∀ generated block in `README.md`; `--check` writes nothing, exit 1 naming each stale block.
- cmd: `xenolith-dev notices [--check] [<path>...]` → regenerate `docs/THIRD-PARTY-NOTICES.md` from `cargo metadata --locked --offline --all-features`, the vendored grammars' `UPSTREAM` & license files, `nix eval --json .#default.toolLicenses`; `--check` exit 1 on drift.
- file: `README.md` blocks between `<!-- BEGIN <name> -->` / `<!-- END <name> -->`, name ∈ {`badges`, `langs`}; a block w/o markers = ⊥ opted in → exit 1 naming the markers, ⊥ a rewrite.

## §R RESEARCH

id|topic|finding|src
R340|fleet 2026-09-27|3 published siblings (sherd, microlith, itok). COMMON: README = `# name` → `<!-- BEGIN badges -->` block (license, edition, MSRV, deps, unsafe · gate hk, coverage floor · nix flake, nixpkgs, platform logos · built with Claude Code / Opus / SDD; colours 000000 rust, 6E4AFF gate, 5277C3 nix, D97757 built-with) → "Read LLM-DISCLAIMER first." (linking `docs/LLM-DISCLAIMER.md`) → the problem, measured → Install · Use/Commands · Exit codes · Use it as a library · Guarantees · Status · The name · Changelog · Contributing · Security · License. `docs/` = CODE_OF_CONDUCT, CONTRIBUTING, LLM-DISCLAIMER, SECURITY, THIRD-PARTY-NOTICES (+ INTEGRATION in 2 of 3); root = AGENTS.md, CHANGELOG.md (Keep a Changelog + version ladder), LICENSE, release.toml (cargo-release, `pre-release-hook` = `hk check --all --check --no-fail-fast`, tag `v{{version}}`, bump via PR). SPECIFIC: only sherd GENERATES its badges (`sherd-dev`, `dev/`), microlith & itok check hand-kept ones; CI/crates.io/docs.rs badges only once published; itok adds a tier badge, sherd federated nodes, lint debt & a graph; THIRD-PARTY-NOTICES hand-written in all 3 w/ "reproduce these numbers" commands|read-only reading of the 3 public repos
R341|sherd-dev 2026-09-27|`dev/src/{badge,select,commands,main}.rs`: facts from owners (Cargo.toml, hk.pkl, .coverage, .lint-debt, flake.lock, ci.yml), node header match in flake.lock (sherd's dev B1: a prefix match read an inputs entry as the node), gate steps scoped to `local fast`/`local all`, percentage truncated to a tenth, splice idempotent, input map narrows at commit & full run at push. hk: `readme-generated` (fast, `{{files}}`) + `readme-generated-full` (all, ⊥ scope), both `depends = test`, `check = cargo run -q -p sherd-dev -- --check`, `fix = … readme`|sherd `dev/`, `hk.pkl`, `flake.nix` (`cargoBuildFlags = -p sherd`)

## §V INVARIANTS

V340: EVERY generated number & name comes from the file that OWNS it — `Cargo.toml` (license, edition, MSRV, members, deps, default languages, unsafe lints), `hk.pkl` (gate steps), `.coverage`, `.lint-debt`, `flake.lock` (nixpkgs node), `.github/workflows/ci.yml` (platforms), the `SPEC.md` tree (nodes), `LangId::ALL` (known languages). a value w/ no owner = ERROR naming the owner, ⊥ a default.
V341: a CLAIM ! be checkable NOW: ⊥ crates.io / docs.rs / CI-status badge while unpublished (each = a broken image or a tick for a run nobody made); they land w/ the publish (`.:T32`).
V342: platform badges from `ci.yml`'s `os:` matrix, ⊥ `flake.nix` `systems` (`nix:C7`: declared 4, gated 3).
V343: RENDER idempotent: splice(splice(x)) == splice(x) ∴ `--check` = a diff, ⊥ a heuristic; `--check` writes nothing.
V344: a percentage TRUNCATED to 1 decimal (coverage is platform-dependent; truncation understates, the safe side of a floor). a count is SCOPED, ⊥ name-filtered: gate steps counted inside `local fast` & `local all` only (`check` is both a hook & a step).
V345: a change SELECTS the blocks it can have invalidated: ∀ block declares its INPUTS, hk hands the changed files. heuristic in one direction ∴ 2 layers — scoped @ pre-commit, UNSCOPED @ pre-push & CI where ∀ block is compared.
V346: honest where the tree is mixed: `unsafe` badge = `forbidden` only when ∀ shipped crate forbids it; a vendored-grammar crate w/ `unsafe_code = "deny"` + FFI (`languages:V121`) is COUNTED in the badge, ⊥ hidden. direct dependencies = distinct non-path `[dependencies]` keys over the SHIPPED manifests (members ∖ `publish = false`), ⊥ the root's keys alone (7 of its 9 are workspace crates).
V347: `docs/THIRD-PARTY-NOTICES.md` = generated output only (`docs:V108`): ∀ third-party crate in the normal closure of the shipped crates w/ version & license, build-only crates listed apart, ∀ vendored grammar w/ repo, rev, license & NOTICE verbatim, ∀ tool the nix package wraps w/ its nixpkgs license. ⊥ absolute path in the output ∴ same bytes on ∀ checkout.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M3 | publication | T340-T342 | README badges & langs block and the notices are generated & gated (`.:T32`) |

id|status|task|cites
T340|x|`xenolith-dev`: workspace member, lib + shim, `readme` badges & langs blocks from owners, `--check`, input selection; unit + e2e tests on a fixture repository|V340,V341,V342,V343,V344,V345,V346
T341|x|`xenolith-dev notices`: `cargo metadata` + vendored `UPSTREAM` + `nix eval .#default.toolLicenses` → `docs/THIRD-PARTY-NOTICES.md`, `--check` drift|V347,`docs:V108`,`docs:T106`
T342|.|hk: scoped `dev-generated` step in `fast`, unscoped `dev-generated-full` in `all`, both `depends = test`, `fix` regenerates|V345,`scripts:V122`

## §B BUGS

id|date|cause|fix
