#!/usr/bin/env bats
#
# Mirror of `scripts/guard/node-config.sh` (scripts:C13).
#
# The rule (.:V90, src/config:V88): every sherd node dir -- every directory
# holding a tracked SPEC.md, the root included -- holds a tracked
# `xenolith.toml`, so the node is checkable on its own. The guard lists
# every node dir that lacks one.

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat `git -C`
  # (tests:B1). Drop every GIT_* variable, keep git off the user's and the
  # system's config, and stop repository discovery at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/node-config.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet
}

# A tracked file, with the content given (`version = 1` by default).
track() {
  local path="$1"
  local body="${2:-version = 1}"
  mkdir -p "${REPO}/$(dirname "$path")"
  printf '%s\n' "$body" >"${REPO}/${path}"
  git -C "$REPO" add "$path"
}

# A node: its SPEC.md, and its xenolith.toml.
node() {
  local dir="$1" prefix=""
  [ "$dir" = "." ] || prefix="${dir}/"
  track "${prefix}SPEC.md" "# SPEC"
  track "${prefix}xenolith.toml"
}

run_guard() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT"
}

@test "an empty repo passes" {
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "every node with its xenolith.toml passes silently" {
  node .
  node src
  node src/config
  node languages/api/src/holes
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "a root SPEC.md without a root xenolith.toml fails naming the root" {
  track SPEC.md "# SPEC"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"xenolith.toml"* ]]
  [[ "$output" == *"."* ]]
  [[ "$output" == *".:V90"* ]]
}

@test "a nested node without xenolith.toml fails naming its dir" {
  node .
  track src/config/SPEC.md "# SPEC"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/config/xenolith.toml"* ]]
}

@test "every missing node is listed, one line each, in path order" {
  node .
  track src/SPEC.md "# SPEC"
  track docs/SPEC.md "# SPEC"
  node nix
  run_guard
  [ "$status" -ne 0 ]
  [ "${#lines[@]}" -eq 2 ]
  [[ "${lines[0]}" == *"docs/xenolith.toml"* ]]
  [[ "${lines[1]}" == *"src/xenolith.toml"* ]]
}

@test "an untracked xenolith.toml does not count" {
  node .
  track src/SPEC.md "# SPEC"
  printf 'version = 1\n' >"${REPO}/src/xenolith.toml"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/xenolith.toml"* ]]
}

@test "an untracked SPEC.md is not a node" {
  node .
  mkdir -p "${REPO}/scratch"
  printf '# SPEC\n' >"${REPO}/scratch/SPEC.md"
  run_guard
  [ "$status" -eq 0 ]
}

@test "a xenolith.toml without a SPEC.md beside it is fine" {
  # Any directory may hold one (src/config §I); only nodes must.
  node .
  track vendor/xenolith.toml
  run_guard
  [ "$status" -eq 0 ]
}

@test "a SPEC.md in a directory with spaces is handled" {
  node .
  track "odd dir/SPEC.md" "# SPEC"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"odd dir/xenolith.toml"* ]]
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
  node .
  node src
  run_guard
  [ "$status" -eq 0 ]
  sentinel_untouched
}
