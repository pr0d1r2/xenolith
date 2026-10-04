#!/usr/bin/env bats
#
# Mirror of `scripts/hk/bats-ready.sh` (scripts:C13).
#
# The pre-commit `bats` step runs this rather than the whole suite
# (scripts:V352): a RED bats commit adds a test before the script it
# covers (scripts:C11), and the whole suite would refuse it. Every test
# whose script exists still runs, and push runs them all.

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat the
  # test's own repository (tests:B1). Drop every GIT_* variable, keep git
  # off the user's and the system's config, and stop repository discovery
  # at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/hk/bats-ready.sh"
  RUN_TOOL="${BATS_TEST_DIRNAME}/../../../../scripts/hk/run-tool.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$BIN" "${REPO}/scripts/hk"
  # The script finds run-tool.sh beside itself, so the copy under test
  # sits in a tree of its own with the real run-tool.sh next to it.
  cp "$SCRIPT" "$RUN_TOOL" "${REPO}/scripts/hk/"
  cd "$REPO"
}

# A stub bats that prints each argument on its own line and exits with a
# chosen code. Its shebang names bash absolutely: PATH holds only this
# directory, so `/usr/bin/env bash` would find no bash at all.
stub_bats() {
  local code="${1:-0}"
  cat >"${BIN}/bats" <<STUB
#!${BASH}
printf '%s\n' "\$@"
exit ${code}
STUB
  chmod +x "${BIN}/bats"
}

# A test file, and optionally the script it mirrors.
mirror() {
  local path="$1" built="${2:-built}"
  mkdir -p "tests/unit/$(dirname "$path")" "$(dirname "$path")"
  touch "tests/unit/${path%.sh}.bats"
  if [ "$built" = built ]; then
    touch "$path"
  fi
}

# The test tree as a git repository with everything so far committed, and
# git on the staged PATH, so the index says what a commit would hold.
git_repo() {
  ln -s "$(command -v git)" "${BIN}/git"
  git init --quiet
  git config user.email t@example.com
  git config user.name test
  git add -A
  git commit --quiet --no-verify -m "chore: seed"
}

# PATH is REPLACED, not prepended to: the suite runs inside the dev shell,
# where the real bats is present, so a missing one has to be staged.
run_ready() {
  PATH="${BIN}" run "$BASH" scripts/hk/bats-ready.sh "$@"
}

@test "every test whose script exists runs, in one bats call" {
  stub_bats
  mirror scripts/a.sh
  mirror scripts/guard/b.sh
  mirror .github/scripts/c.sh
  run_ready
  [ "$status" -eq 0 ]
  [[ "$output" == *"tests/unit/scripts/a.bats"* ]]
  [[ "$output" == *"tests/unit/scripts/guard/b.bats"* ]]
  [[ "$output" == *"tests/unit/.github/scripts/c.bats"* ]]
}

@test "a test whose script does not exist yet is held back, and named" {
  stub_bats
  mirror scripts/a.sh
  mirror scripts/guard/red.sh unbuilt
  run_ready
  [ "$status" -eq 0 ]
  [[ "$output" == *"tests/unit/scripts/a.bats"* ]]
  [[ "$output" == *"holding back tests/unit/scripts/guard/red.bats"* ]]
  [[ "$output" == *"scripts/guard/red.sh"* ]]
  [ "$(grep -c '^tests/unit/scripts/guard/red.bats$' <<<"$output")" -eq 0 ]
}

@test "a failing test fails the step with bats' own exit code" {
  stub_bats 1
  mirror scripts/a.sh
  run_ready
  [ "$status" -eq 1 ]
}

@test "only RED tests is not a failure: nothing is ready, bats is not run" {
  stub_bats 9
  mirror scripts/red.sh unbuilt
  run_ready
  [ "$status" -eq 0 ]
  [[ "$output" == *"holding back tests/unit/scripts/red.bats"* ]]
}

@test "no test file at all fails rather than passing vacuously" {
  stub_bats
  mkdir -p tests/unit
  run_ready
  [ "$status" -ne 0 ]
  [[ "$output" == *"no bats file"* ]]
}

@test "a test directory that does not exist fails" {
  stub_bats
  run_ready no/such/dir
  [ "$status" -ne 0 ]
  [[ "$output" == *"no/such/dir"* ]]
}

@test "another test directory can be named" {
  stub_bats
  mkdir -p other/scripts scripts
  touch other/scripts/a.bats scripts/a.sh
  run_ready other
  [ "$status" -eq 0 ]
  [[ "$output" == *"other/scripts/a.bats"* ]]
}

@test "a path with a space reaches bats as one argument" {
  stub_bats
  mirror "scripts/two words.sh"
  run_ready
  [ "$status" -eq 0 ]
  [ "$(grep -c '^tests/unit/scripts/two words.bats$' <<<"$output")" -eq 1 ]
}

@test "a missing bats is a missing tool, not a pass" {
  mirror scripts/a.sh
  run_ready
  [ "$status" -ne 0 ]
  [[ "$output" == *"not a finding"* ]]
}

@test "more than one argument is a usage error" {
  stub_bats
  run_ready a b
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
}

@test "this repository's ready tests are found" {
  cd "${BATS_TEST_DIRNAME}/../../../.."
  stub_bats
  PATH="${BIN}" run "$BASH" scripts/hk/bats-ready.sh
  [ "$status" -eq 0 ]
  [[ "$output" == *"tests/unit/scripts/hk/run-tool.bats"* ]]
}

@test "a test staged without its script is held back: RED for an existing script" {
  stub_bats
  mirror scripts/a.sh
  mirror scripts/b.sh
  git_repo
  echo "# a new case" >>tests/unit/scripts/a.bats
  git add tests/unit/scripts/a.bats
  run_ready
  [ "$status" -eq 0 ]
  [[ "$output" == *"holding back tests/unit/scripts/a.bats"* ]]
  [[ "$output" == *"staged without scripts/a.sh"* ]]
  [ "$(grep -c '^tests/unit/scripts/a.bats$' <<<"$output")" -eq 0 ]
  [ "$(grep -c '^tests/unit/scripts/b.bats$' <<<"$output")" -eq 1 ]
}

@test "a test staged with its script runs: that is the GREEN commit" {
  stub_bats
  mirror scripts/a.sh
  git_repo
  echo "# a new case" >>tests/unit/scripts/a.bats
  echo "# the change" >>scripts/a.sh
  git add tests/unit/scripts/a.bats scripts/a.sh
  run_ready
  [ "$status" -eq 0 ]
  [ "$(grep -c '^tests/unit/scripts/a.bats$' <<<"$output")" -eq 1 ]
  [[ "$output" != *"holding back"* ]]
}

@test "a changed test that is not staged still runs" {
  stub_bats
  mirror scripts/a.sh
  git_repo
  echo "# a new case" >>tests/unit/scripts/a.bats
  run_ready
  [ "$status" -eq 0 ]
  [ "$(grep -c '^tests/unit/scripts/a.bats$' <<<"$output")" -eq 1 ]
}

@test "outside a git work tree nothing is held back for being staged" {
  stub_bats
  ln -s "$(command -v git)" "${BIN}/git"
  mirror scripts/a.sh
  run_ready
  [ "$status" -eq 0 ]
  [ "$(grep -c '^tests/unit/scripts/a.bats$' <<<"$output")" -eq 1 ]
}
