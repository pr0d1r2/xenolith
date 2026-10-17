#!/usr/bin/env bats

setup() {
  cd "${BATS_TEST_DIRNAME}/../../../.."
}

@test "the Intel Darwin package evaluates and has a buildable system" {
  run nix eval --raw .#packages.x86_64-darwin.default.system
  [ "$status" -eq 0 ]
  [ "$output" = "x86_64-darwin" ]
}

