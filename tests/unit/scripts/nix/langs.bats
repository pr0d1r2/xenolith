#!/usr/bin/env bats
#
# Mirror of `scripts/nix/langs.sh` (scripts:C13).
#
# The script is the body of the flake's subset check (`nix:T41`,
# `nix:V31`): the packaged `xnl langs` must list EXACTLY the languages the
# `languages` override asked for as compiled in -- none missing, none
# extra -- and the check's output exists only when it does.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/nix/langs.sh"
  BIN="${BATS_TEST_TMPDIR}/bin"
  OUT="${BATS_TEST_TMPDIR}/out"
  mkdir -p "$BIN"
}

# A stub `xnl` whose `langs` prints the human table for the compiled-in
# ids given, every other known id compiled out, and exits with $XNL_EXIT.
stub_xnl() {
  {
    echo "#!${BASH}"
    echo "echo \"\$*\" >>\"${BATS_TEST_TMPDIR}/xnl-calls\""
    for id in just nix pkl shell tcl xml yaml; do
      state=compiled-out
      for on in "$@"; do
        [ "$on" = "$id" ] && state=compiled-in
      done
      echo "echo '${id}  ${state}  lang-${id}'"
    done
    echo "exit \${XNL_EXIT:-0}"
  } >"${BIN}/xnl"
  chmod +x "${BIN}/xnl"
}

run_langs() {
  PATH="${BIN}:${PATH}" run bash "$SCRIPT" "$@"
}

@test "exactly the requested languages compiled in passes" {
  stub_xnl nix
  run_langs "$OUT" nix
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
  [ "$(cat "${BATS_TEST_TMPDIR}/xnl-calls")" = "langs" ]
}

@test "the order of the requested languages does not matter" {
  stub_xnl nix pkl shell
  run_langs "$OUT" shell nix pkl
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "an extra compiled-in language fails, naming it" {
  stub_xnl nix shell
  run_langs "$OUT" nix
  [ "$status" -eq 1 ]
  [[ "$output" == *"shell"* ]]
  [ ! -e "$OUT" ]
}

@test "a requested language that is compiled out fails, naming it" {
  stub_xnl nix
  run_langs "$OUT" nix pkl
  [ "$status" -eq 1 ]
  [[ "$output" == *"pkl"* ]]
  [ ! -e "$OUT" ]
}

@test "xnl langs failing fails the check with no output" {
  stub_xnl nix
  XNL_EXIT=2 run_langs "$OUT" nix
  [ "$status" -ne 0 ]
  [ ! -e "$OUT" ]
}

@test "no language named is a usage error" {
  stub_xnl nix
  run_langs "$OUT"
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
  [ ! -f "${BATS_TEST_TMPDIR}/xnl-calls" ]
}
