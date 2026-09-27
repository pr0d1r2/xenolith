#!/usr/bin/env bash
#
# The flake's language-subset check (`nix:T41`, `nix:V31`): the packaged
# `xnl langs` must list EXACTLY LANG... as compiled in -- none missing, none
# extra -- and the check's output is created only when it does.
#
# The human table is `<id>  compiled-in|compiled-out  lang-<id>`, one id
# per line (`src/cli` §I). Both sides are sorted, so the order the override
# named its languages in does not matter.
#
#   scripts/nix/langs.sh OUT LANG...
set -euo pipefail

if [ "$#" -lt 2 ]; then
  echo "usage: langs.sh OUT LANG..." >&2
  exit 2
fi

out="$1"
shift

# Captured first, so a failing `xnl langs` fails here under `set -e`
# rather than reading as an empty list.
table="$(xnl langs)"
have="$(awk '$2 == "compiled-in" { print $1 }' <<<"$table" | LC_ALL=C sort)"
want="$(printf '%s\n' "$@" | LC_ALL=C sort -u)"

if [ "$have" != "$want" ]; then
  echo "langs: xnl langs lists a different set compiled in" >&2
  echo "  wanted:   $(tr '\n' ' ' <<<"$want")" >&2
  echo "  compiled: $(tr '\n' ' ' <<<"$have")" >&2
  exit 1
fi
touch "$out"
