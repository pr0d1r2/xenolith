#!/usr/bin/env bats
#
# Mirror of `scripts/guard/history-audit.sh` (scripts:C13).
#
# The rule (scripts/guard:V117, scripts/guard:V23): before the FIRST public
# push, every commit message and every blob in every ref to be pushed is
# scanned against the private denylist. Only `main` and the tags are
# pushed, so only those are read -- and any other ref is refused rather
# than audited, because auditing it would suggest it may be published.
#
# The tree guard (`private-names.sh`) reads the tree as it is; a name that
# was committed and later deleted is still in every clone. That is what
# this audit is for, so most tests here put the name somewhere the current
# tree no longer shows it.

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat `git -C`
  # (tests:B1). Drop every GIT_* variable, keep git off the user's and the
  # system's config, and stop repository discovery at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/history-audit.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet -b main
  git -C "$REPO" config user.email t@example.com
  git -C "$REPO" config user.name test
  printf 'acmeinternal\n' >"${REPO}/.private-names"
  commit "chore: seed" notes.txt "nothing to see"
}

# commit MESSAGE PATH CONTENT -- write PATH and commit it.
commit() {
  local message="$1" path="$2" content="$3"
  mkdir -p "${REPO}/$(dirname "$path")"
  printf '%s\n' "$content" >"${REPO}/${path}"
  git -C "$REPO" add "$path"
  git -C "$REPO" commit --quiet --no-verify -m "$message"
}

# remove PATH -- delete it in a commit, so only history holds it.
remove() {
  git -C "$REPO" rm --quiet "$1"
  git -C "$REPO" commit --quiet --no-verify -m "chore: drop $1"
}

short() {
  git -C "$REPO" rev-parse --short=12 "$1"
}

# Everything a write could change: refs, objects, index and worktree.
repo_state() {
  git -C "$REPO" for-each-ref
  git -C "$REPO" count-objects -v
  git -C "$REPO" status --porcelain
}

run_audit() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT" "$@"
}

@test "a clean history passes and says what it read" {
  commit "docs: more" docs/a.md "plain words"
  run_audit
  [ "$status" -eq 0 ]
  [[ "$output" == *"2 commits"* ]]
  [[ "$output" == *"1 pattern"* ]]
  [[ "$output" == *"0 hits"* ]]
}

@test "a name in a commit message fails, citing the line and the commit" {
  commit "docs: ported from acmeinternal" docs/a.md "plain words"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"line 1"* ]]
  [[ "$output" == *"$(short HEAD)"* ]]
}

@test "the report never repeats the name it matched" {
  commit "docs: ported from acmeinternal" docs/a.md "acmeinternal again"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" != *"acmeinternal"* ]]
}

@test "a name in a blob deleted since is still found" {
  commit "docs: notes" docs/a.md "copied from acmeinternal"
  local added
  added="$(short HEAD)"
  remove docs/a.md
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"$added"* ]]
}

@test "a name in a path is found" {
  commit "docs: notes" docs/acmeinternal-port.md "plain words"
  remove docs/acmeinternal-port.md
  run_audit
  [ "$status" -eq 1 ]
}

@test "a name in a binary blob is found" {
  printf 'x\000acmeinternal\000y' >"${REPO}/blob.bin"
  git -C "$REPO" add blob.bin
  git -C "$REPO" commit --quiet --no-verify -m "chore: a binary"
  remove blob.bin
  run_audit
  [ "$status" -eq 1 ]
}

@test "a name only a merge introduced is found" {
  git -C "$REPO" checkout --quiet -b side
  commit "docs: side" side.txt "side"
  git -C "$REPO" checkout --quiet main
  commit "docs: main" main.txt "main"
  git -C "$REPO" merge --quiet --no-commit --no-ff side >/dev/null 2>&1 || true
  printf 'resolved against acmeinternal\n' >"${REPO}/merge-only.txt"
  git -C "$REPO" add merge-only.txt
  git -C "$REPO" commit --quiet --no-verify -m "Merge branch side"
  git -C "$REPO" branch --quiet -D side
  run_audit
  [ "$status" -eq 1 ]
}

@test "a name in the author is found" {
  printf 'plain\n' >"${REPO}/b.txt"
  git -C "$REPO" add b.txt
  git -C "$REPO" -c user.name="acmeinternal bot" commit --quiet --no-verify -m "chore: b"
  run_audit
  [ "$status" -eq 1 ]
}

@test "matching ignores case" {
  commit "docs: from AcmeInternal" docs/a.md "plain"
  run_audit
  [ "$status" -eq 1 ]
}

@test "a name in an annotated tag message is found" {
  git -C "$REPO" tag -a v0.1.0 -m "cut for acmeinternal"
  run_audit
  [ "$status" -eq 1 ]
}

@test "a name in a tag's own name is found" {
  git -C "$REPO" tag acmeinternal-snapshot
  run_audit
  [ "$status" -eq 1 ]
}

@test "commits reachable from a tag only are read by default" {
  git -C "$REPO" checkout --quiet -b release
  commit "docs: release note for acmeinternal" rel.txt "plain"
  git -C "$REPO" tag v0.2.0
  git -C "$REPO" checkout --quiet main
  git -C "$REPO" branch --quiet -D release
  run_audit
  [ "$status" -eq 1 ]
}

@test "a branch that is never pushed is not read" {
  git -C "$REPO" checkout --quiet -b backup/old
  commit "docs: acmeinternal scratch" scratch.txt "acmeinternal"
  git -C "$REPO" checkout --quiet main
  run_audit
  [ "$status" -eq 0 ]
}

@test "asking to audit a ref that is neither main nor a tag is refused" {
  git -C "$REPO" branch backup/old
  run_audit backup/old
  [ "$status" -eq 2 ]
  [[ "$output" == *"V117"* ]]
}

@test "main and a named tag may be given explicitly" {
  git -C "$REPO" tag v1.0.0
  run_audit main v1.0.0
  [ "$status" -eq 0 ]
}

@test "every pattern is reported with its own line and count" {
  printf '# private\nacmeinternal\notherco\n' >"${REPO}/.private-names"
  commit "docs: acmeinternal" a.txt "otherco"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"line 2"* ]]
  [[ "$output" == *"line 3"* ]]
  [[ "$output" != *"line 1:"* ]]
}

@test "--denylist names another list" {
  rm "${REPO}/.private-names"
  printf 'otherco\n' >"${BATS_TEST_TMPDIR}/names"
  commit "docs: otherco" a.txt "plain"
  run_audit --denylist "${BATS_TEST_TMPDIR}/names"
  [ "$status" -eq 1 ]
  [[ "$output" != *"otherco"* ]]
}

@test "no denylist fails: an audit without a list proves nothing" {
  rm "${REPO}/.private-names"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "a denylist of comments only fails the same way" {
  printf '# nothing\n\n' >"${REPO}/.private-names"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "a repository with no main fails rather than auditing nothing" {
  git -C "$REPO" branch --quiet -m main trunk
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"main"* ]]
}

@test "outside a git work tree it fails rather than passing vacuously" {
  REPO="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$REPO"
  printf 'acmeinternal\n' >"${REPO}/.private-names"
  run_audit
  [ "$status" -eq 1 ]
  [[ "$output" == *"git"* ]]
}

@test "the audit writes nothing: refs, index and objects are untouched" {
  commit "docs: acmeinternal" a.txt "plain"
  local before after
  before="$(repo_state)"
  run_audit
  after="$(repo_state)"
  [ "$before" = "$after" ]
}

# tests:V150, tests:B1. git exports GIT_DIR and GIT_INDEX_FILE to every hook.
# The environment beats `git -C`, so unless setup drops it, every fixture
# write above lands in the repository the hook is running for. Staged here
# over a sentinel repo: setup runs again under a hook-shaped environment,
# the fixture does its usual git work, and the sentinel must come out
# exactly as it went in.
hook_env_over_sentinel() {
  SENTINEL="${BATS_TEST_TMPDIR}/sentinel"
  git init --quiet "$SENTINEL"
  SENTINEL_CONFIG="$(cat "${SENTINEL}/.git/config")"
  export GIT_DIR="${SENTINEL}/.git" GIT_INDEX_FILE="${SENTINEL}/.git/index"
}

sentinel_untouched() {
  [ "$(cat "${SENTINEL}/.git/config")" = "$SENTINEL_CONFIG" ]
  [ ! -e "${SENTINEL}/.git/index" ]
  [ -z "$(find "${SENTINEL}/.git/objects" -type f)" ]
}

@test "a git hook's environment does not reach the enclosing repo" {
  hook_env_over_sentinel
  rm -rf "${BATS_TEST_TMPDIR}/repo"
  setup
  commit "docs: more" docs/a.md "plain"
  run_audit
  [ "$status" -eq 0 ]
  sentinel_untouched
}
