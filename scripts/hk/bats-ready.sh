#!/usr/bin/env bash
#
# Run every bats test whose script exists and is not held back by the
# commit being made (`scripts:V352`).
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
# Bash builtins (and git, when present) pick the files, and run-tool.sh runs
# under this same bash, so the tests can stage a PATH that holds nothing but
# a stub bats.
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

# What the commit holds (`scripts:B4`): a test staged while its script is
# not is the RED commit for a change to a script that already exists, so
# it is held back like a test whose script does not exist yet. Outside a
# git work tree, or with no git, there is no commit to read and the rule is
# off; the worst that does is refuse a RED commit, never pass a GREEN one.
declare -A staged=()
if command -v git >/dev/null 2>&1 &&
  git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  if ! index="$(git diff --cached --name-only --relative)"; then
    echo "bats-ready: git could not list the staged files -- nothing was tested. This is a failure, not a pass." >&2
    exit 1
  fi
  while IFS= read -r path; do
    if [ -n "$path" ]; then
      staged["$path"]=1
    fi
  done <<<"$index"
fi

ready=()
for test in "${tests[@]}"; do
  script="${test#"$dir"/}"
  script="${script%.bats}.sh"
  if [ ! -f "$script" ]; then
    echo "bats-ready: holding back ${test} -- ${script} does not exist yet (RED, scripts:C11); push runs it." >&2
  elif [ -n "${staged[$test]:-}" ] && [ -z "${staged[$script]:-}" ]; then
    echo "bats-ready: holding back ${test} -- staged without ${script} (RED, scripts:B4); push runs it." >&2
  else
    ready+=("$test")
  fi
done

if [ "${#ready[@]}" -eq 0 ]; then
  exit 0
fi

# run-tool.sh tells a missing bats apart from a failing test, and `exec`
# leaves bats' own exit code as the verdict.
exec "$BASH" "${BASH_SOURCE[0]%/*}/run-tool.sh" bats "${ready[@]}"
