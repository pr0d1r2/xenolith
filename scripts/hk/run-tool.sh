#!/usr/bin/env bash
#
# Run one gate tool, refusing loudly when it is not installed.
#
# Every hk step is one plain command a human can paste (`scripts:C9`), and
# remediation text lives in a script rather than in an inline
# `|| { echo ...; }` (`scripts:C21`). This is that script, and it draws one
# distinction the runner cannot: a tool that is MISSING did not check
# anything, while a tool that RAN and failed found something. Collapse the
# two and a machine with half the toolchain reports a clean repo.
#
#   scripts/hk/run-tool.sh shellcheck --shell=bash file.sh
set -euo pipefail

if [ "$#" -eq 0 ]; then
  echo "usage: run-tool.sh TOOL [ARG...]" >&2
  exit 2
fi

tool="$1"
shift

if ! command -v "$tool" >/dev/null 2>&1; then
  echo "hk: ${tool} is not on PATH -- re-enter the dev shell (\`direnv reload\`, or \`nix develop\`). This is a MISSING TOOL, not a finding: nothing was checked." >&2
  exit 1
fi

# `exec` so the tool owns the process: its exit code, its stdout, its stderr,
# its signals. A wrapper that reports on the tool's behalf is a second place
# for the verdict to be wrong.
exec "$tool" "$@"
