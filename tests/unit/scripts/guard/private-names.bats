#!/usr/bin/env bats
#
# Mirror of `scripts/guard/private-names.sh` (scripts:C13).
#
# The rule (scripts/guard:V23, scripts/guard:C17): no private repo name
# appears in a tracked file. Unknown means private, so the list is a local
# file the repo never carries -- publishing the denylist would publish
# exactly what it protects.
#
# Which is also why the guard reports the LINE NUMBER in the denylist
# rather than the name it matched: a gate that prints the secret into
# every CI log has leaked it.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/private-names.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet
}

denylist() {
  printf '%s\n' "$@" >"${REPO}/.private-names"
}

track() {
  local path="$1" content="$2"
  mkdir -p "${REPO}/$(dirname "$path")"
  printf '%s\n' "$content" >"${REPO}/${path}"
  git -C "$REPO" add "$path"
}

run_guard() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT"
}

@test "a clean tree passes" {
  denylist "acmeinternal"
  track docs/notes.md "nothing to see"
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "a denied name in a tracked file fails" {
  denylist "acmeinternal"
  track docs/notes.md "we copied this from acmeinternal last week"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"docs/notes.md"* ]]
}

@test "the report names the denylist line, never the matched name" {
  denylist "acmeinternal"
  track docs/notes.md "acmeinternal"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" != *"acmeinternal"* ]]
  [[ "$output" == *"line 1"* ]]
}

@test "a denied name in a PATH fails, not only in content" {
  denylist "acmeinternal"
  track "docs/acmeinternal-migration.md" "harmless body"
  run_guard
  [ "$status" -ne 0 ]
}

@test "matching ignores case" {
  denylist "acmeinternal"
  track docs/notes.md "See AcmeInternal for the original."
  run_guard
  [ "$status" -ne 0 ]
}

@test "comments and blank lines in the denylist are not patterns" {
  denylist "# a comment" "" "acmeinternal"
  track docs/notes.md "a comment about nothing"
  run_guard
  [ "$status" -eq 0 ]
}

@test "untracked files are not scanned" {
  denylist "acmeinternal"
  printf 'acmeinternal\n' >"${REPO}/scratch.txt"
  run_guard
  [ "$status" -eq 0 ]
}

@test "the denylist itself is never scanned" {
  denylist "acmeinternal"
  run_guard
  [ "$status" -eq 0 ]
}

@test "no denylist passes, and says the check did not run" {
  track docs/notes.md "anything"
  run_guard
  [ "$status" -eq 0 ]
  [[ "$output" == *".private-names"* ]]
}

@test "an empty denylist passes, and says the check did not run" {
  : >"${REPO}/.private-names"
  track docs/notes.md "anything"
  run_guard
  [ "$status" -eq 0 ]
  [[ "$output" == *".private-names"* ]]
}

@test "every match is reported, not only the first" {
  denylist "acmeinternal"
  track docs/one.md "acmeinternal"
  track docs/two.md "acmeinternal"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"docs/one.md"* ]]
  [[ "$output" == *"docs/two.md"* ]]
}

@test "outside a git work tree it fails rather than passing vacuously" {
  REPO="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$REPO"
  denylist "acmeinternal"
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
  denylist acme
  track notes.txt "nothing private here"
  run_guard
  [ "$status" -eq 0 ]
  sentinel_untouched
}
