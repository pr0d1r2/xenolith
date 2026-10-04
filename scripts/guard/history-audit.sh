#!/usr/bin/env bash
#
# The history audit before the FIRST public push (`scripts/guard:V117`,
# `scripts/guard:V23`, `scripts/guard:C17`).
#
#   scripts/guard/history-audit.sh [--denylist FILE] [REF...]
#
# `private-names.sh` reads the tree as it is. A name committed once and
# deleted later is gone from that tree and present in every clone, so this
# reads the HISTORY: every ref name, every annotated tag message, and
# `git log -p` over the refs to be pushed -- each commit's full header and
# message, every path and every blob a diff introduced, merges included
# (`-m`, so a name only a merge resolution added is not skipped) and
# binaries read as text.
#
# Only `main` and the tags are pushed (V117), so those are the default
# refs, and any other ref given is REFUSED (exit 2): auditing a `backup/*`
# branch would suggest it may be published.
#
# The denylist is `.private-names` at the top of the work tree unless
# `--denylist` names another. Unlike the tree guard, a missing or empty
# list is a FAILURE: this runs once, before a push that cannot be taken
# back, and an audit that read no pattern has proved nothing.
#
# A hit is reported by denylist line number, count and commit id, never by
# the text matched (`history-audit.awk`). Read-only: nothing here writes to
# the repository. Exit 0 clean, 1 hit or could-not-check, 2 usage.
set -euo pipefail

usage() {
  echo "usage: history-audit.sh [--denylist FILE] [REF...]" >&2
  exit 2
}

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "history-audit: not inside a git work tree -- the history is the input," \
    "so nothing was checked. This is a failure, not a pass." >&2
  exit 1
fi

denylist="$(git rev-parse --show-toplevel)/.private-names"
wanted=()
while [ "$#" -gt 0 ]; do
  case "$1" in
  --denylist)
    [ "$#" -ge 2 ] || usage
    denylist="$2"
    shift 2
    ;;
  --denylist=*)
    denylist="${1#--denylist=}"
    shift
    ;;
  -*) usage ;;
  *)
    wanted+=("$1")
    shift
    ;;
  esac
done

if [ ! -f "$denylist" ] || ! grep -qv -e '^[[:space:]]*$' -e '^#' "$denylist"; then
  echo "history-audit: no pattern in ${denylist} -- nothing was checked. The list" \
    "is gitignored, so write it (or pass --denylist) before the first push (V117)." >&2
  exit 1
fi

# The refs V117 publishes: `main`, and the tags.
refs=()
if [ "${#wanted[@]}" -eq 0 ]; then
  if ! git show-ref --verify --quiet refs/heads/main; then
    echo "history-audit: no main branch -- V117 publishes main and the tags," \
      "so there is nothing to audit; nothing was checked." >&2
    exit 1
  fi
  refs+=(refs/heads/main)
  while IFS= read -r tag; do
    [ -n "$tag" ] && refs+=("$tag")
  done < <(git for-each-ref --format='%(refname)' refs/tags)
else
  for ref in "${wanted[@]}"; do
    case "$ref" in
    main | refs/heads/main) full=refs/heads/main ;;
    refs/tags/*) full="$ref" ;;
    *) full="refs/tags/${ref}" ;;
    esac
    if ! git show-ref --verify --quiet "$full"; then
      echo "history-audit: ${ref} is neither main nor a tag; V117 pushes only" \
        "those, so it is not audited as if it could be." >&2
      exit 2
    fi
    refs+=("$full")
  done
fi

corpus="$(mktemp)"
trap 'rm -f "$corpus"' EXIT

{
  for ref in "${refs[@]}"; do
    printf 'ref %s\n' "$ref"
  done
  for ref in "${refs[@]}"; do
    case "$ref" in refs/tags/*) ;; *) continue ;; esac
    if [ "$(git cat-file -t "$ref")" = tag ]; then
      printf 'tag %s\n' "$ref"
      git cat-file tag "$ref"
    fi
  done
  # `core.quotepath=off`: a non-ASCII path in a diff header as its bytes,
  # not octal escapes that put a digit before the name (V23 whole word).
  git -c core.quotepath=off log -p -m --root --text --no-color --no-ext-diff \
    --no-textconv --no-renames --no-mailmap --format=fuller "${refs[@]}" --
} >"$corpus"

here="${BASH_SOURCE[0]}"
case "$here" in
*/*) here="${here%/*}" ;;
*) here="." ;;
esac

# Bytes, not characters: history holds binaries read as text, and a
# multibyte locale would warn on them and fold case differently per machine.
LC_ALL=C awk -v refs="${#refs[@]}" -f "${here}/history-audit.awk" "$denylist" "$corpus"
