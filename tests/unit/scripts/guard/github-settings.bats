#!/usr/bin/env bats
#
# Mirror of `scripts/guard/github-settings.sh` (scripts:C13).
#
# The rule (scripts:V115): the GitHub settings the project relies on are
# stated and checked, never assumed -- `main` protected, every CI job
# required, administrators included, and Actions unable to create or
# approve pull requests while no bot needs that. The script compares what
# `gh api` reports with that list; offline, it says it checked nothing and
# stays advisory.
#
# Every test runs against a `gh` STUB on PATH that serves canned API
# replies from a fixture directory. The real GitHub API is never called:
# the gate is offline by design (`.:C3`), and so is its test.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/github-settings.sh"
  ROOT="$(cd "${BATS_TEST_DIRNAME}/../../../.." && pwd)"
  BIN="${BATS_TEST_TMPDIR}/bin"
  export FIXTURES="${BATS_TEST_TMPDIR}/api"
  export CALLS="${BATS_TEST_TMPDIR}/calls"
  mkdir -p "$BIN" "$FIXTURES"
  : >"$CALLS"
  # `gh api ENDPOINT` prints FIXTURES/<ENDPOINT, slashes as __>.json, or
  # fails the way gh does on a 404. `STUB_OFFLINE` fails every call the way
  # gh does without a network.
  cat >"${BIN}/gh" <<'EOF'
#!/usr/bin/env bash
printf '%s\n' "$*" >>"$CALLS"
if [ -n "${STUB_OFFLINE:-}" ]; then
  echo "error connecting to api.github.com" >&2
  exit 1
fi
[ "$1" = api ] || exit 64
endpoint="$2"
file="${FIXTURES}/${endpoint//\//__}.json"
if [ ! -f "$file" ]; then
  echo "gh: Not Found (HTTP 404)" >&2
  exit 1
fi
cat "$file"
EOF
  chmod +x "${BIN}/gh"
}

# reply ENDPOINT JSON -- what the stub serves for ENDPOINT.
reply() {
  printf '%s\n' "$2" >"${FIXTURES}/${1//\//__}.json"
}

REPO_API="repos/pr0d1r2/xenolith"
CHECKS='"gate (ubuntu-latest)","gate (ubuntu-24.04-arm)","gate (macos-latest)"'
# Pull requests land by rebase merge only (scripts:V115, scripts/guard:B4).
MERGE_ONLY_REBASE='{"full_name":"pr0d1r2/xenolith","allow_rebase_merge":true,"allow_squash_merge":false,"allow_merge_commit":false}'

# Every intended setting in place.
intended() {
  reply "$REPO_API" "$MERGE_ONLY_REBASE"
  reply "${REPO_API}/branches/main/protection" \
    "{\"enforce_admins\":{\"enabled\":true},\"required_status_checks\":{\"contexts\":[${CHECKS}]}}"
  reply "${REPO_API}/actions/permissions/workflow" \
    '{"default_workflow_permissions":"read","can_approve_pull_request_reviews":false}'
}

run_script() {
  PATH="${BIN}:${PATH}" run "$BASH" "$SCRIPT" "$@"
}

@test "the intended settings pass silently" {
  intended
  run_script
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "OWNER/REPO selects the repository asked about" {
  intended
  reply repos/someone/fork '{"full_name":"someone/fork"}'
  run_script someone/fork
  grep -q '^api repos/someone/fork$' "$CALLS"
}

@test "an unprotected main fails" {
  intended
  rm "${FIXTURES}/${REPO_API//\//__}__branches__main__protection.json"
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"main"*"not protected"* ]]
  [[ "$output" == *"scripts:V115"* ]]
}

@test "administrators left out of the protection fails" {
  intended
  reply "${REPO_API}/branches/main/protection" \
    "{\"enforce_admins\":{\"enabled\":false},\"required_status_checks\":{\"contexts\":[${CHECKS}]}}"
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"administrators"* ]]
}

@test "a CI job that is not required fails, naming it" {
  intended
  local one='"contexts":["gate (ubuntu-latest)"]'
  reply "${REPO_API}/branches/main/protection" \
    "{\"enforce_admins\":{\"enabled\":true},\"required_status_checks\":{${one}}}"
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"gate (macos-latest)"* ]]
  [[ "$output" == *"gate (ubuntu-24.04-arm)"* ]]
  [[ "$output" != *"\`gate (ubuntu-latest)\`"* ]]
}

@test "required checks listed in the newer checks array count" {
  intended
  reply "${REPO_API}/branches/main/protection" \
    "{\"enforce_admins\":{\"enabled\":true},\"required_status_checks\":{\"checks\":[
      {\"context\":\"gate (ubuntu-latest)\"},{\"context\":\"gate (ubuntu-24.04-arm)\"},
      {\"context\":\"gate (macos-latest)\"}]}}"
  run_script
  [ "$status" -eq 0 ]
}

@test "no required status checks at all fails" {
  intended
  reply "${REPO_API}/branches/main/protection" '{"enforce_admins":{"enabled":true}}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"gate (ubuntu-latest)"* ]]
}

@test "Actions allowed to create and approve pull requests fails" {
  intended
  reply "${REPO_API}/actions/permissions/workflow" \
    '{"default_workflow_permissions":"read","can_approve_pull_request_reviews":true}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"pull requests"* ]]
}

@test "an unreadable Actions setting is a finding, not a pass" {
  intended
  rm "${FIXTURES}/${REPO_API//\//__}__actions__permissions__workflow.json"
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"actions/permissions/workflow"* ]]
}

@test "every mismatch is reported, not only the first" {
  intended
  reply "${REPO_API}/branches/main/protection" \
    '{"enforce_admins":{"enabled":false},"required_status_checks":{"contexts":[]}}'
  reply "${REPO_API}/actions/permissions/workflow" '{"can_approve_pull_request_reviews":true}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"administrators"* ]]
  [[ "$output" == *"gate (macos-latest)"* ]]
  [[ "$output" == *"pull requests"* ]]
}

@test "offline it checks nothing, says so and stays advisory" {
  intended
  STUB_OFFLINE=1 run_script
  [ "$status" -eq 0 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "a repository GitHub does not know is advisory too" {
  run_script
  [ "$status" -eq 0 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "without gh it checks nothing, says so and stays advisory" {
  intended
  rm "${BIN}/gh"
  ln -s "$(command -v jq)" "${BIN}/jq"
  PATH="${BIN}" run "$BASH" "$SCRIPT"
  [ "$status" -eq 0 ]
  [[ "$output" == *"gh"* ]]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "it only ever reads: no method, field or input flag reaches gh" {
  intended
  run_script
  [ -s "$CALLS" ]
  run grep -qE -- '(^| )(-X|--method|-f|-F|--field|--raw-field|--input)( |=|$)' "$CALLS"
  [ "$status" -eq 1 ]
}

@test "--print states the intended settings without calling gh" {
  run_script --print
  [ "$status" -eq 0 ]
  [ ! -s "$CALLS" ]
  [[ "$output" == *"main"* ]]
  [[ "$output" == *"administrators"* ]]
  [[ "$output" == *"pull requests"* ]]
}

@test "the required checks are exactly the CI workflow's gate jobs" {
  run_script --print
  local matrix os
  matrix="$(grep -E '^[[:space:]]+os: \[' "${ROOT}/.github/workflows/ci.yml")"
  matrix="${matrix#*[}"
  matrix="${matrix%]*}"
  IFS=', ' read -r -a oses <<<"$matrix"
  [ "${#oses[@]}" -gt 0 ]
  for os in "${oses[@]}"; do
    [[ "$output" == *"gate (${os})"* ]]
  done
  [ "$(grep -o 'gate (' <<<"$output" | wc -l)" -eq "${#oses[@]}" ]
}

@test "an unknown flag is a usage error" {
  run_script --frobnicate
  [ "$status" -eq 2 ]
}

@test "squash merges allowed fails: a squash collapses RED and GREEN" {
  intended
  reply "$REPO_API" '{"allow_rebase_merge":true,"allow_squash_merge":true,"allow_merge_commit":false}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"squash"* ]]
  [[ "$output" == *"scripts:V115"* ]]
}

@test "merge commits allowed fails" {
  intended
  reply "$REPO_API" '{"allow_rebase_merge":true,"allow_squash_merge":false,"allow_merge_commit":true}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"merge commit"* ]]
}

@test "rebase merges not allowed fails" {
  intended
  reply "$REPO_API" '{"allow_rebase_merge":false,"allow_squash_merge":false,"allow_merge_commit":false}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"rebase"* ]]
}

@test "a repository reply without the merge settings is a finding, not a pass" {
  intended
  reply "$REPO_API" '{"full_name":"pr0d1r2/xenolith"}'
  run_script
  [ "$status" -eq 1 ]
  [[ "$output" == *"rebase"* ]]
}

@test "--print states that pull requests land by rebase merge only" {
  run_script --print
  [ "$status" -eq 0 ]
  [[ "$output" == *"rebase merge only"* ]]
}
