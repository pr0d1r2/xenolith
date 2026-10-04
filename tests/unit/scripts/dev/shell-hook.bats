#!/usr/bin/env bats
#
# Mirror of `scripts/dev/shell-hook.sh` (scripts:C13). The hook is the only
# thing that installs the gate (scripts:C10), so every way it can decline to
# install has to be a way it says so out loud (scripts:V122).

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat `git -C`
  # (tests:B1). Drop every GIT_* variable, keep git off the user's and the
  # system's config, and stop repository discovery at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/dev/shell-hook.sh"
  WORK="${BATS_TEST_TMPDIR}/work"
  BIN="${BATS_TEST_TMPDIR}/bin"
  mkdir -p "$WORK" "$BIN"
  git -C "$WORK" init --quiet
}

# A stub `hk` that records every invocation, so a test can assert both THAT
# it ran and WITH WHAT -- a hook that runs the wrong subcommand installs
# nothing while looking perfectly healthy.
stub_hk() {
  cat >"${BIN}/hk" <<STUB
#!/usr/bin/env bash
echo "\$*" >>"${BATS_TEST_TMPDIR}/hk-calls"
exit ${1:-0}
STUB
  chmod +x "${BIN}/hk"
}

run_hook() {
  cd "$WORK" || return 1
  PATH="${BIN}:${PATH}" run bash "$SCRIPT"
}

# The dev shell this suite runs IN has hk on PATH, so "hk is missing" has to
# be staged rather than assumed: PATH is replaced, not prepended to. The
# script checks for hk before it calls anything else, so a PATH holding only
# the stub directory is enough to reach that branch. bash is named by its
# absolute path because PATH no longer resolves it.
run_hook_without_hk() {
  cd "$WORK" || return 1
  PATH="${BIN}" run "$BASH" "$SCRIPT"
}

@test "installs the hooks via hk install" {
  stub_hk
  run_hook
  [ "$status" -eq 0 ]
  [ "$(cat "${BATS_TEST_TMPDIR}/hk-calls")" = "install" ]
}

@test "is idempotent: a second entry installs again without complaint" {
  stub_hk
  run_hook
  [ "$status" -eq 0 ]
  run_hook
  [ "$status" -eq 0 ]
  [ "$(wc -l <"${BATS_TEST_TMPDIR}/hk-calls" | tr -d ' ')" = "2" ]
}

@test "refuses loudly when hk is off PATH, rather than skipping silently" {
  run_hook_without_hk
  [ "$status" -ne 0 ]
  [[ "$output" == *"hk"* ]]
  [[ "$output" == *"PATH"* ]]
}

@test "propagates a failing hk install instead of reporting success" {
  stub_hk 3
  run_hook
  [ "$status" -eq 3 ]
}

@test "outside a git work tree it says so and does not run hk install" {
  stub_hk
  WORK="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$WORK"
  run_hook
  [ "$status" -eq 0 ]
  [[ "$output" == *"git"* ]]
  [ ! -f "${BATS_TEST_TMPDIR}/hk-calls" ]
}

@test "success is silent" {
  stub_hk
  run_hook
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
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
  stub_hk
  run_hook
  [ "$status" -eq 0 ]
  sentinel_untouched
}

# The READ half of the same leak: with GIT_DIR inherited, a directory that
# is no work tree looks like one, and the hook installs into the wrong repo.
@test "a git hook's environment does not make a non-repo look like one" {
  hook_env_over_sentinel
  setup
  stub_hk
  WORK="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$WORK"
  run_hook
  [ "$status" -eq 0 ]
  [[ "$output" == *"git"* ]]
  [ ! -f "${BATS_TEST_TMPDIR}/hk-calls" ]
  sentinel_untouched
}
