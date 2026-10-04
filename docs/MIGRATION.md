# Migrating to xenolith

This guide is for a repository that already polices embedded code with
hand-rolled hooks and allowlist files, and wants `xnl` to take over. It
covers four things:

- the per-language allowlist files, which `xnl migrate` turns into one
  `xenolith.toml`;
- the justfile hook that allowed only listed one-line commands;
- the XML hook that ran `xmllint --noout`;
- the Tcl hook that asked `tclsh` whether each file was complete.

Each section says what `xnl` does instead and, where it behaves
differently, exactly how.

## Allowlist files → `xenolith.toml`

### What the old files looked like

One file per host language at the repository root, named
`.<lang>-embedded-shell-allowlist` — `.nix-embedded-shell-allowlist`,
`.pkl-embedded-shell-allowlist` and so on. Each line is a repository path;
lines starting with `#` are comments; blank lines separate entries:

```text
# backup unit kept inline until the host is retired
hosts/backup.nix
```

Every listed file was exempt as a whole: any embed in it, now or later,
passed.

### What replaces them

xenolith has no whole-file exemption. An `[[allow]]` entry keeps **one**
embed inline, and it is keyed by the host path, the sink (where in the
host the embed sits) and a hash of the embedded body:

```toml
[[allow]]
path = "hosts/backup.nix"
sink = "systemd.services.backup.script"
hash = "f257f88ab5045d1a"
reason = "migrated from .nix-embedded-shell-allowlist: backup unit kept inline until the host is retired"
```

That is deliberately narrower than the old promise. Edits elsewhere in the
file leave the entry valid; an edit to the body itself invalidates it, so a
grandfathered script cannot quietly grow. An entry that no longer matches
anything is reported as `stale-allow`, so dead entries do not pile up.

### Running the migration

```sh
xnl migrate            # print the xenolith.toml it would write; exit 1 if non-empty
xnl migrate --write    # write it
```

`xnl migrate` reads every `.<lang>-embedded-shell-allowlist` at the root,
checks each listed file with the default settings, and writes one
`[[allow]]` entry for every embed that check would flag. The `reason` of
each entry is `migrated from <list file>`, followed by the comment written
just above the path in the list, if there was one.

What it cannot carry over, it reports as a warning instead of dropping:

- `legacy-missing` — a listed file that no longer exists (or is reached
  through a symlink);
- `legacy-no-site` — a listed file holding nothing `xnl check` would flag,
  so it never needed the exemption.

If a `xenolith.toml` already exists at the root, `xnl migrate` refuses with
exit 2: it creates that file and never merges into one. Move yours aside,
run the migration, then carry your other keys across by hand.

### After the migration

- Review the entries. Every one is an embed that could be extracted
  instead — `xnl extract <path>` shows what that would look like, and
  an entry you can remove is usually the better outcome.
- Delete the old `.<lang>-embedded-shell-allowlist` files. `xnl migrate`
  leaves them in place.
- Replace the old hook in your `hk.pkl` with `xnl check {{files}}` (see
  [the README](../README.md#running-it-from-hk)).

## The justfile hook

### What it did

The old hook read every file named exactly `justfile` and required **every
line of every recipe** to match an allowlist of commands — running a script
under `scripts/`, running bats or expect tests, and a few more. Anything
else failed.

### What `xnl` does instead

just runs a recipe without a shebang **one line at a time**, each line in
a fresh shell. So `xnl` applies one rule: a recipe is fine inline when it is
one line and that line is one simple command. Two or more lines is a
script, and a script belongs in its own file. There is no command
allowlist, and no justfile-specific setting.

The difference from the old hook, measured on the same inputs, is real and
goes in both directions:

| Recipe | Old hook | `xnl check` |
| ------ | -------- | ----------- |
| one allowlisted command | passes | passes |
| one command **not** on the allowlist (`cargo build`, `rm -rf dist`) | fails | **passes** — looser |
| a shebang line plus one command | fails | **passes** — looser |
| two or more lines, even if every one is allowlisted | passes | **flagged** — stricter |
| a file named `Justfile`, `.justfile` or `*.just` | not checked | **checked** — wider |

If you relied on the allowlist to keep particular commands out of your
justfiles, that guarantee is gone; `xnl` judges shape, not content.

### Moving multi-line recipes out

```sh
xnl extract justfile            # show the diff
xnl extract --write justfile    # apply it
```

Each flagged recipe body moves to `scripts/just/<recipe>.sh` next to the
justfile, and the recipe becomes one line that runs it. Before:

```just
lint:
  bash scripts/lint.sh
  bats tests/unit
```

After:

```just
lint:
  sh ./scripts/just/lint.sh
```

The extracted script starts with `set -eu`, because just stops at the first
failing line and its default shell is `sh -cu`. A line prefixed with `-`
(ignore failure) becomes `… || true`. If the justfile sets
`set shell := ["bash", …]`, the script and its load use bash instead.

Some recipes cannot be merged into one script without changing what they
do, and `xnl extract` refuses them with a reason rather than guess: bodies
using `{{…}}` interpolation, shebang recipes, parameters under
`set positional-arguments`, and lines that change shell state a later line
depends on (`cd`, `export`, assignments, `set`) — each line used to get a
fresh shell, a single script does not. Move those by hand, or keep them
inline with an `[[allow]]` entry and a reason.

`[threshold.shell] allow = ["sequence"]` would also silence multi-line
recipes, but it relaxes **every** shell embed in the repository, not only
justfiles. Prefer extraction or a targeted `[[allow]]`.

## The XML hook

### What it did

It ran `xmllint --noout` on each staged `*.xml` file. Property lists were
not checked.

### What `xnl` does instead

- `xnl lint` runs `xmllint --noout` on every `*.xml` file and every
  **text** `*.plist` (binary plists are skipped). That is wider than the old
  hook: plists are now checked too.
- `xnl check` also looks inside launchd property lists: a
  `ProgramArguments` array that runs `sh -c`, `bash -c`, `zsh -c` or
  `dash -c` with more than a single simple command is flagged as an embed.
  `xnl extract` does not move these yet — launchd runs jobs from `/` unless
  told otherwise, so where the script should live is still undecided
  (`languages/data/xml:T190`). Move it by hand or allow it with a reason.
- A file that is not well-formed XML is reported by `xnl check` as
  `host-parse-error`.

`xmllint` comes from libxml2 and must be on your `PATH`.

## The Tcl hook

### What it did

For each `*.tcl` and `*.exp` file it fed the file to `tclsh` and asked
`info complete` — unclosed braces, brackets or quotes fail — and warned
about a `#` inside `set x { … }`, which Tcl treats as text, not a comment.
The check was itself a Tcl program embedded in a shell heredoc.

### What `xnl` does instead

- `xnl lint` runs `xenolith-tcl-syntax` on every `*.tcl`, `*.tk` and `*.exp`
  file, on files whose shebang names `tclsh`, `wish` or `expect`, and on
  Tcl extracted from shell. It reports what Tcl's own parser rejects: an
  unclosed `{`, `"`, `[`, `${` or `$a(`, a file ending in a
  backslash-newline, and extra characters after a close-brace or
  close-quote — the first error per file, as `file:line:col: message`.
  It needs no `tclsh`.
- The `#`-inside-braces warning is **not** carried over. That text is
  valid Tcl, and `xenolith-tcl-syntax` checks syntax only.
- `xnl check` flags shell embedded in Tcl (`exec sh -c {…}`, and expect's
  `spawn sh -c …`) and Tcl embedded in shell (a heredoc fed to `tclsh`,
  `wish` or `expect`, and `expect -c '…'`) — the old hook's own shape
  included. `xnl extract` does not move either yet; they are reported so
  you can move them by hand or allow them.

`xenolith-tcl-syntax` is a binary of the `xenolith-lang-tcl` crate in this
workspace. The nix package puts it on `xnl`'s `PATH` (`nix:T99`); from a
checkout, `cargo build --workspace` builds it next to `xnl`.
