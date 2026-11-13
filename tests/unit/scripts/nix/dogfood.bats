#!/usr/bin/env bats
#
# Mirror of `scripts/nix/dogfood.sh` (scripts:C13).
#
# The script is the body of the flake's `dogfood` check (`nix:T26`,
# `.:V19`): run the packaged `xnl check` over the source tree the flake
# was given, and produce the check's output only when it passes. It is a
# script rather than an inline builder string because that string would be
# a shell sequence embedded in nix -- the thing `xnl check` flags.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/nix/dogfood.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  SRC="${BATS_TEST_TMPDIR}/src"
  OUT="${BATS_TEST_TMPDIR}/out"
  mkdir -p "$BIN" "$SRC"
}

# A stub `xnl` that records where it ran and with what, then exits with a
# chosen code. The shebang names bash absolutely because PATH is replaced.
stub_xnl() {
  cat >"${BIN}/xnl" <<STUB
#!${BASH}
echo "\$(pwd) \$*" >>"${BATS_TEST_TMPDIR}/xnl-calls"
exit ${1:-0}
STUB
  chmod +x "${BIN}/xnl"
}

run_dogfood() {
  PATH="${BIN}:${PATH}" run "$BASH" "$SCRIPT" "$@"
}

@test "runs xnl check over the source tree, from inside it" {
  stub_xnl
  run_dogfood "$SRC" "$OUT"
  [ "$status" -eq 0 ]
  [ "$(cat "${BATS_TEST_TMPDIR}/xnl-calls")" = "$(cd "$SRC" && pwd) check ." ]
}

@test "a clean check produces the output path" {
  stub_xnl
  run_dogfood "$SRC" "$OUT"
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "a failing check keeps its exit code and produces no output" {
  stub_xnl 1
  run_dogfood "$SRC" "$OUT"
  [ "$status" -eq 1 ]
  [ ! -e "$OUT" ]
}

@test "a refused check (exit 2) is not reported as a finding" {
  stub_xnl 2
  run_dogfood "$SRC" "$OUT"
  [ "$status" -eq 2 ]
  [ ! -e "$OUT" ]
}

@test "the wrong number of arguments is a usage error" {
  stub_xnl
  run_dogfood "$SRC"
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
  [ ! -f "${BATS_TEST_TMPDIR}/xnl-calls" ]
}

@test "a source directory that does not exist fails before xnl runs" {
  stub_xnl
  run_dogfood "${BATS_TEST_TMPDIR}/missing" "$OUT"
  [ "$status" -ne 0 ]
  [ ! -f "${BATS_TEST_TMPDIR}/xnl-calls" ]
  [ ! -e "$OUT" ]
}
