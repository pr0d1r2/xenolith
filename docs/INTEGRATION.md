# Integration

How a change gets from an edit to `main`, what checks it at each point, and
why the order is what it is -- then how to wire `xnl` into a gate of your own.

The short version: **the gate is a set of git hooks that also run on CI.**
Every check runs on your machine before the push, and CI runs the same steps
from the same definition -- not a parallel pipeline that can disagree with
your laptop.

## One definition, three callers

[`hk.pkl`](../hk.pkl) defines every check exactly once. Three things call it,
and none of them redefines anything:

```text
                        hk.pkl
                    one definition
                          |
        +-----------------+-----------------+
        |                 |                 |
    pre-commit        pre-push           ci.yml
    fast              all                all, then nix flake check
```

`all` is not a second list. In `hk.pkl` it is literally the fast set plus the
steps too slow or too whole-tree to sit on every commit:

```pkl
local all = (fast) {
  ["deny"]               { ... }
  ["hack-test"]          { ... }
  ["coverage"]           { ... }
  ["dogfood"]            { ... }
  ["dev-generated-full"] { ... }
  ...
}
```

Adding a cheap check means editing `fast`, and pre-push and CI inherit it.
There is no second copy to forget. How many steps that is today is the
`gate steps` badge in the [README](../README.md), counted from `hk.pkl` rather
than typed here.

## The path a change takes

```text
  edit
    |
    v
  git commit ---> pre-commit  (fast, staged files)  ---fails---> fix, retry
    |         +-> commit-msg  (Conventional Commits + a `Why:` line)
    | passes
    v
  git push   ---> pre-push    (all, the pushed range)  ---fails---> fix, retry
    |
    v
  pull request ---> ci.yml    (all, every file, three platforms)
    |                          + nix flake check
    | green, and reviewed
    v
  merged to main
```

**pre-commit** runs the fast set with `fix = true`, so formatters rewrite
rather than merely complain, and with `stash = "git"`, which is correctness
rather than speed: without it a partially staged file (`git add -p`) would be
judged as it looks in the worktree, a verdict about code you are not
committing.

**commit-msg** runs [`scripts/guard/commit-msg.sh`](../scripts/guard/commit-msg.sh)
on the message git hands it: a Conventional Commits subject, and a body with
a `Why:` line citing the spec id the change serves.

**pre-push** runs everything, adding the steps too costly for every commit:
`cargo deny`, the `cargo hack --each-feature` pair over every language
subset, the coverage floor and lint-debt ratchets, the whole-tree guards
(mirrors, node configs, crate dependency shape, private names, TDD order),
the dogfood run of `xnl check` over this repository, the unscoped comparison
of every generated output, and `lychee --offline`.

**CI** runs `nix develop --command hk check --all --check --no-fail-fast` on
`ubuntu-latest`, `ubuntu-24.04-arm` and `macos-latest`, then `nix flake
check`, which builds the package and its checks -- the test suite and clippy
over the package's own source set, the closure, the language subset and the
wrapped linters. On `main` it pushes what it built to the binary cache.

## What runs on which files

Two things vary by stage, not one. Which **steps** run is the `fast`/`all`
split above. Which **files** they see is separate:

| stage | steps | files examined |
|---|---|---|
| `pre-commit` | `fast` | **staged files only** (hk's default) |
| `pre-push` | `all` | **everything in the push**, from the ref range git hands the hook |
| CI | `all` | **every file in the repository** (`hk check --all`) |

A step runs only when a file in scope also matches its `glob`. The
whole-tree guards glob `**/*` or read the tree themselves, because the change
that breaks them -- a rename, a deletion, a new `SPEC.md` -- is not always a
changed file they would be handed.

## The steps that guard claims, not code

Several steps do something different from linting: they check that a
sentence written somewhere else in this repository is still **true**.

| step | the claim it enforces | where the claim lives |
|---|---|---|
| `dev-generated`, `dev-generated-full` | every badge number, the Languages table and the third-party notices match the files that own them | `README.md`, `docs/THIRD-PARTY-NOTICES.md` |
| `coverage`, `lint-debt` | the floor and the ceiling only move the right way | `.coverage`, `.lint-debt` |
| `deny` | no network crate, only allowed licences | `deny.toml`, `docs/SECURITY.md` |
| `dogfood` | the repository holds none of the embeds it exists to find | `SPEC.md` (`.:V19`) |
| `spec-tokens`, `budget` | every spec chain fits its ceiling | `.context-limits` |
| `node-config` | every spec node carries its own `xenolith.toml` | `SPEC.md` (`.:V90`) |
| `links` | every relative link resolves | every `*.md` |

`dev-generated` is scoped: `xenolith-dev` narrows to the outputs the staged
files feed, so a commit touching `.coverage` compares the badges and not the
notices. That map is a heuristic, and a heuristic that is too narrow lets a
stale output through -- so `dev-generated-full` runs the same checker again on
push with **no** scope, where every output is compared against its owners.
Cheap and approximate at the near end, complete at the far one
(`dev:V345`).

## Why the steps are chained

Cargo takes a lock on the target directory, so two cargo jobs launched in
parallel do not run in parallel -- the second blocks on *"Blocking waiting for
file lock on build directory"*, which reads as a hang. `depends` makes that
serialization explicit and leaves hk free to run everything else
concurrently:

```text
  serialized by depends -- cargo locks the target dir:

    fmt --> clippy --> test --+--> dev-generated          (fast)
                              +--> dev-generated-full     (all)
                              +--> deny                   (all)
                              +--> hack-test --> hack-no-dev-deps --> coverage --> lint-debt

    spec-fmt --> spec-check
    federation ---+--> nav
                  +--> spec-structure
                  +--> budget

  everything else has no depends and runs concurrently.
```

Ordering is cheapest-first on purpose. `fail_fast = true` locally, so the
first failure is the one you see and it arrives quickly. CI inverts this with
`--no-fail-fast`: there a round trip costs minutes, so a complete list beats
an early one.

## Where spec-driven development fits

The spec is not documentation sitting beside the gate -- it is an **input to
it**. Its own steps read the `SPEC.md` tree: `mth fmt` and `mth check` for
the format, `itok check` for token cost, and `sherd validate`, `sync
--check`, `check` and `budget` for the federation -- and `dev-generated`
counts its nodes for the badge.

```text
spec first          ⊥ code first ∴ the rule exists before the thing it governs
rule + runner       SAME commit ∵ a rule with no runner gates nothing
RED before GREEN    a test: commit precedes the code (tdd-order checks history)
dogfood             our own tree is the first one `xnl check` runs on
one definition      a step lives in hk.pkl, once, called by hooks and CI
never bypass        a check that is wrong is a SPEC change, ⊥ a --no-verify
```

Read [`SPEC.md`](../SPEC.md) for the node list. [`CONTRIBUTING.md`](CONTRIBUTING.md)
walks the loop for a first-time change.

## Getting the hooks

Entering the dev shell installs them:

```sh
nix develop          # or: direnv allow
```

The shell hook, [`scripts/dev/shell-hook.sh`](../scripts/dev/shell-hook.sh),
runs `hk install`. It refuses loudly rather than skipping when `hk` is
missing, because an ungated tree that looks gated is worse than one that says
so.

## Reproducing any verdict without hk

No verdict rests on hk's own logic. Every step is one plain command you can
paste into the dev shell:

```sh
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo deny check
cargo hack --workspace --each-feature test
cargo hack --workspace --each-feature --no-dev-deps check
sherd coverage --check                         # cargo llvm-cov vs .coverage
sherd debt --check                             # clippy density vs .lint-debt
cargo run -q -p xenolith-dev -- --check        # the generated outputs
cargo run -q --bin xnl -- check                # the dogfood
scripts/guard/rust-mirror.sh
scripts/guard/bats-mirror.sh
scripts/guard/node-config.sh
scripts/guard/crate-deps.sh
bats --recursive tests/unit
lychee --offline --no-progress .
nix flake check
```

hk decides *when* things run. It never hides *what* runs; `hk check --all
--check -S <step>` runs one step as the gate would.

## Releasing

Configured in [`release.toml`](../release.toml) and run by `cargo-release`,
never by a script (`nix:V109`). Dry-run is its default, so any command
without `--execute` verifies and changes nothing. The version **bump** goes
through a pull request like any other change -- the version edit without
cargo-release's commit, then the CHANGELOG by hand (`nix:V110`):

```sh
cargo release version minor --workspace --execute
```

The tail then runs from `main`:

```sh
cargo release hook                 # the gate. NOT optional -- see below
cargo release tag --execute
cargo release publish --workspace --execute
cargo release push --workspace --execute
```

`--workspace` is required: the root manifest is also a package, so without
it cargo-release selects `xenolith` alone and `publish` refuses, because the
crates it depends on were never selected (`nix:B4`). `tag` runs without it --
one `v<x.y.z>` tag marks the lockstep version.

`hook` runs first because `tag`, `publish` and `push` do **not** run
`pre-release-hook` -- only the full flow and `cargo release hook` do. Start at
`tag` and you publish whatever the tree happens to hold.

The workspace is released in lockstep: every crate carries the one
`[workspace.package] version`, one `v<x.y.z>` tag marks it, and cargo-release
publishes in dependency order -- `xenolith-shebang`, `xenolith-lang-api`, the
language crates, then `xenolith`. `xenolith-dev` is never released.

A new crate name is rate-limited by crates.io: a burst of 5, then about one
every 10 minutes. cargo-release refuses up front rather than waiting, so a
release that adds more new crates than that publishes in `-p` batches,
dependencies first, and pushes the tag only after the last batch. 0.1.0
went out as 5, then 4 an hour later. New versions of existing crates are
not affected.

## Using `xnl` in your own gate

`xnl check` exits `1` on a finding and `2` on a refusal, and with file
arguments it checks only those files, so it drops into any hook runner as
one command. Every snippet below was run against this tree.

### hk

A step in your `hk.pkl`:

```pkl
["xenolith"] {
  check = "xnl check {{files}}"
  fix = "xnl extract --write {{files}}"
}
```

hk runs `fix` wherever fixing is on -- `hk fix`, or a hook with `fix = true` --
so leave the line out if extraction should always be a deliberate step. A
file no host claims is skipped, so the step needs no glob; add one to keep it
from being scheduled on commits that touch no host.

### lefthook

```yaml
pre-commit:
  commands:
    xenolith:
      run: xnl check {staged_files}
```

A commit whose staged files hold a non-trivial embed is refused with the
same line `xnl check` prints; a clean one goes through.

### A flake check

`nix flake check` can run `xnl` over your source the way this repository's
own `checks.dogfood` does: one command in Nix, with the script in a file of
its own -- which is the rule `xnl` enforces.

```nix
checks.${system}.xenolith = pkgs.runCommand "xenolith-check" {
  nativeBuildInputs = [
    xenolith.packages.${system}.default
    pkgs.git
  ];
} "bash ${./scripts/nix/xenolith-check.sh} ${./.} $out";
```

```sh
#!/usr/bin/env bash
# scripts/nix/xenolith-check.sh SOURCE_DIR OUT
set -euo pipefail
cd "$1"
xnl check .
touch "$2"
```

`git` is an input because discovery asks git first, even for an explicit
directory, and walks the tree itself when the source is no repository.

### Pull requests from forks

`--trust-config` guards the commands `xnl` would run and nothing else. A CI
job that checks pull requests from forks must take its workflow and its hk or
lefthook configuration from the **base branch** -- otherwise the contributor
chooses what the job runs. [`SECURITY.md`](SECURITY.md) has the rest.

## Two rules that shape all of this

**Never bypass.** `--no-verify`, lowering a floor, raising a ceiling,
deleting a test, or adding `#[allow]` to silence clippy all ship the defect
with the alarm switched off. If the check itself is wrong, that is a spec
change -- say so in [`SPEC.md`](../SPEC.md), in its own commit.

**A generated number is never typed.** The badges, the Languages table and
the notices come from `xenolith-dev`; a number in prose is true the day it is
written and quietly wrong after. This document states no counts of its own
for the same reason.

## Known gaps

Listed rather than silently absent, because a gap you can read is not the
same failure as a gap you cannot.

| gap | where it goes |
|---|---|
| no `semver` gate step yet -- there is no release tag to diff against | `nix:T110` |
| `.ctrm` is reviewed evidence, but `ctrm check` is not yet a gate step; pinning its release in the Nix toolchain is deferred until the next gate-tool update | issue #38 |
| crate metadata for every crate, checked rather than reviewed | `nix:T111` |
| each `.crate` run from its unpacked tarball, not only compiled | `nix:T112` |
| the `Unreleased` rule of the changelog has no runner yet | `nix:T109` |
| `cargo-release` is not in the dev shell; the first release brings it | `.:T32` |
| `--format sarif` | `src/cli:T103` |

## Deeper

[`hk.pkl`](../hk.pkl) is the definition and is heavily commented -- every step
says why it exists. [`AGENTS.md`](../AGENTS.md) is the working guide,
[`CONTRIBUTING.md`](CONTRIBUTING.md) the arrival path, and
[`SPEC.md`](../SPEC.md) holds the invariants this all enforces.
