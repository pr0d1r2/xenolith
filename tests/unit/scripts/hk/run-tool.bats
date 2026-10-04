#!/usr/bin/env bats
#
# Mirror of `scripts/hk/run-tool.sh` (scripts:C13).
#
# The script exists because a gate step may not carry inline shell
# (scripts:C9, scripts:C21): "the tool is missing" and "the tool found
# something" are different verdicts, and a step that cannot tell them apart
# reports a clean repo as clean when nothing ran.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/hk/run-tool.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  mkdir -p "$BIN"
}

# A stub tool that echoes its arguments and exits with a chosen code. Its
# shebang names bash absolutely: these stubs run under a PATH that holds
# only this directory, so `/usr/bin/env bash` would find no bash at all.
stub_tool() {
  local name="$1" code="${2:-0}"
  cat >"${BIN}/${name}" <<STUB
#!${BASH}
echo "args: \$*"
exit ${code}
STUB
  chmod +x "${BIN}/${name}"
}

# PATH is REPLACED, not prepended to: the suite runs inside the dev shell,
# where the real tools are present, so a missing tool has to be staged.
# bash is named absolutely because PATH no longer resolves it.
run_tool() {
  PATH="${BIN}" run "$BASH" "$SCRIPT" "$@"
}

@test "runs the tool and passes its arguments through" {
  stub_tool fakelint
  run_tool fakelint --check a.txt "b c.txt"
  [ "$status" -eq 0 ]
  [ "$output" = "args: --check a.txt b c.txt" ]
}

@test "a finding is the tool's own exit code, unchanged" {
  stub_tool fakelint 7
  run_tool fakelint a.txt
  [ "$status" -eq 7 ]
}

@test "a missing tool fails, and says it is a missing tool rather than a finding" {
  run_tool fakelint a.txt
  [ "$status" -ne 0 ]
  [[ "$output" == *"fakelint"* ]]
  [[ "$output" == *"dev shell"* ]]
  [[ "$output" == *"not a finding"* ]]
}

@test "no tool named is a usage error, distinct from a finding" {
  run_tool
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
}

@test "adds nothing to the tool's own output" {
  stub_tool fakelint
  run_tool fakelint
  [ "$output" = "args: " ]
}
