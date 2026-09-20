#!/usr/bin/env bash
#
# The commit-msg gate: Conventional Commits in the subject, a `Why:` line
# in the body (`scripts/guard:V20`, `scripts/guard:C12`).
#
# Deliberately `xnl`-free. This repo's own tool does not exist yet, and a
# guard that cannot run until the thing it guards is built is a guard that
# is added late -- which is to say, after the history it was meant to
# shape.
#
# The subject rule is the cheap half. The BODY rule is the point: a diff
# already says what changed, and `Why:` is the only part of the record
# that cannot be reconstructed from the tree.
#
#   scripts/guard/commit-msg.sh "$(git rev-parse --git-path COMMIT_EDITMSG)"
set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: commit-msg.sh COMMIT_MSG_FILE" >&2
  exit 2
fi

msg_file="$1"

# Not a usage error: the hook was wired and the file it named is gone, so
# NOTHING was checked. Reporting success here would be the worst outcome
# available -- a green gate over an unread message.
if [ ! -f "$msg_file" ]; then
  echo "commit-msg: no message file at ${msg_file} -- nothing was checked, so this is a failure rather than a pass." >&2
  exit 1
fi

# Comment lines are stripped by git before the commit is written, so a
# `Why:` in one would satisfy a naive grep and vanish from the history.
# Strip them here, exactly as git will.
body_and_subject="$(grep -v '^#' "$msg_file" || true)"
subject="$(printf '%s\n' "$body_and_subject" | sed -n '1p')"
body="$(printf '%s\n' "$body_and_subject" | sed -n '2,$p')"

# git writes merge subjects itself, and `fixup!`/`squash!` are rewritten by
# the rebase that consumes them. Failing either blocks work without
# improving any audit trail.
case "$subject" in
Merge\ * | Revert\ \"* | fixup!\ * | squash!\ * | amend!\ *)
  exit 0
  ;;
esac

types='feat|fix|docs|test|refactor|perf|build|ci|chore|style|revert'
if ! printf '%s\n' "$subject" | grep -qE "^(${types})(\([a-z0-9/._-]+\))?!?: .+"; then
  echo "commit-msg: the subject is not Conventional Commits. Expected \`type(scope): summary\`, where type is one of: ${types//|/, }. Got: ${subject}" >&2
  exit 1
fi

# The subject is excluded from the search on purpose: `feat: Why: ...` is a
# summary that happens to contain the word, not a reason for the change.
if ! printf '%s\n' "$body" | grep -qE '^Why:[[:space:]]*[^[:space:]]'; then
  echo "commit-msg: the body has no \`Why:\` line. The diff already says what changed; state why it changed, and cite the spec id it serves (scripts/guard:V20, scripts/guard:C12)." >&2
  exit 1
fi
