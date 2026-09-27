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
# `-z` lists every path as its bytes: without it git C-quotes a non-ASCII
# byte as an octal escape and a tab as `\t`, and a name after either would
# follow a word character and never match.
mapfile -d '' -t tracked < <(git ls-files -z -- . ':!:.private-names')

# An empty tracked set means there is nothing in the repo to check.
if [ "${#tracked[@]}" -eq 0 ]; then
  exit 0
fi

# An entry matches as a WHOLE word (V23): case ignored, and no word
# character -- [A-Za-z0-9_], the set `git grep -w` uses -- directly before
# or after it. A substring match found short names inside unrelated words
# (a licence's MERCHANTABILITY, any path with two letters in a row), and a
# guard that fails on every tree is one nobody runs (B3).
#
# word_in TEXT NAME -- the path half of that rule, in bash so the same
# semantics hold without leaning on which grep is on PATH. Every
# occurrence is tried: "macme/acme" matches on the second.
word_in() {
  local text="${1,,}" name="${2,,}" from=0 rest head at end
  while :; do
    rest="${text:from}"
    [[ "$rest" == *"$name"* ]] || return 1
    head="${rest%%"$name"*}"
    at=$((from + ${#head}))
    end=$((at + ${#name}))
    if { [ "$at" -eq 0 ] || [[ "${text:at-1:1}" != [A-Za-z0-9_] ]]; } &&
      [[ "${text:end:1}" != [A-Za-z0-9_] ]]; then
      return 0
    fi
    from=$((at + 1))
  done
}

# content_hits NAME -- the tracked files whose content holds NAME as a
# whole word, NUL-separated.
content_hits() {
  git grep -z -l -i -w -F -I -e "$1" -- . ':!:.private-names' 2>/dev/null || true
}

status=0
index=0
for pattern in "${patterns[@]}"; do
  line="${lines[$index]}"
  index=$((index + 1))

  # Paths first: a file NAMED after a private repo leaks it in the tree
  # listing, where no content scan would look.
  for file in "${tracked[@]}"; do
    if word_in "$file" "$pattern"; then
      echo "private-names: the path ${file} matches ${denylist} line ${line} (C17). Rename it; the pattern is not repeated here on purpose." >&2
      status=1
    fi
  done

  # Then content, over tracked files only: `git grep` never reads an
  # untracked file. `-F` keeps the entry a fixed string, `-w` whole words,
  # `-I` skips binaries and `-l` keeps the output to paths: printing the
  # matching line would print the name.
  while IFS= read -r -d '' file; do
    echo "private-names: ${file} contains the name on ${denylist} line ${line} (C17). Remove it -- anonymise the fixture or say \"a sibling repo\"; the pattern is not repeated here on purpose." >&2
    status=1
  done < <(content_hits "$pattern")
done

exit "$status"
