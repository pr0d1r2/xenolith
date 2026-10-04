#!/usr/bin/env bats
#
# Mirror of `scripts/hk/run-per-file.sh` (scripts:C13).
#
# Some tools take exactly one path per run -- `mth` says so and exits 2 when
# handed more. hk hands a step every changed file at once, so without this
# the spec steps would refuse on any commit that touched two SPEC.md files,
# which is most of them.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/hk/run-per-file.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  WORK="${BATS_TEST_TMPDIR}/work"
  mkdir -p "$BIN" "$WORK"
  CALLS="${BATS_TEST_TMPDIR}/calls"
}

# Records one line per invocation. Absolute shebang: these stubs run under a
# PATH holding only the stub directory.
stub_tool() {
  local name="$1" fail_on="${2:-}"
  cat >"${BIN}/${name}" <<STUB
#!${BASH}
echo "\$*" >>"${CALLS}"
if [ -n "${fail_on}" ] && [ "\${*: -1}" = "${fail_on}" ]; then
  echo "finding in \${*: -1}" >&2
  exit 1
fi
exit 0
STUB
  chmod +x "${BIN}/${name}"
}

run_per_file() {
  cd "$WORK" || return 1
  PATH="${BIN}" run "$BASH" "$SCRIPT" "$@"
}

@test "invokes the tool once per file, arguments repeated each time" {
  stub_tool faketool
  run_per_file faketool check -- a.md b.md c.md
  [ "$status" -eq 0 ]
  [ "$(cat "$CALLS")" = "check a.md
check b.md
check c.md" ]
}

@test "a tool with no flags still gets one file per run" {
  stub_tool faketool
  run_per_file faketool -- only.md
  [ "$status" -eq 0 ]
  [ "$(cat "$CALLS")" = "only.md" ]
}

@test "every file is visited even after one fails" {
  stub_tool faketool a.md
  run_per_file faketool check -- a.md b.md
  [ "$status" -ne 0 ]
  [ "$(wc -l <"$CALLS" | tr -d ' ')" = "2" ]
  [[ "$output" == *"finding in a.md"* ]]
}

@test "no files is success, not a silent skip of a broken invocation" {
  stub_tool faketool
  run_per_file faketool check --
  [ "$status" -eq 0 ]
  [ ! -f "$CALLS" ]
}

@test "a missing tool fails once, naming it as a missing tool" {
  run_per_file faketool check -- a.md
  [ "$status" -ne 0 ]
  [[ "$output" == *"faketool"* ]]
  [[ "$output" == *"not a finding"* ]]
}

@test "a missing -- separator is a usage error" {
  stub_tool faketool
  run_per_file faketool check a.md
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
}

@test "success is silent" {
  stub_tool faketool
  run_per_file faketool check -- a.md
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}
