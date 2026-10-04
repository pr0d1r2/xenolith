#!/usr/bin/env bash
#
# Run every bats test whose script exists (`scripts:V352`).
#
#   scripts/hk/bats-ready.sh [DIR]      # DIR defaults to tests/unit
#
# The pre-commit `bats` step. A RED commit adds a test before the script it
# covers (`scripts:C11`), so on commit the whole suite would refuse exactly
# the commit the method asks for (`scripts:B3`). A test file mirrors its
# script by path (`scripts:C13`): `DIR/a/b.bats` covers `a/b.sh`. Every
# test whose script exists runs; each one held back is named on stderr,
# never dropped in silence. Push and `hk check` still run the whole suite,
# so a held-back test cannot leave the branch failing.
#
# Only bash builtins pick the files, and run-tool.sh runs under this same
# bash, so the tests can stage a PATH that holds nothing but a stub bats.
set -euo pipefail
shopt -s globstar nullglob dotglob

if [ "$#" -gt 1 ]; then
  echo "usage: bats-ready.sh [DIR]" >&2
  exit 2
fi

dir="${1:-tests/unit}"
if [ ! -d "$dir" ]; then
  echo "bats-ready: ${dir} is not a directory -- nothing was tested. This is a failure, not a pass." >&2
  exit 1
fi

tests=("$dir"/**/*.bats)
if [ "${#tests[@]}" -eq 0 ]; then
  echo "bats-ready: no bats file under ${dir} -- nothing was tested. This is a failure, not a pass." >&2
  exit 1
fi

ready=()
for test in "${tests[@]}"; do
  script="${test#"$dir"/}"
  script="${script%.bats}.sh"
  if [ -f "$script" ]; then
    ready+=("$test")
  else
    echo "bats-ready: holding back ${test} -- ${script} does not exist yet (RED, scripts:C11); push runs it." >&2
  fi
done

if [ "${#ready[@]}" -eq 0 ]; then
  exit 0
fi

# run-tool.sh tells a missing bats apart from a failing test, and `exec`
# leaves bats' own exit code as the verdict.
exec "$BASH" "${BASH_SOURCE[0]%/*}/run-tool.sh" bats "${ready[@]}"
