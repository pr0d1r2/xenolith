# Security policy

## Reporting a vulnerability

Report privately, not in a public issue.

- Preferred: [GitHub private vulnerability
  reporting](https://github.com/pr0d1r2/xenolith/security/advisories/new) on
  this repository.
- Or email **pr0d1r2@gmail.com** with `xenolith security` in the subject.

Include what you ran, what happened, and the file that triggered it. A
reproducing file is worth more than a description of one — if it comes from a
private codebase, cut it down to the smallest file that still shows the
problem, or describe its shape instead.

Expect an acknowledgement within a week. If a report is valid, the fix and the
advisory go out together, and you are credited unless you ask otherwise.

## Supported versions

Pre-1.0, and not yet released. Once it is, only the latest published version
is supported; there are no backports.

## What the attack surface actually is

Stated plainly, because this tool does more than read files and it would be
misleading to present it as though it did not.

**`xnl` reads your tree, may rewrite it, and may run linters over it.**

- **Reads** the files `git ls-files` lists (or the paths you name), and the
  `xenolith.toml` files beside them.
- **Writes** only on request: `xnl extract --write`, `xnl inline --write`
  and `xnl migrate --write`. Without `--write` each prints a diff and changes
  nothing.
- **Runs** `git` to list files, for every verb that scans. `xnl lint` is the
  only verb that runs anything else: the linters for each language it
  checks, found on `PATH`.

That combination is what is worth attacking, and what is worth reporting:

1. **A config file reaching a command.** `xenolith.toml` can name commands:
  `[lint.<guest>] checks` and `fixers`, and `[lint] all`. A config file is
  part of the repository being checked, so it may come from someone you do
  not trust — a pull request, a vendored directory, a nested
  `xenolith.toml` deep in the tree. `xnl lint` therefore runs
  config-defined commands **only** under `--trust-config`; without it each
  is skipped with an `untrusted-command` warning naming it, and the
  built-in checks still run. No key in any config file can grant the flag.
  Anything that runs a config-named command without it is a vulnerability.
2. **A write outside its lane.** Before writing an extract or a rewritten
  host, `xnl extract --write` resolves every existing directory on the
  target path. If the resolved path leaves the repository root, or any
  existing component is a symlink, it refuses with exit 2: it never writes
  through a symlink and never creates a directory through one. A path that
  gets past that is a defect regardless of how it was reached.
3. **A read that follows a link.** When `xnl graph` resolves the file a host
  loads, and when it looks for orphaned extracts, it walks the path one
  component at a time without following symlinks. A load that resolves
  through a symlink is reported as `dangling-load` rather than followed.
4. **Text injected into a host.** The path of an extract, and the path
  written into the host's load line, may contain only `A-Z a-z 0-9 . _ / -`
  and may not start with `-`; anything else is refused with exit 2. Those
  paths are pasted into host syntax — a Nix path, an hk step string, a
  justfile recipe — so a crafted file that makes an extraction inject text
  into the host, or change what the host runs beyond moving the embed, is in
  scope.
5. **A crash or hang on hostile input.** The parsers are tree-sitter
  grammars written in C, so memory-safety bugs there are possible even
  though the Rust code forbids `unsafe` everywhere except the one-function
  shim that hands each vendored grammar (just, pkl, tcl) to tree-sitter.

**The flag guards `xnl`, not your CI.** `--trust-config` covers the commands
`xnl` would run. It does nothing about the other files in a repository that
are also commands: `hk.pkl` steps, justfile recipes, the CI workflow itself.
When CI checks a pull request from a fork, the job must take its workflow and
its `hk` configuration from the **base branch**, not from the pull request —
otherwise the contributor chooses what the job runs, and `--trust-config`
protects nothing. A job that holds secrets or a token with write access must
never run a fork's configuration. This repository's own CI runs on
`pull_request` with a read-only token (`permissions: contents: read`), does
not push to the binary cache for pull requests, and gets no secrets for pull
requests from forks.

**What is structurally excluded:**

- **No network.** No network crate is in the dependency tree, and
  `cargo deny` bans them.
- **No model.** Every verdict is a pure function of the files; the same tree
  gives the same bytes out.
- **No private fixtures.** Every fixture in this repository is written for
  the purpose or anonymised. None is copied from a private codebase, and no
  private repository is named anywhere in the tree or its history; a guard
  checks tracked files against a local denylist.

## What is out of scope

- What the linters `xnl lint` runs do with the files they are given.
- Anything that needs control of your `PATH` or your shell already: the
  built-in checks are run by name from `PATH`, which is the same trust you
  already extend to your shell.
- `xnl` modifying your repository when you asked it to with `--write`.
  Moving an embed is the job; doing it outside the lane in point 2 is not.
