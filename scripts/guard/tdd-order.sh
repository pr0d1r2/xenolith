#!/usr/bin/env bash
#
# The RED commit precedes the GREEN one (`scripts/guard:C11`,
# `scripts/guard:V16`).
#
#   scripts/guard/tdd-order.sh [REV_RANGE]
#
# Default range is `@{upstream}..HEAD` -- what a push would publish -- and
# the whole history when there is no upstream yet. The ordering is a
# property of HISTORY, so no single tree can be checked for it: a script
# and its test sitting side by side prove only that both exist.
#
# Two rules, because the two languages give different evidence:
#
#   *.sh  -- the mirrored bats file (`scripts:C13`) must already exist in
#            the commit's PARENT. Added alongside the script, it was
#            written with the script in view, and the one thing nobody can
#            show is that it ever failed.
#   *.rs  -- a commit in the range, BEFORE this one, has a `test:` subject.
#            Rust tests live inside the module they cover as often as
#            beside it, so there is no mirrored path to look for; what C11
#            names is the commit type, and that is what is checked.
#            Limitation, stated rather than hidden: if the RED commit was
#            pushed in an earlier range, pass the wider range explicitly.
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "tdd-order: not inside a git work tree -- the history is the input, so nothing could be checked. This is a failure, not a pass." >&2
  exit 1
fi

if [ "$#" -gt 1 ]; then
  echo "usage: tdd-order.sh [REV_RANGE]" >&2
  exit 2
fi

range="${1:-}"
if [ -z "$range" ]; then
  if git rev-parse --verify --quiet '@{upstream}' >/dev/null 2>&1; then
    range='@{upstream}..HEAD'
  else
    range='HEAD'
  fi
fi

if ! commits="$(git rev-list --reverse "$range" 2>/dev/null)"; then
  echo "tdd-order: cannot resolve the range ${range} -- nothing was checked, which is a failure rather than a pass." >&2
  exit 1
fi

status=0
seen_test_commit=false

fail() {
  echo "tdd-order: $1" >&2
  status=1
}

for commit in $commits; do
  short="$(git rev-parse --short "$commit")"
  subject="$(git log -1 --format=%s "$commit")"

  # A `test:` commit IS the RED side. Whatever it adds -- a bats file, a
  # fixture, a Rust test module that sits beside the code rather than
  # under `tests/` -- is the evidence this guard is looking for, not the
  # implementation it gates.
  #
  # The literal `test(` is QUOTED rather than backslash-escaped: bash reads
  # both the same, but tree-sitter-bash 0.25 turns `test\(*` into an ERROR
  # node, and then `xnl check` cannot read this file at all.
  case "$subject" in
  test:* | 'test('*)
    seen_test_commit=true
    continue
    ;;
  esac

  while IFS= read -r file; do
    [ -n "$file" ] || continue
    case "$file" in
    # Tests themselves, and fixtures: these are the RED side, never the
    # GREEN side.
    tests/* | */tests/*) continue ;;
    esac

    case "$file" in
    *.sh)
      case "$file" in
      scripts/* | .github/scripts/*) ;;
      # Shell outside the allowed trees has no mirrored path at all;
      # `bats-mirror` reports that, and reporting it twice would make one
      # mistake look like two.
      *) continue ;;
      esac
      mirror="tests/unit/${file%.sh}.bats"
      if git cat-file -e "${commit}^:${mirror}" 2>/dev/null; then
        continue
      fi
      if git cat-file -e "${commit}:${mirror}" 2>/dev/null; then
        fail "${short} adds ${file} and its test ${mirror} in the SAME commit. The test commit must precede the script (C11): written together, the test is shaped by the code it was meant to constrain, and nobody can show it ever failed."
      else
        fail "${short} adds ${file} with no test at ${mirror}. Write the failing bats first, commit it, then the script (C11)."
      fi
      ;;
    *.rs)
      if [ "$seen_test_commit" = false ]; then
        fail "${short} adds ${file} with no \`test:\` commit before it in ${range}. Rust tests have no mirrored path, so C11 is checked on the commit type: commit the failing test first. If the RED commit is older than this range, pass the wider range."
      fi
      ;;
    esac
    # Renames ARE detected (git's default, >= 50% similar): a moved file,
    # even one lightly edited on the way, adds no code (`scripts/guard:B2`).
    # A file rewritten beyond that under a new name still counts as added.
  done < <(git show -M --diff-filter=A --name-only --pretty=format: "$commit")
done

exit "$status"
