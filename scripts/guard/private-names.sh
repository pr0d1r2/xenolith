#!/usr/bin/env bash
#
# No private repo name in a tracked file (`scripts/guard:V23`,
# `scripts/guard:C17`). Unknown means private: the public set is small and
# named in C17, and everything else is assumed not to be.
#
# The denylist is `.private-names`, one pattern per line, and it is
# GITIGNORED -- publishing the list would publish exactly what it protects.
# That has two consequences, both deliberate:
#
#   1. A fresh clone has no list, so the guard says it could not run and
#      exits 0. Failing there would be a gate every contributor learns to
#      skip, and the history audit before the first public push
#      (`scripts/guard:V117`) is the backstop that does not depend on a
#      local file.
#   2. A finding cites the LINE NUMBER in the denylist, never the name it
#      matched. A guard that prints the private name into every CI log has
#      leaked it while reporting that it prevented a leak.
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "private-names: not inside a git work tree -- the file list comes from git, so nothing could be checked. This is a failure, not a pass." >&2
  exit 1
fi

denylist=".private-names"

if [ ! -s "$denylist" ]; then
  echo "private-names: no patterns in ${denylist} -- NOTHING was checked. The list is gitignored by design, so a fresh clone starts empty; write one before trusting this step (V23)." >&2
  exit 0
fi

# The denylist is read into an array so a finding can cite its line number
# instead of its content.
patterns=()
lines=()
lineno=0
while IFS= read -r raw || [ -n "$raw" ]; do
  lineno=$((lineno + 1))
  pattern="${raw%%$'\r'}"
  case "$pattern" in
  '' | '#'*) continue ;;
  esac
  patterns+=("$pattern")
  lines+=("$lineno")
done <"$denylist"

if [ "${#patterns[@]}" -eq 0 ]; then
  echo "private-names: ${denylist} holds only comments -- NOTHING was checked (V23)." >&2
  exit 0
fi

# The denylist itself is excluded: it is untracked, so `git ls-files` would
# not return it anyway, but saying so here keeps the exclusion a decision
# rather than an accident of another tool's behaviour.
mapfile -t tracked < <(git ls-files -- . ':!:.private-names')

# With no file arguments `grep -r` falls back to the working directory,
# which would scan untracked scratch files and the denylist itself -- the
# two things this guard must not read. An empty tracked set means there is
# nothing in the repo to check.
if [ "${#tracked[@]}" -eq 0 ]; then
  exit 0
fi

status=0
index=0
for pattern in "${patterns[@]}"; do
  line="${lines[$index]}"
  index=$((index + 1))

  # Paths first: a file NAMED after a private repo leaks it in the tree
  # listing, where no content scan would look.
  for file in "${tracked[@]}"; do
    if printf '%s' "$file" | grep -qiF -- "$pattern"; then
      echo "private-names: the path ${file} matches ${denylist} line ${line} (C17). Rename it; the pattern is not repeated here on purpose." >&2
      status=1
    fi
  done

  # Then content, over the tracked list only -- never `-r`, which would
  # walk the working directory. `-I` skips binaries and `-l` keeps the
  # output to paths: printing the matching line would print the name.
  while IFS= read -r file; do
    [ -n "$file" ] || continue
    echo "private-names: ${file} contains the name on ${denylist} line ${line} (C17). Remove it -- anonymise the fixture or say \"a sibling repo\"; the pattern is not repeated here on purpose." >&2
    status=1
  done < <(grep -liIF -- "$pattern" "${tracked[@]}" 2>/dev/null || true)
done

exit "$status"
