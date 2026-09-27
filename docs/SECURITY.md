# Security policy

## Reporting a vulnerability

Report privately, not in a public issue.

- Preferred: [GitHub private vulnerability
  reporting](https://github.com/pr0d1r2/xenolith/security/advisories/new) on
  this repository.
- Or email **pr0d1r2@gmail.com** with `xenolith security` in the subject.

Say what you ran, what happened, and what you expected. The file that
triggered it is worth more than a description of it — if it comes from a
private codebase, cut it down to the smallest file that still shows the
problem, or describe its shape instead.

You will get an acknowledgement within a week. For a valid report the fix
and the advisory are published together, and you are credited unless you
ask not to be.

## Supported versions

xenolith is pre-1.0 and not yet released. Once it is, only the latest
published version is supported; there are no backports to earlier minors.

## What xenolith does to your repository

It helps to know exactly what the tool reads, writes and runs, because that
is where the boundaries are.

- **Reads** the files `git ls-files` lists (or the paths you name), and the
  `xenolith.toml` files beside them. It never opens a network connection:
  no network crate is in the dependency tree, and `cargo deny` bans them.
- **Writes** only on request: `xnl extract --write` and `xnl migrate --write`.
  Without `--write` both print a diff and change nothing.
- **Runs** `git` to list files, for every verb that scans. `xnl lint` is the
  only verb that runs anything else: the linters for each language it
  checks, found on `PATH`.

## Trust boundaries

### `--trust-config`: commands from `xenolith.toml`

`xenolith.toml` can name commands: `[lint.<guest>] checks` and `fixers`, and
`[lint] all`. A config file is part of the repository being checked, so it
may come from someone you do not trust — a pull request, a vendored
directory, a nested `xenolith.toml` deep in the tree.

`xnl lint` therefore runs config-defined commands **only** when you pass
`--trust-config`. Without it, each one is skipped with an
`untrusted-command` warning naming it, and the built-in checks still run.
No key in any config file can grant the flag.

The built-in checks are run by name from `PATH`, so whoever controls your
`PATH` controls what they are. That is the same trust you already extend to
your shell.

### The flag guards `xnl`, not your CI

`--trust-config` covers the commands `xnl` would run. It does nothing about
the other files in a repository that are also commands: `hk.pkl` steps,
justfile recipes, the CI workflow itself. When CI checks a pull request from
a fork, the job must take its workflow and its `hk` configuration from the
**base branch**, not from the pull request — otherwise the contributor
chooses what the job runs, and `--trust-config` protects nothing. A job that
holds secrets or a token with write access must never run a fork's
configuration.

This repository's own CI runs on `pull_request` with a read-only token
(`permissions: contents: read`), does not push to the binary cache for pull
requests, and gets no secrets for pull requests from forks.

### Writes cannot escape the repository

Before writing an extract or a rewritten host, `xnl extract --write`
resolves every existing directory on the target path. If the resolved path
leaves the repository root, or any existing component is a symlink, it
refuses with exit 2: it never writes through a symlink and never creates a
directory through one.

### Reads do not follow symlinks

When `xnl graph` resolves the file a host loads, and when it looks for
orphaned extracts, it walks the path one component at a time without
following symlinks. A load that resolves through a symlink is reported as
`dangling-load` rather than followed.

### Generated paths are plain

The path of an extract, and the path written into the host's load line, may
contain only `A-Z a-z 0-9 . _ / -` and may not start with `-`. Anything else
is refused with exit 2. Those paths are pasted into host syntax — a Nix
path, an hk step string, a justfile recipe — so this is what keeps a crafted
name from injecting into it.

### Fixtures are synthetic

Every fixture in this repository is written for the purpose or anonymised.
None is copied from a private codebase, and no private repository is named
anywhere in the tree or its history; a guard checks tracked files against a
local denylist.

## In scope

- `xnl` writing outside the repository, or through a symlink.
- `xnl lint` running a command from `xenolith.toml` without
  `--trust-config`.
- A crafted file that makes an extraction inject text into the host's
  syntax, or change what the host runs beyond moving the embed.
- A crash or hang on hostile input. The parsers are tree-sitter grammars
  written in C, so memory-safety bugs there are possible even though the
  Rust code forbids `unsafe` everywhere except the one-function shim that
  hands each vendored grammar (just, pkl, tcl) to tree-sitter.

## Out of scope

- What the linters `xnl lint` runs do with the files they are given.
- Anything that needs control of your `PATH` or your shell already.
