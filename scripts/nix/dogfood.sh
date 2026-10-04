#!/usr/bin/env bash
#
# The flake's `dogfood` check (`nix:T26`, `.:V19`, `.:C19`): run the packaged
# `xnl check` over the source tree the flake was given, and create the
# check's output only when that run is clean.
#
# A script rather than a builder string in nix/checks.nix: the same three
# steps inline would be a shell sequence embedded in nix, which is exactly
# what `xnl check` flags. The flake's source is not a git repository, so
# `xnl check .` names the tree explicitly and discovery walks it
# (`src/discover:V57`); running from inside it makes it the root of the nested
# `xenolith.toml` chain (`src/config:V88`).
#
#   scripts/nix/dogfood.sh SOURCE_DIR OUT
set -euo pipefail

if [ "$#" -ne 2 ]; then
  echo "usage: dogfood.sh SOURCE_DIR OUT" >&2
  exit 2
fi

src="$1"
out="$2"

cd "$src"
# Not `exec`: the output is created only after a clean run, and a failing
# run keeps its own exit code (1 = findings, 2 = refused) under `set -e`.
xnl check .
touch "$out"
