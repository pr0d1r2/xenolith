#!/usr/bin/env bash
#
# Run on every dev shell entry (`scripts:C10`), wired into the shell with
# `builtins.readFile` so this file is the only copy -- an inlined shellHook
# is shell that no shellcheck, no shfmt and no bats ever sees
# (`scripts:C13`).
#
# Its whole job is to make sure the gate is INSTALLED. `hk install` is
# idempotent and rewrites the hooks each time, which is deliberate: the hook
# a contributor has is then always the one `hk.pkl` describes today, and
# editing the gate needs no separate reinstall step to take effect.
set -euo pipefail

if ! command -v hk >/dev/null 2>&1; then
  echo "shell-hook: hk is not on PATH -- the git hooks were NOT installed, so nothing is gating this repo (scripts:V122). This means the dev shell is broken, not that hk is optional: re-enter it (\`direnv reload\`, or \`nix develop\`)." >&2
  exit 1
fi

# In a worktree or a submodule `.git` is a FILE, and a source tarball has no
# `.git` at all. Refusing here is the honest answer -- there is no hooks
# directory to install into -- but it is not a broken toolchain, so it is not
# a failure either. It is still LOUD: a shell that quietly installed nothing
# is how a repo ends up ungated for a week.
if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "shell-hook: not inside a git work tree -- no hooks directory, so the gate was not installed. Commits made from here are ungated (scripts:V122)." >&2
  exit 0
fi

# Success is silence. `hk install` prints what it wrote; on entry that line
# is noise on every single shell, and noise on every shell is how a real
# message gets scrolled past. A failure keeps its own output and its own
# exit code.
hk install >/dev/null
