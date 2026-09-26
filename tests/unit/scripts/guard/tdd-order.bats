#!/usr/bin/env bats
#
# Mirror of `scripts/guard/tdd-order.sh` (scripts:C13).
#
# The rule (scripts/guard:C11, scripts/guard:V16): a RED test commit
# precedes the GREEN commit that makes it pass. Written the other way
# round, the test is shaped by the code it was meant to constrain, and the
# only proof it can ever fail is that somebody says so.
#
# The guard walks a commit RANGE, because the ordering is a property of
# history and not of any single tree.

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat `git -C`
  # (tests:B1). Drop every GIT_* variable, keep git off the user's and the
  # system's config, and stop repository discovery at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/tdd-order.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet
  git -C "$REPO" config user.email t@example.com
  git -C "$REPO" config user.name test
  commit chore "seed the history" seed.txt
  BASE="$(git -C "$REPO" rev-parse HEAD)"
}

# One commit: a conventional subject, and the files it writes.
commit() {
  local type="$1" summary="$2"
  shift 2
  local path
  for path in "$@"; do
    mkdir -p "${REPO}/$(dirname "$path")"
    printf '#!/usr/bin/env bash\n# %s\n' "$summary" >>"${REPO}/${path}"
    git -C "$REPO" add "$path"
  done
  git -C "$REPO" commit --quiet --no-verify -m "${type}: ${summary}

Why: fixture."
}

run_guard() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT" "${1:-${BASE}..HEAD}"
}

@test "a test commit followed by the script it covers passes" {
  commit test "cover the thing" tests/unit/scripts/dev/thing.bats
  commit feat "add the thing" scripts/dev/thing.sh
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "a script and its test in the SAME commit fails: the test must precede" {
  commit feat "add the thing with its test" scripts/dev/thing.sh tests/unit/scripts/dev/thing.bats
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"precede"* ]]
  [[ "$output" == *"scripts/dev/thing.sh"* ]]
}

@test "a script with no test anywhere fails" {
  commit feat "add the thing" scripts/dev/thing.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"tests/unit/scripts/dev/thing.bats"* ]]
}

@test "editing a script whose test already exists passes" {
  commit test "cover the thing" tests/unit/scripts/dev/thing.bats
  commit feat "add the thing" scripts/dev/thing.sh
  commit fix "repair the thing" scripts/dev/thing.sh
  run_guard
  [ "$status" -eq 0 ]
}

@test "a new .rs file preceded by a test commit passes" {
  commit test "cover the parser" src/parser/tests.rs
  commit feat "add the parser" src/parser/mod.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "a new .rs file with no test commit before it fails" {
  commit feat "add the parser" src/parser/mod.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/parser/mod.rs"* ]]
  [[ "$output" == *"test"* ]]
}

@test "a commit that only adds tests passes" {
  commit test "cover the thing" tests/unit/scripts/dev/thing.bats
  run_guard
  [ "$status" -eq 0 ]
}

@test "commits touching neither Rust nor shell pass" {
  commit docs "write the readme" README.md
  commit chore "tidy" .gitignore
  run_guard
  [ "$status" -eq 0 ]
}

@test "an empty range passes" {
  run_guard "HEAD..HEAD"
  [ "$status" -eq 0 ]
}

@test "every offending commit is named, not only the first" {
  commit feat "add one" scripts/dev/one.sh
  commit feat "add two" scripts/dev/two.sh
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"scripts/dev/one.sh"* ]]
  [[ "$output" == *"scripts/dev/two.sh"* ]]
}

@test "outside a git work tree it fails rather than passing vacuously" {
  REPO="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$REPO"
  run_guard "HEAD~1..HEAD"
  [ "$status" -ne 0 ]
  [[ "$output" == *"git"* ]]
}

@test "an unresolvable range fails rather than checking nothing" {
  run_guard "nosuchref..HEAD"
  [ "$status" -ne 0 ]
  [[ "$output" == *"nosuchref"* ]]
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
  commit test "cover the thing" tests/unit/scripts/dev/thing.bats
  commit feat "add the thing" scripts/dev/thing.sh
  run_guard
  [ "$status" -eq 0 ]
  sentinel_untouched
}
