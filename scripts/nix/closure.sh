#!/usr/bin/env bash
#
# The flake's closure checks (`nix:T38`, `nix:V29`, `nix:V250`): fail naming
# every store path of a closure whose package name is one of NAME..., and
# create the check's output only when none is.
#
# STORE_PATHS is closureInfo's `store-paths`, one path per line: the same
# set `nix path-info -r` prints, computed where a sandboxed check can read
# it. A path's name is what follows the hash; NAME matches it bare
# (`itok`), at a version (`cargo-1.95.0`) or as a split output
# (`libxml2-2.14.5-bin`), and never a longer name sharing the prefix
# (`cargo-hack`): that is a different package.
#
#   scripts/nix/closure.sh STORE_PATHS OUT [NAME...]
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: closure.sh STORE_PATHS OUT [NAME...]" >&2
  exit 2
fi

paths="$1"
out="$2"
shift 2

# Read first, so a missing file fails here under `set -e` instead of
# reading as an empty -- and so clean -- closure.
listing="$(cat "$paths")"

found=0
while IFS= read -r path; do
  [ -n "$path" ] || continue
  base="${path##*/}"
  name="${base#*-}"
  for forbidden in "$@"; do
    case "$name" in
    "$forbidden" | "$forbidden"-[0-9]*)
      echo "closure: ${path} is ${forbidden}, which this closure must not hold" >&2
      found=1
      ;;
    esac
  done
done <<<"$listing"

if [ "$found" -ne 0 ]; then
  exit 1
fi
touch "$out"
