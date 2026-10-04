#!/usr/bin/env bash
#
# Every shell script has a test, and every test has a script
# (`scripts/guard:V21`, `scripts:C13`).
#
#   scripts/a/b.sh            <-> tests/unit/scripts/a/b.bats
#   .github/scripts/ci/x.sh   <-> tests/unit/.github/scripts/ci/x.bats
#
# BOTH directions, because they fail differently. A script with no test is
# visible the moment someone looks; an ORPHAN test passes forever -- the
# script it covered was renamed, the suite still says green, and the number
# of tests went up.
#
# Tracked files only. An untracked scratch script is not part of the repo,
# and a guard that fails on one fails during every experiment -- which is
# how a guard gets disabled.
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "bats-mirror: not inside a git work tree -- the file list comes from git, so nothing could be checked. This is a failure, not a pass." >&2
  exit 1
fi

status=0

fail() {
  echo "bats-mirror: $1" >&2
  status=1
}

# Direction one: every allowed script has its mirror. Anything ending .sh
# outside the two allowed trees is a separate violation of the same
# constraint -- C13 says where shell may live, not merely that it must be
# tested.
while IFS= read -r script; do
  [ -n "$script" ] || continue
  case "$script" in
  scripts/* | .github/scripts/*) ;;
  *)
    fail "${script} is shell outside scripts/ and .github/scripts/. C13 names those two trees so that every script has one mirrored test path; move it, or inline it into the nix or hk file that calls it."
    continue
    ;;
  esac
  expected="tests/unit/${script%.sh}.bats"
  if ! git ls-files --error-unmatch "$expected" >/dev/null 2>&1; then
    fail "${script} has no test. Expected one at ${expected} (V21)."
  fi
done < <(git ls-files '*.sh')

# Direction two: every test has its script.
while IFS= read -r test_file; do
  [ -n "$test_file" ] || continue
  script="${test_file#tests/unit/}"
  script="${script%.bats}.sh"
  if ! git ls-files --error-unmatch "$script" >/dev/null 2>&1; then
    fail "${test_file} tests nothing: ${script} does not exist. An orphan test passes forever -- delete it, or restore the script it covered (V21)."
  fi
done < <(git ls-files 'tests/unit/*.bats')

exit "$status"
