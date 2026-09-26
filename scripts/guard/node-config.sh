#!/usr/bin/env bash
#
# Every sherd node dir holds its own `xenolith.toml` (`.:V90`,
# `src/config:V88`). A node dir is a directory holding a tracked SPEC.md,
# the root included; the file beside it may say no more than `version = 1`
# -- configs hold deviations only (`src/config:V89`) -- but it must exist,
# because a node checked on its own (`cd node && xnl check`, or from its
# crates.io package) reads no ancestor. Without one, the node's verdict
# silently becomes whatever the defaults say rather than what its authors
# wrote.
#
# Lists every node dir missing one, one line each, in path order, and
# exits 1; silent and 0 when every node has its file. Tracked files only,
# as the other guards: an untracked SPEC.md is not a node, and an
# untracked xenolith.toml would not ship.
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "node-config: not inside a git work tree -- the node list comes from git, so nothing could be checked. This is a failure, not a pass." >&2
  exit 1
fi

status=0

while IFS= read -r spec; do
  [ -n "$spec" ] || continue
  case "$spec" in
  */*)
    dir="${spec%/SPEC.md}"
    config="${dir}/xenolith.toml"
    ;;
  *)
    dir="."
    config="xenolith.toml"
    ;;
  esac
  if ! git ls-files --error-unmatch -- "$config" >/dev/null 2>&1; then
    echo "node-config: ${dir} is a sherd node (it holds SPEC.md) with no tracked ${config}; add one saying at least version = 1 (.:V90, src/config:V88)." >&2
    status=1
  fi
done < <(git ls-files -- 'SPEC.md' '*/SPEC.md' | LC_ALL=C sort)

exit "$status"
