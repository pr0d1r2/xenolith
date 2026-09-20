#!/usr/bin/env bats
#
# Mirror of `scripts/dev/shell-hook.sh` (scripts:C13). The hook is the only
# thing that installs the gate (scripts:C10), so every way it can decline to
# install has to be a way it says so out loud (scripts:V122).

setup() {
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
  run_hook
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
