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

@test "the dev shell carries cargo-release 1.1.6" {
  run nix develop --no-write-lock-file --command cargo release --version
  [ "$status" -eq 0 ]
  [[ "$output" == *"cargo-release 1.1.6"* ]]
}

@test "the flake checks its exact root input set" {
  # The check is an `assert` the attribute forces at eval time, so evaluating
  # its drvPath proves it on every host. Building it would need an
  # x86_64-linux builder, which the arm and macOS runners do not have.
  run nix eval --no-write-lock-file --raw .#checks.x86_64-linux.input-set.drvPath
  [ "$status" -eq 0 ]
}
