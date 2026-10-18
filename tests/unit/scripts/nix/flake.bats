#!/usr/bin/env bats

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/nix/flake.sh"
  cd "${BATS_TEST_DIRNAME}/../../../.."
}

@test "the Intel Darwin package evaluates and has a buildable system" {
  run bash "$SCRIPT" "$BATS_TEST_TMPDIR/out"
  [ "$status" -eq 0 ]
  [ -e "$BATS_TEST_TMPDIR/out" ]
}

@test "flake evaluation does not override a missing microlith input" {
  run nix flake metadata --no-write-lock-file
  [ "$status" -eq 0 ]
  [[ "$output" != *"input 'microlith' has an override for a non-existent input 'itok'"* ]]
}
