#!/usr/bin/env bats
#
# Mirror of `scripts/guard/bats-mirror.sh` (scripts:C13) -- and, since the
# rule is one-to-one, this file is also its own first fixture.
#
# The rule (scripts/guard:V21, scripts:C13): every shell script under
# `scripts/` or `.github/scripts/` has a bats file at the mirrored path
# under `tests/unit/`, and every bats file has the script it tests. Both
# directions, because an orphan test is a test that passes forever: the
# script it covered was renamed, and nothing said so.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/bats-mirror.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet
}

# Tracked files only: an untracked scratch script is not part of the repo,
# and a guard that fails on one would fail during every experiment.
track() {
  local path="$1"
  mkdir -p "${REPO}/$(dirname "$path")"
  printf '#!/usr/bin/env bash\n' >"${REPO}/${path}"
  git -C "$REPO" add "$path"
}

run_guard() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT"
}

@test "a matched pair passes" {
  track scripts/dev/thing.sh
  track tests/unit/scripts/dev/thing.bats
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "an empty repo passes" {
  run_guard
  [ "$status" -eq 0 ]
}

@test "a script with no bats fails and names the missing path" {
  track scripts/dev/thing.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"tests/unit/scripts/dev/thing.bats"* ]]
}

@test "an orphan bats fails and names the script it expected" {
  track tests/unit/scripts/dev/gone.bats
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"scripts/dev/gone.sh"* ]]
}

@test "the .github/scripts tree mirrors the same way" {
  track .github/scripts/ci/publish.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"tests/unit/.github/scripts/ci/publish.bats"* ]]
}

@test "a matched pair under .github/scripts passes" {
  track .github/scripts/ci/publish.sh
  track tests/unit/.github/scripts/ci/publish.bats
  run_guard
  [ "$status" -eq 0 ]
}

@test "shell outside the two allowed trees is a violation of its own" {
  track src/helper.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/helper.sh"* ]]
}

@test "an untracked script is not the guard's business" {
  mkdir -p "${REPO}/scripts/dev"
  printf '#!/usr/bin/env bash\n' >"${REPO}/scripts/dev/scratch.sh"
  run_guard
  [ "$status" -eq 0 ]
}

@test "reports every violation, not only the first" {
  track scripts/a.sh
  track scripts/b.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"tests/unit/scripts/a.bats"* ]]
  [[ "$output" == *"tests/unit/scripts/b.bats"* ]]
}

@test "outside a git repository it fails rather than passing vacuously" {
  REPO="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$REPO"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"git"* ]]
}

# tests:V150, tests:B1. git exports GIT_DIR and GIT_INDEX_FILE to every hook,
# and hk runs this suite from pre-commit and pre-push. The environment beats
# `git -C`, so unless setup drops it, every fixture write above lands in the
# repository the hook is running for. Staged here over a sentinel repo: setup
# runs again under a hook-shaped environment, the fixture does its usual git
# work, and the sentinel must come out exactly as it went in.
hook_env_over_sentinel() {
  SENTINEL="${BATS_TEST_TMPDIR}/sentinel"
  git init --quiet "$SENTINEL"
  SENTINEL_CONFIG="$(cat "${SENTINEL}/.git/config")"
  export GIT_DIR="${SENTINEL}/.git" GIT_INDEX_FILE="${SENTINEL}/.git/index"
}

sentinel_untouched() {
  [ "$(cat "${SENTINEL}/.git/config")" = "$SENTINEL_CONFIG" ]
  [ ! -e "${SENTINEL}/.git/index" ]
  [ -z "$(find "${SENTINEL}/.git/objects" -type f)" ]
}

@test "a git hook's environment does not reach the enclosing repo" {
  hook_env_over_sentinel
  setup
  track scripts/dev/thing.sh
  track tests/unit/scripts/dev/thing.bats
  run_guard
  [ "$status" -eq 0 ]
  sentinel_untouched
}
