# SPEC

## §G GOAL

packaging & release: flake inputs & follows, `packages`, `checks`, devShell, cachix, language-subset override, closure hygiene, crates.io release (`release.toml`, CHANGELOG, semver, crate metadata). `flake.nix` stays at root & imports `./nix/*.nix`.

## §N NAV

rel|path|lens
up|.|-
self|nix|flake inputs, packaging, devShell, cachix, subset override, closure
sib|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
sib|src|root crate: lib + `xnl` bin, core model, CLI, cross-language engines extract/graph/lint/config
sib|scripts|∀ shell in repo: dev shell hook, guardrail scripts; gate config `hk.pkl`, vendored hk schema `pkl/Config.pkl`, `.github/workflows/`, `.github/zizmor.yml`
sib|tests|fixture format & runner, integration fixtures, bats mirroring `scripts/` & `.github/scripts/`
sib|docs|public project docs & notices

## §C CONSTRAINTS

- C6: nix flake. inputs ! `nixpkgs-lock` (`github:pr0d1r2/nixpkgs-lock`), `nix-hk` (`github:pr0d1r2/nix-hk`), `itok` (`github:pr0d1r2/itok`), `microlith` (`github:pr0d1r2/microlith`), `sherd` (`github:pr0d1r2/sherd`). `nixpkgs.follows = "nixpkgs-lock/nixpkgs"`; ∀ other input follows root `nixpkgs-lock` & `nix-hk`; tool→tool edges (`microlith`→`itok`, `itok`→`microlith`) follow root. ⊥ other nixpkgs edge. `flake.lock` committed, pure eval, ⊥ IFD. tool inputs = dev/guardrail only, ⊥ in `packages.default` closure.
- C7: systems: declared 4 (`aarch64-darwin`, `x86_64-darwin`, `x86_64-linux`, `aarch64-linux`); tier-1 3 (⊥ `x86_64-darwin`) built + pushed cachix `pr0d1r2` from `main` only (mirror `nix-hk`).
- C8: consumers take `xenolith` as flake input w/ `inputs.nixpkgs-lock.follows = "nixpkgs-lock"` ∴ same rev, cache hit. consumer picks language subset → `lang-*` features ∴ binary carries only those grammars & sinks. subset ≠ default → built locally (cachix holds default = all only), trade: smaller & faster binary vs cache miss.
- C20: cycle risk: itok, microlith, sherd may later adopt xenolith as guard → flake input cycle. ∴ those edges ! be devShell-only & `follows` root; ⊥ lib (cargo) dep on each other ?. decide before first consumer adopts.

## §I INTERFACES

- nix: `packages.<sys>.default` = xenolith; `overlays.default` ?; `checks` run cargo test + clippy + dogfood.
- nix: `packages.<sys>.default.override { languages = [ "nix" "pkl" ]; }` → `buildNoDefaultFeatures` + `buildFeatures = lang-<l>` ∀ l; default `languages` = ∀ supported.
- nix: `packages.<sys>.default` = `xnl` wrapped `--prefix PATH` ∀ runtime tool (V250); `passthru.tools` = {argv0 → drv} & `passthru.languages` of that build; supported names = `lang-*` features of root `Cargo.toml`.

## §V INVARIANTS

V17: flake inputs ! exactly `nixpkgs-lock`, `nix-hk`, `itok`, `microlith`, `sherd` (+ follows per C6); `flake.lock` holds exactly 1 nixpkgs node, rev ≡ nixpkgs-lock rev; check fails otherwise.
V29: `packages.default` closure ∌ itok, microlith, sherd, hk (dev-only inputs, C6).
V31: nix `languages` subset exact: `xnl langs` of subset build lists exactly subset as compiled in; unknown name → eval error listing supported names, ⊥ silent drop; empty list = eval error.
V96: `packages.default` = `xnl` wrapped w/ PATH ⊇ ∀ confirmed (non-`?`) default check & fixer of compiled-in languages & hosts; `languages` override drops tools of excluded languages; dev-only inputs still excluded (V29).
V250: runtime tool = argv0 of ∀ `checks()` & `fixers()` (host & guest) of a compiled-in language crate: nix `statix` `deadnix` `nixfmt`; shell `shellcheck` `shfmt` `checkbashisms` `zsh`; just `just`; xml `xmllint`; tcl `xenolith-tcl-syntax` (own drv from the tcl crate); pkl ∅. dev tool = devShell-only: Rust toolchain & `cargo-*`, llvm, git (discovery's, ⊥ a check: xnl exits 2 w/o it, the consumer's serves), hk, bats, repo-file linters, itok, microlith, sherd. tool in both = runtime. closure (`closureInfo` ≡ `nix path-info -r`) ∌ ∀ dev tool by store name. `--prefix` ∴ pinned tool wins (`.:C3`); ∅ tools → ⊥ PATH entry.
V251: `checks`: `tools` = wrapped `xnl lint` on a generated fixture (∀ host & guest dialect) w/ PATH = stdenv + git + package → exit ≠ 2, ⊥ `status: error`, ∀ runtime tool ran; `subset-*` = override `[ "nix" ]`: `xnl langs` compiled-in ≡ {nix}, tools ≡ nix's, closure ∌ excluded languages' tools.
V109: release ONLY via cargo-release (`release.toml`, `pre-release-hook` = full gate); version bump lands through a PR; tag, publish & push run from `main` (`cargo release hook` first ∵ `tag`/`publish`/`push` skip the hook); ⊥ release scripts.
V110: CHANGELOG keeps `Unreleased` & a version LADDER — each minor = a stated guarantee, a patch sits off the ladder; ∀ user-visible change adds an `Unreleased` entry in its PR.
V111: `cargo semver-checks` in the gate ∀ published crate vs last release tag; any break ⇒ minor bump in the same PR; lockstep version across the workspace (`src` C1).
V112: ∀ published crate: `description`, `license`, `repository`, `homepage`, `documentation` (docs.rs), `readme`, `keywords`, `categories`, `rust-version`, `exclude` set; docs.rs builds root w/ all `lang-*`; checked, ⊥ by review.
V113: packaged content proven: `cargo package` ∀ crate & `cargo test` from the unpacked `.crate` passes (per-crate fixtures ship, `tests:V14`).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T1, T26, T38, T41, T99 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |
| M3 | publication | T39, T108-T112 | public doc set, release machinery, history audit green, crates published (`.:T32`) |

id|status|task|cites
T1|x|scaffold flake: inputs nixpkgs-lock + nix-hk w/ follows, devShell (rustc, cargo, clippy, rustfmt, cargo-deny, cargo-llvm-cov ?, hk, bats, shellcheck, shfmt, nixfmt, statix, deadnix), `.gitignore`, `flake.lock`|V17,C6
T26|x|nix package `packages.default`, `checks` (test, clippy, dogfood); cachix push from CI `main`|C7,C19,`.:V19`
T38|.|closure check: `nix path-info -r` of `packages.default` ∌ dev tools|V29,V250,C6
T39|.|resolve C20 cycle policy before itok/microlith/sherd adopt xenolith|C20
T41|.|nix `languages` override arg → cargo features; flake check builds subset `[ "nix" ]` & asserts `xnl langs`; README consumer snippet w/ subset|V31,V251,C8
T99|.|wrap `xnl` w/ tool PATH per compiled-in language; check: `xnl lint` on fixture repo finds ∀ tool; subset build lacks excluded tools|V96,V250,V251
T108|.|`release.toml` for the workspace (lockstep, publish order api → languages → root) + runbook section|V109
T109|.|CHANGELOG w/ ladder (M1 rung, M2+ rungs per C25) & Unreleased rule in gate|V110
T110|.|semver gate step ∀ workspace crate, skip loudly w/o baseline tag (sibling pattern)|V111
T111|.|crate metadata ∀ crate + check script + docs.rs `[package.metadata.docs.rs]`|V112
T112|.|package-suite gate step: unpack each `.crate`, run its tests|V113

## §B BUGS

id|date|cause|fix
B1|2026-09-26|`checks.test` sandbox had no `git`: discovery (`src/discover:V57`) & its tests run git; T26 was verified on a tree before discovery landed ∴ `nix flake check` failed 25 lib tests after both merged|`pkgs.git` in `nativeCheckInputs` of `checks.test`; runtime closure unchanged (V29)
B2|2026-09-26|`checks.clippy` `buildPhase` = 3-line `''…''` script (`runHook` × 2 + cargo): own `xnl check` flags it `sequence` — repo held the embed it exists to forbid (`.:V19`)|each phase = one command string; package sets ⊥ pre/postBuild hooks ∴ nothing dropped; `nix flake check` green
