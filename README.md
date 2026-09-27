# xenolith

**One language per file.** xenolith finds code of one language embedded in a
file of another — a shell script inside a Nix attribute, a justfile recipe,
an hk step, a launchd plist — moves it into a file of its own, and rewrites
the host to run that file. Then it checks that every such load resolves and
that every extracted file passes its own language's linters.

The binary is `xnl`.

> **Status: pre-release.** Nothing is published yet — no crates.io release,
> no tagged version, and the GitHub repository named below is not public
> yet. What this page describes is what the tree builds today; anything
> planned is marked with the spec task that will deliver it.

## Why

An embedded script is code nobody lints. shellcheck does not look inside a
Nix string; `just --fmt` does not look inside a recipe's shell; an hk step
is a string in a Pkl file. Each one grows a line at a time until it is a
real program with no tests, no formatter and no linter — and every tool that
could have caught its bugs was pointed at the host instead.

xenolith treats that as a single rule — **a file holds one language** — and
gives you the tool to enforce it: find the embeds, move them out
mechanically, and prove the result still wires together.

## The name

A **xenolith** is a fragment of foreign rock enclosed in a host rock (Greek
*xenos*, foreign, and *lithos*, stone). An embedded script is exactly that:
a piece of one language trapped inside another. The tool finds those
fragments and removes them, leaving a host of one material.

`xnl` is the consonant skeleton of the name, the same shape as `rg` for
ripgrep.

## Quick tour

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

After that, `xnl check` is silent and `xnl graph` confirms the load:

```console
$ xnl check
$ xnl graph --verbose
1 edges, 0 violations
```

A one-line body such as `script = "echo hi";` is not flagged: a single
simple command is fine inline. What gets flagged is a program — a pipeline,
a sequence, a loop, a heredoc.

## Commands

| Command | What it does |
| ------- | ------------ |
| `xnl check [--format human\|json] [paths…]` | Report every non-trivial embed. |
| `xnl extract [--write] <path>[:line]…` | Print the diff that moves each embed into its own file; `--write` applies it. |
| `xnl extract --relocate [--write] <host>[:line]…` | Move extracts the config now places elsewhere, and rewrite their loads. |
| `xnl inline [--write] <extract>…` | Put an extract that has become trivial back into its host (refused for a shared extract). |
| `xnl graph [--format human\|json] [paths…]` | List host → extract loads; flag dangling loads and orphaned extracts. |
| `xnl lint [--fix] [--trust-config] [--sites] [--format human\|json] [paths…]` | Run each language's linters over extracts and host files; `--sites` lints embeds in place and reports findings at the host's line. |
| `xnl langs [--format human\|json]` | List every language xenolith knows, and whether this build has it. |
| `xnl migrate [--write]` | Turn legacy `.<lang>-embedded-shell-allowlist` files into `xenolith.toml` ([migration guide](docs/MIGRATION.md)). |
| `xnl --version` | Print the version. |

Every verb also takes `--verbose` and `--strict-hosts` (a file no host
claims is an error instead of being skipped). With no paths, a verb scans
every file `git ls-files` lists. Output is silent on success.

Exit codes: **0** clean, **1** violations found (or, for `extract` and
`migrate` without `--write`, a non-empty diff), **2** usage error, bad
config, or a refusal. When several apply, the highest wins.

Rule ids in output: `xenolith` (an embed), `dangling-load`,
`orphan-extract`, `stale-allow`, `stale-exclude`, `host-parse-error`.

Planned, not in this build: `--format sarif` (`src/cli:T103`, refused with
exit 2 today), `xnl extract --relocate` (`src/extract:T101`), `xnl init`
(`src/cli:T96`) and `xnl inline` (`src/extract:T102`).

## What works today

Six languages are compiled in by default. `xnl langs` lists them, plus the
twelve more the spec plans for, which this build reports as
`compiled-out`.

| Host | Files it claims | Where it finds embeds | Guest | `xnl extract` |
| ---- | --------------- | --------------------- | ----- | ------------- |
| nix | `*.nix` | `script`, `preStart` & other systemd hooks, `shellHook`, `*Phase` & `pre`/`post` hooks, `runCommand` and `writeShellScript*` / `writeShellApplication` bodies, shell init options, any string starting with a shell shebang | shell | yes: `builtins.readFile ./<host-stem>/<name>.sh` |
| pkl | `*.pkl`, `PklProject` | hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` | shell | yes: `sh ./scripts/hk/<step>.sh {{files}}`, or the step's own shell |
| just | `justfile` (any case), `.justfile`, `*.just` | recipe bodies; a shebang recipe goes to the shebang's language | shell | yes: `sh ./scripts/just/<recipe>.sh`, or the justfile's `set shell` |
| shell | `*.sh`, `*.bash`, `.envrc`, files with a shell shebang (not `*.bats`) | heredocs and `-c` / `-e` arguments to interpreters | tcl (`tclsh`, `wish`, `expect`) | report only |
| tcl | `*.tcl`, `*.tk`, `*.exp`, files with a `tclsh` / `wish` / `expect` shebang | `exec` / `spawn` of `sh -c` and friends, `exec <interpreter> << …` | shell | report only |
| xml | `*.xml`, text `*.plist` | launchd `ProgramArguments` running `sh -c` and friends | shell | report only |

"Report only" means `xnl check` flags the embed and tells you where it is,
but `xnl extract` refuses it with exit 2 and a reason until that host's load
line is decided (`src/extract:T177`).

An embed whose language is recognised but not compiled in — a `python3 <<`
heredoc in a shell script, say — stops the run with exit 2 naming the
missing language, unless `[langs] missing_guest` says otherwise.

### Linters `xnl lint` runs

| Language | Checks | Fixers (`--fix`) |
| -------- | ------ | ---------------- |
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
never a silent skip. `xenolith-tcl-syntax` is a binary of this workspace (the
`xenolith-lang-tcl` crate): Tcl's own parse errors, found in Rust, with no
`tclsh` needed. The nix package wraps `xnl` with the linters of its compiled-in
languages on its `PATH` (`nix:T99`), `xenolith-tcl-syntax` included; `git`
comes from your own environment. Outside nix, bring the tools yourself.

## `xenolith.toml`

Configuration is optional: with no file, the defaults apply. A file only has
to hold what differs, and must start with `version = 1`. A `xenolith.toml`
can sit in any directory; a file's effective config merges every one from
the repository root down to its own directory, nearest last. An unknown key
is an error (exit 2), not a silent no-op.

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

| Key | Default | Meaning |
| --- | ------- | ------- |
| `version` | required | Config schema version; `1` is the only one. |
| `[[allow]]` `path`, `sink`, `hash`, `reason` | — | Keep one embed inline. `reason` is required. |
| `[[exclude]]` `glob`, `reason` | — | Files every verb skips. `reason` is required. |
| `[check]`, `[extract]`, `[lint]`, `[graph]` `exclude` | `[]` | The same, for one verb only. |
| `[extract] layout` | `"host"` | Where extracts go: `host` (the host's own placement), `mirror`, `sibling` or `central`. |
| `[extract] root` | `"scripts"` | Root directory for the `mirror` and `central` layouts. |
| `[threshold.shell] allow` | `[]` | Shell constructs kept inline: `and-or`, `case`, `command-substitution`, `for`, `function-definition`, `heredoc`, `if`, `pipeline`, `redirect`, `sequence`, `subshell`, `while`. |
| `[threshold.<guest>] max_lines`, `max_bytes` | `1`, `80` | Inline size limit for any other guest. |
| `[langs] unclaimed` | `"ignore"` | A file no host claims: `ignore`, `warn` or `error`. |
| `[langs] missing_guest` | `"error"` | An embed whose language is not compiled in: `ignore`, `warn` or `error`. |
| `[parse] host_errors` | `"error"` | A host file that does not parse: `ignore`, `warn` or `error`. |
| `[lint] hosts` | `true` | Also lint host files, not only extracts. |
| `[lint] timeout` | `60` | Seconds per check; `0` for no limit. |
| `[lint] all` | `[]` | Checks run on every extract, whatever its language. |
| `[lint.<guest>] checks`, `fixers`, `extend` | built-ins, `true` | Commands per language; `{file}` is replaced by the path. `extend = false` replaces the built-ins instead of adding to them. |

The complete schema, including `[[extract.rule]]` placement rules and the
remaining thresholds, is `§I` of [`src/config/SPEC.md`](src/config/SPEC.md)
and [`src/extract/SPEC.md`](src/extract/SPEC.md). A generated reference page
is planned (`src/config:T89`).

## Using it from a flake

xenolith is a Nix flake. Its `packages.<system>.default` is `xnl`, for
`aarch64-darwin`, `x86_64-linux` and `aarch64-linux`. Take it as an input and
make it follow your `nixpkgs-lock`, so both flakes share one nixpkgs
revision and the binary comes from the cache instead of being rebuilt:

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

The package carries the linters of its languages (`nix:T99`). To build only
the languages a repo has -- a smaller binary and fewer linters on its `PATH`
(`nix:T41`) -- override it; cachix holds the default build only, so a subset
builds locally:

```nix
(xenolith.packages.${system}.default.override { languages = [ "nix" "shell" ]; })
```

An empty list or an unknown name is an evaluation error that lists the
supported names. Outside nix, `cargo build --no-default-features --features
lang-nix,lang-shell` builds the same subset.

### The binary cache and `trusted-users`

The flake declares the `pr0d1r2.cachix.org` substituter and its public key
in `nixConfig`. Nix applies a flake's substituters only for a **trusted
user**, and only once the flake's config is accepted; for anyone else it
warns and ignores them, and every build the cache could have served runs
from source. Either add yourself to `trusted-users`
in `nix.conf` (on multi-user installs that file is owned by root), or add
the cache to `extra-substituters` and `extra-trusted-public-keys` there
directly:

```text
extra-substituters = https://pr0d1r2.cachix.org
extra-trusted-public-keys = pr0d1r2.cachix.org-1:NfWjbhgAj41byXhCKiaE+av3Vnphm1fTezHXEGsiQIM=
```

### Running it from hk

```pkl
["xenolith"] {
  check = "xnl check {{files}}"
  fix = "xnl extract --write {{files}}"
}
```

`xnl check` exits 1 on a finding, which fails the step. hk runs `fix`
wherever fixing is on — `hk fix`, or a hook with `fix = true` — so leave
the line out if extraction should always be a deliberate step.

## Trust boundary

`xnl lint` runs commands a `xenolith.toml` names only when you pass
`--trust-config`; no key in a config file can grant it. That flag guards
`xnl` and nothing else. `hk.pkl`, justfiles and your CI workflow are also
commands chosen by whoever wrote them, so a CI job that checks pull requests
from forks must take its workflow and its hk configuration from the **base
branch**, not from the pull request. [`docs/SECURITY.md`](docs/SECURITY.md)
lists every boundary and how to report a vulnerability.

## More

- [`docs/MIGRATION.md`](docs/MIGRATION.md) — moving from embedded-shell
  allowlists and fleet hooks to `xnl`.
- [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) — the gate, test-first, and
  commit rules. [`AGENTS.md`](AGENTS.md) is the same as a checklist.
- [`docs/LLM-DISCLAIMER.md`](docs/LLM-DISCLAIMER.md) — how this was built.
- [`docs/CODE_OF_CONDUCT.md`](docs/CODE_OF_CONDUCT.md).
- [`SPEC.md`](SPEC.md) — the specification, root of a tree of per-directory
  specs.

## Licence

[MIT](LICENSE).
