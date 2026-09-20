#!/usr/bin/env bats
#
# Mirror of `scripts/guard/commit-msg.sh` (scripts:C13).
#
# Two rules, one checker: Conventional Commits for the subject, and a
# `Why:` line in the body (scripts/guard:V20, scripts/guard:C12). The body
# rule is the one that matters -- the subject says WHAT changed, and the
# diff already says that; `Why:` is the only part of the audit trail that
# cannot be reconstructed from the tree.
#
# The script takes the path git gives a commit-msg hook, so it stays free
# of `xnl`: it must work before the tool it gates has been written.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/commit-msg.sh"
  MSG="${BATS_TEST_TMPDIR}/COMMIT_EDITMSG"
}

check_msg() {
  printf '%s\n' "$1" >"$MSG"
  run "$BASH" "$SCRIPT" "$MSG"
}

@test "accepts a conventional subject with a Why: line" {
  check_msg "feat(scripts): add the commit-msg guard

Why: an unexplained commit is a decision nobody can review later."
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "accepts every conventional type, scope optional" {
  for subject in "fix: repair" "docs: write" "test: cover" "refactor: move" \
    "perf: speed up" "build: package" "ci: wire" "chore: tidy" \
    "style: reformat" "revert: undo" "feat(nix/devshell): pin"; do
    check_msg "${subject}

Why: stated."
    [ "$status" -eq 0 ]
  done
}

@test "accepts a breaking-change marker" {
  check_msg "feat(src)!: change the config shape

Why: stated."
  [ "$status" -eq 0 ]
}

@test "rejects a subject that is not Conventional Commits" {
  check_msg "added a guard

Why: stated."
  [ "$status" -ne 0 ]
  [[ "$output" == *"Conventional Commits"* ]]
}

@test "rejects an unknown type" {
  check_msg "improvement: tidy up

Why: stated."
  [ "$status" -ne 0 ]
}

@test "rejects an empty subject after the type" {
  check_msg "feat:

Why: stated."
  [ "$status" -ne 0 ]
}

@test "rejects a message with no Why: line" {
  check_msg "feat(scripts): add the commit-msg guard

It checks the subject and the body."
  [ "$status" -ne 0 ]
  [[ "$output" == *"Why:"* ]]
}

@test "rejects a subject-only message" {
  check_msg "feat(scripts): add the commit-msg guard"
  [ "$status" -ne 0 ]
  [[ "$output" == *"Why:"* ]]
}

@test "a Why: inside a comment line does not count" {
  check_msg "feat(scripts): add the commit-msg guard

# Why: git strips this line before the commit is written."
  [ "$status" -ne 0 ]
}

@test "a Why: in the subject line does not count as the body's" {
  check_msg "feat(scripts): Why: this is a subject, not a reason"
  [ "$status" -ne 0 ]
}

@test "passes a merge commit through untouched" {
  check_msg "Merge branch 'main' into topic"
  [ "$status" -eq 0 ]
}

@test "passes a fixup commit through, since rebase rewrites it" {
  check_msg "fixup! feat(scripts): add the commit-msg guard"
  [ "$status" -eq 0 ]
}

@test "no path given is a usage error" {
  run "$BASH" "$SCRIPT"
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
}

@test "a path that does not exist fails rather than passing vacuously" {
  run "$BASH" "$SCRIPT" "${BATS_TEST_TMPDIR}/absent"
  [ "$status" -ne 0 ]
  [ "$status" -ne 2 ]
}
