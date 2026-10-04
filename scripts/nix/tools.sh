#!/usr/bin/env bash
#
# The flake's tool checks (`nix:T99`, `nix:V96`, `nix:V251`): write a
# fixture repo with a file for every host and guest dialect, run the
# packaged `xnl lint` over it, and create the check's output only when no
# check went unspawned and every TOOL ran.
#
# The check builds with stdenv's PATH and the package, nothing else, so a
# linter can only be found through the wrapper's PATH. Unspawned is
# `xnl lint` exit 2 or a result with `status: error` (`src/lint:V8`,
# `src/lint` §I); findings (exit 1) are a tool that RAN. Files of a
# language the build left out are unclaimed and skipped (`src/check:V13`),
# so one fixture serves every subset.
#
#   scripts/nix/tools.sh OUT [TOOL...]
set -euo pipefail

if [ "$#" -lt 1 ]; then
  echo "usage: tools.sh OUT [TOOL...]" >&2
  exit 2
fi

out="$1"
shift

fixture="$(mktemp -d)"
cd "$fixture"
printf 'version = 1\n' >xenolith.toml
printf '{ }\n' >default.nix
printf 'default:\n    echo hi\n' >justfile
printf '<?xml version="1.0"?>\n<a/>\n' >a.xml
printf 'puts hi\n' >a.tcl
# A shebang makes a script an extract of its dialect (`src/lint` §I):
# checkbashisms runs only for sh, `zsh -n` only for zsh.
printf '#!/bin/sh\necho hi\n' >posix.sh
printf '#!/usr/bin/env bash\necho hi\n' >bash.sh
printf '#!/usr/bin/env zsh\necho hi\n' >z.zsh

set +e
report="$(xnl lint --format json .)"
status=$?
set -e

failed=0
if [ "$status" -ge 2 ]; then
  echo "tools: xnl lint exited ${status}: a check could not run" >&2
  failed=1
fi
if grep -q '"status": "error"' <<<"$report"; then
  echo "tools: a check errored instead of running" >&2
  failed=1
fi
for tool in "$@"; do
  if ! grep -qF "\"check\": \"${tool}\"" <<<"$report"; then
    echo "tools: ${tool} never ran" >&2
    failed=1
  fi
done

if [ "$failed" -ne 0 ]; then
  printf '%s\n' "$report" >&2
  exit 1
fi
touch "$out"
