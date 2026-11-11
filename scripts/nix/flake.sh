#!/usr/bin/env bash
#
# The flake output check: evaluate the Intel Darwin package and verify that
# its derivation targets the declared system (`nix:C7`).
#
#   scripts/nix/flake.sh OUT
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: flake.sh OUT" >&2
  exit 2
fi

out="$1"
system="$(nix eval --raw .#packages.x86_64-darwin.default.system)"
if [ "$system" != "x86_64-darwin" ]; then
  echo "flake: expected x86_64-darwin package, got ${system}" >&2
  exit 1
fi

touch "$out"
