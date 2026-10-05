# xenolith (xnl)

<!-- BEGIN badges -->
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![edition 2024](https://img.shields.io/badge/edition-2024-000000?logo=rust&logoColor=white)](Cargo.toml)
[![MSRV 1.95](https://img.shields.io/badge/MSRV-1.95-000000?logo=rust&logoColor=white)](Cargo.toml)
[![direct dependencies 6](https://img.shields.io/badge/direct_dependencies-6-brightgreen)](docs/THIRD-PARTY-NOTICES.md)
[![unsafe forbidden, deny in 3 FFI crates](https://img.shields.io/badge/unsafe-forbidden,_deny_in_3_FFI_crates-yellowgreen)](Cargo.toml)
[![languages 6 built, 12 planned](https://img.shields.io/badge/languages-6_built,_12_planned-6E4AFF)](languages/SPEC.md)

[![gate hk](https://img.shields.io/badge/gate-hk-6E4AFF)](hk.pkl)
[![gate steps 46](https://img.shields.io/badge/gate_steps-46-6E4AFF)](hk.pkl)
[![coverage floor 97.9%](https://img.shields.io/badge/coverage_floor-%E2%89%A597.9%25-brightgreen)](.coverage)
[![lint debt 0.0/KLoC](https://img.shields.io/badge/lint_debt-%E2%89%A40.0%2FKLoC-orange)](.lint-debt)
[![federated nodes 46](https://img.shields.io/badge/federated_nodes-46-6E4AFF)](SPEC.md)

[![nix flake](https://img.shields.io/badge/nix-flake-5277C3?logo=nixos&logoColor=white)](flake.nix)
[![nixpkgs 26.05 (2026-10-04 - 0d9e9b8)](https://img.shields.io/badge/nixpkgs-26.05_(2026--10--04_--_0d9e9b8)-5277C3?logo=nixos&logoColor=white)](flake.lock)
[![amd linux](https://img.shields.io/badge/linux-5277C3?logo=amd&logoColor=white)](.github/workflows/ci.yml)
[![arm linux](https://img.shields.io/badge/linux-5277C3?logo=arm&logoColor=white)](.github/workflows/ci.yml)
[![arm macos](https://img.shields.io/badge/macos-5277C3?logo=arm&logoColor=white)](.github/workflows/ci.yml)
[![intel linux](https://img.shields.io/badge/linux-5277C3?logo=intel&logoColor=white)](.github/workflows/ci.yml)

[![built with Claude Code](https://img.shields.io/badge/built_with-Claude_Code-D97757)](https://claude.com/claude-code)
[![built with Opus 5.5](https://img.shields.io/badge/built_with-Opus_5.5-D97757)](https://www.anthropic.com/claude)
[![built with SDD](https://img.shields.io/badge/built_with-spec--driven_development-D97757)](SPEC.md)
<!-- END badges -->

<!-- hallucinogen:autonomy-disclaimer start -->
> Read [LLM-DISCLAIMER](docs/LLM-DISCLAIMER.md) first. This repository is
> tended by an autonomous loop, and that file says what the loop may do here,
> what it may not, and what to check before trusting anything in this tree.
<!-- hallucinogen:autonomy-disclaimer end -->

Read [LLM-DISCLAIMER](docs/LLM-DISCLAIMER.md) first.

**One language per file.** xenolith finds code of one language embedded in
a file of another — a shell script inside a Nix attribute, a justfile
recipe, an hk step, a launchd plist — moves it into a file of its own, and
rewrites the host to run that file. Then it checks that every such load
resolves and that every extracted file passes its own language's linters.
CPU only — no model, no network, the same bytes out for the same tree.

The problem in one number: a read-only survey of the local repositories
this tool was built for found **188 justfiles across 34 of them**. Every
recipe body in them is a script, and the check they had was a hook matching
each recipe line against an allowlist of five command shapes — a pattern
match, not a linter. The Tcl files fared worse: **23 files in 5 repositories**,
5 of which run `sh -c` or `bash -c`, checked by a hook that is itself a Tcl
program fed to `tclsh` from a shell heredoc — an embed of exactly the kind
this tool exists to find. (Counts only, from `languages/ci/just:R178` and
`languages/shells/tcl:R193`; the repositories are not named.)

xenolith replaces those hand-rolled guards with one rule and one binary.

## Why

An embedded script is code nobody lints. shellcheck does not look inside a
Nix string; `just --fmt` does not look inside a recipe's shell; an hk step
is a string in a Pkl file. Each one grows a line at a time until it is a
real program with no tests, no formatter and no linter — and every tool
that could have caught its bugs was pointed at the host instead.

xenolith treats that as a single rule — **a file holds one language** — and
gives you the tool to enforce it: find the embeds, move them out
mechanically, and prove the result still wires together. A one-line body
such as `script = "echo hi";` is not flagged: a single simple command is
fine inline. What gets flagged is a program — a pipeline, a sequence, a
loop, a heredoc.

## Install

From crates.io, `cargo install xenolith` installs `xnl`; bring the
linters its languages call onto your `PATH` yourself (`cargo install
xenolith-lang-tcl` adds `xenolith-tcl-syntax`). The Nix flake ships `xnl`
with those linters wrapped in.

### From a flake

`packages.<system>.default` is `xnl`, for `aarch64-darwin`, `x86_64-linux`
and `aarch64-linux`, wrapped with the linters of its compiled-in languages
on `PATH`. Take it as an input and make it follow your `nixpkgs-lock`, so
both flakes share one nixpkgs revision and the binary comes from the cache
instead of being rebuilt:

```nix
{
  inputs = {
    nixpkgs-lock.url = "github:pr0d1r2/nixpkgs-lock";
    nixpkgs.follows = "nixpkgs-lock/nixpkgs";

    xenolith = {
      url = "github:pr0d1r2/xenolith";
      inputs.nixpkgs-lock.follows = "nixpkgs-lock";
      # If you also take nix-hk as an input, share it too:
      # inputs.nix-hk.follows = "nix-hk";
    };
  };

  outputs =
    { nixpkgs, xenolith, ... }:
    let
      system = "aarch64-darwin";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      devShells.${system}.default = pkgs.mkShell {
        packages = [ xenolith.packages.${system}.default ];
      };
    };
}
```

xenolith's other inputs (`nix-hk` and its spec tools) are for its own dev
shell; they appear in your lock file but not in the package.

To build only the languages a repository has — a smaller binary and fewer
linters on its `PATH` — override the package. The binary cache holds the
default build only, so a subset builds locally:

```nix
(xenolith.packages.${system}.default.override { languages = [ "nix" "shell" ]; })
```

An empty list or an unknown name is an evaluation error that lists the
supported names.

### The binary cache and `trusted-users`

The flake declares the `pr0d1r2.cachix.org` substituter and its public key
in `nixConfig`. Nix applies a flake's substituters only for a **trusted
user**, and only once the flake's config is accepted; for anyone else it
warns and ignores them, and every build the cache could have served runs
from source. Either add yourself to `trusted-users` in `nix.conf` (on
multi-user installs that file is owned by root), or add the cache to
`extra-substituters` and `extra-trusted-public-keys` there directly:

```text
extra-substituters = https://pr0d1r2.cachix.org
extra-trusted-public-keys = pr0d1r2.cachix.org-1:NfWjbhgAj41byXhCKiaE+av3Vnphm1fTezHXEGsiQIM=
```

### With cargo

```sh
cargo build --release                                  # every language
cargo build --release --no-default-features --features lang-nix,lang-shell
```

Outside nix, bring the linters yourself: `xnl lint` looks each one up on
`PATH`, and a missing one is an error (exit 2), never a silent skip.

## Use

A NixOS module with a script inside it:

```nix
{
  systemd.services.backup.script = ''
    cd /srv/data
    tar czf /var/backups/data.tgz . | tee /var/log/backup.log
  '';
}
```

`xnl check` finds it:

```console
$ xnl check
hosts/backup.nix:2:36 xenolith: shell in nix systemd.services.backup.script (non-trivial shell: pipeline, sequence; a script belongs in its own file)
```

`xnl extract hosts/backup.nix` prints a unified diff, headed
`removing xenolith → hosts/backup/backup-script.sh`, and changes nothing.
`xnl extract --write hosts/backup.nix` applies it. The script moves to its
own file:

```sh
#!/usr/bin/env bash
set -e
cd /srv/data
tar czf /var/backups/data.tgz . | tee /var/log/backup.log
```

and the host loads it:

```nix
{
  systemd.services.backup.script = builtins.readFile ./backup/backup-script.sh;
}
```

After that, `xnl check` is silent about the file and `xnl graph` confirms
the load resolves. The warning is honest bookkeeping: the new `.sh` file is
itself a shell host, and shell cannot yet report what it loads, so no
orphan was judged there:

```console
$ xnl graph --verbose
warning: loads-unsupported: shell cannot report its loads in this build (1 file(s)), so no orphan-extract was judged: an extract only it loads would be reported wrongly (src/graph:V7)
1 edges, 0 violations
```

To run it on every commit — from hk, lefthook, or a flake check — see
[docs/INTEGRATION.md](docs/INTEGRATION.md#using-xnl-in-your-own-gate).

## Commands

| command | what it does |
|---|---|
| `xnl check [--format human\|json] [paths…]` | report every non-trivial embed |
| `xnl extract [--write] <path>[:line]…` | print the diff that moves each embed into its own file; `--write` applies it |
| `xnl extract --relocate [--write] <host>[:line]…` | move extracts the config now places elsewhere, and rewrite their loads |
| `xnl inline [--write] <extract>…` | put an extract that has become trivial back into its host (refused for a shared extract) |
| `xnl graph [--format human\|json] [paths…]` | list host → extract loads; flag dangling loads and orphaned extracts |
| `xnl lint [--fix] [--trust-config] [--sites] [--format human\|json] [paths…]` | run each language's linters over extracts and host files; `--sites` lints embeds in place and reports at the host's line |
| `xnl langs [--format human\|json]` | list every language xenolith knows, and whether this build has it |
| `xnl migrate [--write]` | turn legacy `.<lang>-embedded-shell-allowlist` files into `xenolith.toml` ([migration guide](docs/MIGRATION.md)) |
| `xnl --version` | print the version |

Every verb also takes `--verbose` and `--strict-hosts` (a file no host
claims is an error instead of being skipped). With no paths, a verb scans
every file `git ls-files` lists. Output is silent on success.

Rule ids in output: `xenolith` (an embed), `dangling-load`,
`orphan-extract`, `stale-allow`, `stale-exclude`, `host-parse-error`.

Planned, not in this build: `--format sarif` (`src/cli:T103`, refused with
exit 2 today) and `xnl init` (`src/cli:T96`).

## Exit codes

A contract, because scripts and hooks read them:

| code | meaning |
|---|---|
| `0` | clean — nothing to report |
| `1` | a violation, or — for `extract` and `migrate` without `--write` — a non-empty diff |
| `2` | usage error, bad config, a missing tool or language, or a refusal |

When several apply, the highest wins. A refusal is not a crash: `xnl
extract` exiting `2` with a reason is the tool declining an edit it cannot
make safely.

## Configuration

Configuration is optional: with no file, the defaults apply. A file only
has to hold what differs, and must start with `version = 1`. A
`xenolith.toml` can sit in any directory; a file's effective config merges
every one from the repository root down to its own directory, nearest last.
An unknown key is an error (exit 2), not a silent no-op.

```toml
version = 1

# Keep one embed inline, on purpose. Keyed by the body's hash, not its line
# number: an edit elsewhere in the file keeps the entry valid, an edit to
# the body invalidates it, and an entry that matches nothing is itself a
# `stale-allow` violation. `xnl check --format json` prints the exact
# entry for each site.
[[allow]]
path = "hosts/backup.nix"
sink = "systemd.services.backup.script"
hash = "f257f88ab5045d1a"
reason = "retired with the old host next quarter"

# Files no verb reads. A glob that matches nothing is `stale-exclude`.
[[exclude]]
glob = "vendor/**"
reason = "third-party code, checked upstream"

# Shell constructs tolerated inline, everywhere.
[threshold.shell]
allow = ["pipeline"]

# Extra checks for extracted shell. Commands from a config file run only
# with `xnl lint --trust-config`.
[lint.shell]
checks = ["shellcheck --severity=error {file}"]
```

| key | default | meaning |
|---|---|---|
| `version` | required | config schema version; `1` is the only one |
| `[[allow]]` `path`, `sink`, `hash`, `reason` | — | keep one embed inline; `reason` is required |
| `[[exclude]]` `glob`, `reason` | — | files every verb skips; `reason` is required |
| `[check]`, `[extract]`, `[lint]`, `[graph]` `exclude` | `[]` | the same, for one verb only |
| `[extract] layout` | `"host"` | where extracts go: `host` (the host's own placement), `mirror`, `sibling` or `central` |
| `[extract] root` | `"scripts"` | root directory for the `mirror` and `central` layouts |
| `[threshold.shell] allow` | `[]` | shell constructs kept inline: `and-or`, `case`, `command-substitution`, `for`, `function-definition`, `heredoc`, `if`, `pipeline`, `redirect`, `sequence`, `subshell`, `while` |
| `[threshold.<guest>] max_lines`, `max_bytes` | `1`, `80` | inline size limit for any other guest |
| `[langs] unclaimed` | `"ignore"` | a file no host claims: `ignore`, `warn` or `error` |
| `[langs] missing_guest` | `"error"` | an embed whose language is not compiled in: `ignore`, `warn` or `error` |
| `[parse] host_errors` | `"error"` | a host file that does not parse: `ignore`, `warn` or `error` |
| `[lint] hosts` | `true` | also lint host files, not only extracts |
| `[lint] timeout` | `60` | seconds per check; `0` for no limit |
| `[lint] all` | `[]` | checks run on every extract, whatever its language |
| `[lint.<guest>] checks`, `fixers`, `extend` | built-ins, `true` | commands per language; `{file}` is replaced by the path. `extend = false` replaces the built-ins instead of adding to them |

The complete schema, including `[[extract.rule]]` placement rules and the
remaining thresholds, is `§I` of [`src/config/SPEC.md`](src/config/SPEC.md)
and [`src/extract/SPEC.md`](src/extract/SPEC.md). A generated reference
page is planned (`src/config:T89`).

## Languages

Six languages are compiled in by default; each is a cargo feature and a
crate of its own under [`languages/`](languages/SPEC.md).

| host | files it claims | where it finds embeds | guest | `xnl extract` |
|---|---|---|---|---|
| nix | `*.nix` | `script`, `preStart` & other systemd hooks, `shellHook`, `*Phase` & `pre`/`post` hooks, `runCommand` and `writeShellScript*` / `writeShellApplication` bodies, shell init options, any string starting with a shell shebang | shell | yes: `builtins.readFile ./<host-stem>/<name>.sh` |
| pkl | `*.pkl`, `PklProject` | hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` | shell | yes: `sh ./scripts/hk/<step>.sh {{files}}`, or the step's own shell |
| just | `justfile` (any case), `.justfile`, `*.just` | recipe bodies; a shebang recipe goes to the shebang's language | shell | yes: `sh ./scripts/just/<recipe>.sh`, or the justfile's `set shell` |
| shell | `*.sh`, `*.bash`, `.envrc`, files with a shell shebang (not `*.bats`) | heredocs and `-c` / `-e` arguments to interpreters | tcl (`tclsh`, `wish`, `expect`) | report only |
| tcl | `*.tcl`, `*.tk`, `*.exp`, files with a `tclsh` / `wish` / `expect` shebang | `exec` / `spawn` of `sh -c` and friends, `exec <interpreter> << …` | shell | report only |
| xml | `*.xml`, text `*.plist` | launchd `ProgramArguments` running `sh -c` and friends | shell | report only |

"Report only" means `xnl check` flags the embed and tells you where it is,
but `xnl extract` refuses it with exit 2 and a reason until that host's
load line is decided (`src/extract:T177`). An embed whose language is
recognised but not compiled in — a `python3 <<` heredoc in a shell script,
say — stops the run with exit 2 naming the missing language, unless
`[langs] missing_guest` says otherwise.

Every language xenolith knows, generated from `LangId::ALL`, the root
`Cargo.toml` and the spec tree by `xenolith-dev` — the same list `xnl
langs` prints, plus where each one's rules live:

<!-- BEGIN langs -->
| language | in `xnl` | cargo feature | spec |
|---|---|---|---|
| awk | planned | — | [`languages/data/awk`](languages/data/awk/SPEC.md) |
| css | planned | — | [`languages/web/css`](languages/web/css/SPEC.md) |
| dockerfile | planned | — | [`languages/ci/dockerfile`](languages/ci/dockerfile/SPEC.md) |
| html | planned | — | [`languages/web/html`](languages/web/html/SPEC.md) |
| jq | planned | — | [`languages/data/jq`](languages/data/jq/SPEC.md) |
| js | planned | — | [`languages/web/js`](languages/web/js/SPEC.md) |
| just | default build | `lang-just` | [`languages/ci/just`](languages/ci/just/SPEC.md) |
| nix | default build | `lang-nix` | [`languages/ci/nix`](languages/ci/nix/SPEC.md) |
| perl | planned | — | [`languages/data/perl`](languages/data/perl/SPEC.md) |
| pkl | default build | `lang-pkl` | [`languages/ci/pkl`](languages/ci/pkl/SPEC.md) |
| python | planned | — | [`languages/data/python`](languages/data/python/SPEC.md) |
| ruby | planned | — | [`languages/ruby`](languages/ruby/SPEC.md) |
| rust | planned | — | [`languages/rust`](languages/rust/SPEC.md) |
| shell | default build | `lang-shell` | [`languages/shells/shell`](languages/shells/shell/SPEC.md) |
| sql | planned | — | [`languages/data/sql`](languages/data/sql/SPEC.md) |
| tcl | default build | `lang-tcl` | [`languages/shells/tcl`](languages/shells/tcl/SPEC.md) |
| xml | default build | `lang-xml` | [`languages/data/xml`](languages/data/xml/SPEC.md) |
| yaml | planned | — | [`languages/ci/yaml`](languages/ci/yaml/SPEC.md) |
<!-- END langs -->

### Linters `xnl lint` runs

| language | checks | fixers (`--fix`) |
|---|---|---|
| shell extract, `sh` | `shellcheck --shell=sh`, `checkbashisms`, `shfmt --diff` | `shfmt --write` |
| shell extract, `bash` | `shellcheck --shell=bash`, `shfmt --diff` | `shfmt --write` |
| shell extract, `zsh` | `zsh -n` | — |
| shell host file | `shellcheck`, `shfmt --diff` | `shfmt --write` |
| nix host file | `statix check`, `deadnix`, `nixfmt --check` | `statix fix`, `deadnix --edit`, `nixfmt` |
| just host file | `just --fmt --check` | `just --fmt` |
| xml host file | `xmllint --noout` | — |
| tcl, host or extract | `xenolith-tcl-syntax` | — |
| pkl host file | — | — |

The tools are looked up on `PATH`, and a missing one is an error (exit 2),
never a silent skip. `xenolith-tcl-syntax` is a binary of this workspace
(the `xenolith-lang-tcl` crate): Tcl's own parse errors, found in Rust,
with no `tclsh` needed. The nix package wraps `xnl` with the linters of its
compiled-in languages on its `PATH`, `xenolith-tcl-syntax` included; `git`
comes from your own environment.

## Use it as a library

The binary is a shim over the `xenolith` library, and the library is what
another tool should call rather than parsing `xnl`'s output:

```rust
use std::path::Path;

let config = xenolith::config::parse("version = 1\n")?;
let report = xenolith::check(Path::new("."), &config, &xenolith::check::Options::default())?;
for v in report.violations() {
    println!("{}", v.to_human());
}
# Ok::<(), Box<dyn std::error::Error>>(())
```

Each language is its own crate behind the `xenolith-lang-api` contract, so
a language can be added without touching the engines
([`languages/SPEC.md`](languages/SPEC.md)).

## Guarantees

- **Nothing changes until you ask.** `xnl check`, `xnl graph` and `xnl
  lint` only read; `xnl extract`, `xnl inline` and `xnl migrate` print a
  diff and change nothing without `--write`.
- **Writes stay inside the repository.** An extract or a rewritten host is
  never written through a symlink or outside the repository root, and a
  generated path is plain ASCII that cannot inject into host syntax
  ([docs/SECURITY.md](docs/SECURITY.md)).
- **No network, no model.** No network crate is in the dependency tree and
  `cargo deny` bans them; the same tree gives the same bytes out.
- **Config cannot run commands on its own.** Commands a `xenolith.toml`
  names run only under `xnl lint --trust-config`.
- **The tool passes its own check.** `xnl check` over this repository is a
  gate step: the project may not hold the kind of embed it exists to remove.
- **Numbers here are generated.** The badges, the Languages table and
  [the third-party notices](docs/THIRD-PARTY-NOTICES.md) are rendered from
  the files that own each fact by `xenolith-dev`, this repository's own
  unpublished tooling, and the gate fails when they drift.

## Status

**`0.1.0` — the first public release** (odd rung: functional, not yet for
production; see the ladder in [CHANGELOG.md](CHANGELOG.md)). What this
page describes is what that release builds; anything planned is marked
with the spec task that will deliver it.

`0.1.0` is milestone M3 of [`SPEC.md`](SPEC.md): the six languages above,
the public doc set, the release machinery ([`release.toml`](release.toml))
and the crates published in dependency order (`.:T32`). What comes after
is ordered by the language survey — CI languages next, then data, web and
application languages (`.:C25`). [CHANGELOG.md](CHANGELOG.md) keeps the
version ladder.

## The name

A **xenolith** is a fragment of foreign rock enclosed in a host rock (Greek
*xenos*, foreign, and *lithos*, stone). An embedded script is exactly that:
a piece of one language trapped inside another. The tool finds those
fragments and removes them, leaving a host of one material.

`xnl` is the consonant skeleton of the name, the same shape as `rg` for
ripgrep. The theme is the fleet's:
[`microlith`](https://github.com/pr0d1r2/microlith) is a small stone blade,
and a xenolith is the stone that does not belong.

## Changelog

[CHANGELOG.md](CHANGELOG.md), in [Keep a Changelog](https://keepachangelog.com)
form. Pre-`1.0` a minor bump may change behaviour; what is built and what
is planned is in Status above rather than implied by the version number.

## Contributing

- [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) — setup, the gate, test
  first, and the one hard rule
- [docs/INTEGRATION.md](docs/INTEGRATION.md) — how the gate fits together,
  and how to wire `xnl` into yours
- [docs/MIGRATION.md](docs/MIGRATION.md) — moving from embedded-shell
  allowlists and fleet hooks to `xnl`
- [docs/CODE_OF_CONDUCT.md](docs/CODE_OF_CONDUCT.md)
- [AGENTS.md](AGENTS.md) — the working guide, for agents and humans alike
- [SPEC.md](SPEC.md) — the specification, root of a tree of per-directory
  specs

## Security

`xnl lint` runs linters, and — only under `--trust-config` — commands a
`xenolith.toml` names. That flag guards `xnl` and nothing else: `hk.pkl`,
justfiles and your CI workflow are also commands chosen by whoever wrote
them, so a CI job that checks pull requests from forks must take its
workflow and its hk configuration from the **base branch**. Report a
vulnerability privately — [docs/SECURITY.md](docs/SECURITY.md) lists every
boundary and the channel.

## License

MIT — see [LICENSE](LICENSE).

Three tree-sitter grammars are vendored as C source, and the nix package
wraps third-party linters; both are acknowledged, with every crate in the
dependency closure, in
[docs/THIRD-PARTY-NOTICES.md](docs/THIRD-PARTY-NOTICES.md).
