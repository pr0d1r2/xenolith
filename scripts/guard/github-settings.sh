#!/usr/bin/env bash
#
# The GitHub settings this project relies on, stated and checked rather
# than assumed (`scripts:V115`).
#
#   scripts/guard/github-settings.sh [OWNER/REPO]   compare (default below)
#   scripts/guard/github-settings.sh --print        the runbook, no network
#
# THE RUNBOOK -- what the repository must say, and where to set it:
#
#   1. `main` is protected: Settings -> Branches -> a rule for `main`.
#   2. Every CI job is a REQUIRED status check on it: one per matrix entry
#      of `.github/workflows/ci.yml`, named `gate (<os>)`. A job that runs
#      but is not required is a job a merge may ignore.
#   3. Administrators are included (`enforce_admins`): a rule the owner
#      can bypass without noticing is a rule for everyone else.
#   4. GitHub Actions may NOT create or approve pull requests (Settings ->
#      Actions -> General -> Workflow permissions). No bot here needs it;
#      the day one does, this line changes with the reason (V115).
#
# Read-only: every call is a plain `gh api GET`. The comparison itself is
# `github-settings.jq` beside this file.
#
# ADVISORY OFFLINE: the gate never reaches the network (`.:C3`), so this is
# not an hk step. Without `gh`, without a network or credentials, or before
# the repository exists (`.:T116`), it says that nothing was checked and
# exits 0. Once the repository answers, a setting that differs -- or one
# that cannot be read -- is a finding and exits 1. Usage errors exit 2.
set -euo pipefail

repo="pr0d1r2/xenolith"
required=(
  "gate (ubuntu-latest)"
  "gate (ubuntu-24.04-arm)"
  "gate (macos-latest)"
)

print_runbook() {
  echo "Intended GitHub settings for ${repo} (scripts:V115):"
  echo "  main: protected by a branch rule"
  local check
  for check in "${required[@]}"; do
    echo "  main: status check \`${check}\` required"
  done
  echo "  main: rule includes administrators (enforce_admins = true)"
  echo "  Actions: may not create or approve pull requests" \
    "(can_approve_pull_request_reviews = false)"
}

case "${1:-}" in
--print)
  [ "$#" -eq 1 ] || {
    echo "usage: github-settings.sh [--print | OWNER/REPO]" >&2
    exit 2
  }
  print_runbook
  exit 0
  ;;
-*)
  echo "usage: github-settings.sh [--print | OWNER/REPO]" >&2
  exit 2
  ;;
?*/?*)
  repo="$1"
  ;;
'') ;;
*)
  echo "usage: github-settings.sh [--print | OWNER/REPO]" >&2
  exit 2
  ;;
esac

if ! command -v gh >/dev/null 2>&1; then
  echo "github-settings: gh is not on PATH -- nothing was checked." \
    "Advisory: run it where gh is installed and logged in (scripts:V115)." >&2
  exit 0
fi
if ! command -v jq >/dev/null 2>&1; then
  echo "github-settings: jq is not on PATH -- re-enter the dev shell;" \
    "nothing was checked." >&2
  exit 1
fi

if ! gh api "repos/${repo}" >/dev/null 2>&1; then
  echo "github-settings: ${repo} did not answer (offline, not logged in, or" \
    "not created yet) -- nothing was checked. Advisory (scripts:V115)." >&2
  exit 0
fi

here="${BASH_SOURCE[0]}"
case "$here" in
*/*) here="${here%/*}" ;;
*) here="." ;;
esac
rule="${here}/github-settings.jq"

status=0

# compare SECTION ENDPOINT -- one reply against the intended settings.
compare() {
  local section="$1" endpoint="$2" reply findings
  if ! reply="$(gh api "$endpoint" 2>/dev/null)"; then
    return 1
  fi
  # Checked explicitly: a caller's `if !` switches `set -e` off in here,
  # and a reply jq cannot read must not read as a match.
  if ! findings="$(printf '%s' "$reply" |
    jq -r --arg section "$section" -f "$rule" --args "${required[@]}")"; then
    echo "github-settings: the reply from ${endpoint} could not be read --" \
      "not confirmed, which is not a pass (scripts:V115)." >&2
    status=1
    return 0
  fi
  if [ -n "$findings" ]; then
    printf '%s\n' "$findings" >&2
    status=1
  fi
}

if ! compare protection "repos/${repo}/branches/main/protection"; then
  echo "github-settings: main is not protected -- no branch rule answered" \
    "at repos/${repo}/branches/main/protection (scripts:V115)." >&2
  status=1
fi
if ! compare actions "repos/${repo}/actions/permissions/workflow"; then
  echo "github-settings: could not read repos/${repo}/actions/permissions/workflow" \
    "-- the Actions setting is not confirmed, which is not a pass (scripts:V115)." >&2
  status=1
fi

exit "$status"
