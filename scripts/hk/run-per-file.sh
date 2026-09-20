#!/usr/bin/env bash
#
# Run one gate tool once PER FILE, when the tool takes exactly one path.
#
#   scripts/hk/run-per-file.sh mth fmt --check -- SPEC.md nix/SPEC.md
#
# `mth` is such a tool: handed two paths it prints "one path per run" and
# exits 2. hk hands a step every changed file at once, so without this the
# spec steps would refuse on any commit touching two SPEC.md files -- and
# refuse with a usage error, which reads like a broken gate rather than a
# finding.
#
# The `--` is REQUIRED and not decoration: it is the only thing separating
# the tool's own flags from the file list, and guessing that boundary is how
# a file named like a flag becomes a silent no-op.
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: run-per-file.sh TOOL [ARG...] -- [FILE...]" >&2
  exit 2
fi

tool="$1"
shift

args=()
saw_separator=false
while [ "$#" -gt 0 ]; do
  if [ "$1" = "--" ]; then
    saw_separator=true
    shift
    break
  fi
  args+=("$1")
  shift
done

if [ "$saw_separator" = false ]; then
  echo "usage: run-per-file.sh TOOL [ARG...] -- [FILE...] (the -- separator is required)" >&2
  exit 2
fi

if ! command -v "$tool" >/dev/null 2>&1; then
  echo "hk: ${tool} is not on PATH -- re-enter the dev shell (\`direnv reload\`, or \`nix develop\`). This is a MISSING TOOL, not a finding: nothing was checked." >&2
  exit 1
fi

# EVERY file is visited, even after one fails. A loop that stops at the
# first finding reports one problem per run, so the fix for it uncovers the
# next one and the gate becomes a queue -- the same reason CI runs with
# --no-fail-fast.
status=0
for file in "$@"; do
  "$tool" "${args[@]}" "$file" || status=1
done

exit "$status"
