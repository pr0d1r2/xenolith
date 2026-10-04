#!/usr/bin/env bats
#
# Mirror of `scripts/nix/tools.sh` (scripts:C13).
#
# The script is the body of the flake's tool checks (`nix:T99`,
# `nix:V96`, `nix:V251`): write a fixture repo holding a file for every
# host and guest dialect, run the packaged `xnl lint` over it, and pass
# only when no check went unspawned -- no exit 2, no `status: error` --
# and every named tool actually ran.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/nix/tools.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  OUT="${BATS_TEST_TMPDIR}/out"
  mkdir -p "$BIN"
}

# A stub `xnl` that records its arguments and the fixture it ran in, then
# prints a `--format json` report with one passing result per tool given
# and exits with $XNL_EXIT. $XNL_STATUS replaces the last result's status.
stub_xnl() {
  {
    echo "#!${BASH}"
    echo "echo \"\$*\" >\"${BATS_TEST_TMPDIR}/xnl-args\""
    echo "ls -A | LC_ALL=C sort >\"${BATS_TEST_TMPDIR}/xnl-fixture\""
    echo "echo '{'"
    for tool in "$@"; do
      echo "echo '      \"check\": \"${tool}\",'"
      echo "echo '      \"status\": \"pass\"'"
    done
    echo "[ -z \"\${XNL_STATUS:-}\" ] || echo \"      \\\"status\\\": \\\"\${XNL_STATUS}\\\"\""
    echo "echo '}'"
    echo "exit \${XNL_EXIT:-0}"
  } >"${BIN}/xnl"
  chmod +x "${BIN}/xnl"
}

run_tools() {
  PATH="${BIN}:${PATH}" run bash "$SCRIPT" "$@"
}

@test "every named tool ran and none errored: passes with the output" {
  stub_xnl statix nixfmt
  run_tools "$OUT" statix nixfmt
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "runs xnl lint as json over the fixture, from inside it" {
  stub_xnl statix
  run_tools "$OUT" statix
  [ "$(cat "${BATS_TEST_TMPDIR}/xnl-args")" = "lint --format json ." ]
}

@test "the fixture holds a file for every host and guest dialect" {
  stub_xnl statix
  run_tools "$OUT" statix
  fixture="$(cat "${BATS_TEST_TMPDIR}/xnl-fixture")"
  for file in xenolith.toml default.nix justfile a.xml a.tcl posix.sh bash.sh z.zsh; do
    [[ "$fixture" == *"$file"* ]]
  done
}

@test "findings (exit 1) still prove the tools ran" {
  stub_xnl statix
  XNL_EXIT=1 run_tools "$OUT" statix
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "a refused lint (exit 2, a tool not on PATH) fails the check" {
  stub_xnl statix
  XNL_EXIT=2 run_tools "$OUT" statix
  [ "$status" -eq 1 ]
  [ ! -e "$OUT" ]
}

@test "a check that errored fails even when the exit code says findings" {
  stub_xnl statix
  XNL_EXIT=1 XNL_STATUS=error run_tools "$OUT" statix
  [ "$status" -eq 1 ]
  [ ! -e "$OUT" ]
}

@test "a named tool that never ran fails the check, naming it" {
  stub_xnl statix
  run_tools "$OUT" statix xmllint
  [ "$status" -eq 1 ]
  [[ "$output" == *"xmllint"* ]]
  [ ! -e "$OUT" ]
}

@test "no tools named is a build that must still spawn every check it runs" {
  stub_xnl
  run_tools "$OUT"
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "no output path is a usage error" {
  stub_xnl statix
  run_tools
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
  [ ! -f "${BATS_TEST_TMPDIR}/xnl-args" ]
}
