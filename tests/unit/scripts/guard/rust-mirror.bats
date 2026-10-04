#!/usr/bin/env bats
#
# Mirror of `scripts/guard/rust-mirror.sh` (scripts:C13).
#
# The rule (scripts/guard:V140, src:C139): every tracked `.rs` with logic
# -- a fn with a body -- has a sibling `tests.rs` wired by
# `#[cfg(test)] mod tests;`, and every `tests.rs` has the module that
# wires it. Both directions, and a third failure besides: a `tests.rs`
# that sits in the right place but is never declared compiles nothing,
# so its tests pass by not existing.
#
#   <d>/<m>.rs, <d>/<m>/mod.rs  <->  <d>/<m>/tests.rs
#   <d>/lib.rs                  <->  <d>/tests.rs

setup() {
  # tests:V150: a hook exports GIT_DIR and friends, and they beat `git -C`
  # (tests:B1). Drop every GIT_* variable, keep git off the user's and the
  # system's config, and stop repository discovery at the test's own tmpdir.
  unset "${!GIT_@}"
  export GIT_CONFIG_GLOBAL="${BATS_TEST_TMPDIR}/gitconfig" GIT_CONFIG_NOSYSTEM=1
  export GIT_CEILING_DIRECTORIES="$BATS_TEST_TMPDIR"
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/rust-mirror.sh"
  REPO="${BATS_TEST_TMPDIR}/repo"
  mkdir -p "$REPO"
  git -C "$REPO" init --quiet
}

# Tracked files only, with the content given (a fn with a body by
# default, which is what puts a file under the rule).
track() {
  local path="$1"
  local body="${2:-pub fn f() -> u8 {
    1
}
}"
  mkdir -p "${REPO}/$(dirname "$path")"
  printf '%s\n' "$body" >"${REPO}/${path}"
  git -C "$REPO" add "$path"
}

WIRED='pub fn f() -> u8 {
    1
}

#[cfg(test)]
mod tests;'

run_guard() {
  cd "$REPO" || return 1
  run "$BASH" "$SCRIPT"
}

@test "an empty repo passes" {
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "<m>.rs with a wired <m>/tests.rs passes" {
  track src/model.rs "$WIRED"
  track src/model/tests.rs
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "<m>/mod.rs with a wired <m>/tests.rs passes" {
  track src/config/mod.rs "$WIRED"
  track src/config/tests.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "lib.rs with a wired sibling tests.rs passes" {
  track languages/ci/nix/src/lib.rs "$WIRED"
  track languages/ci/nix/src/tests.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "<m>.rs with logic and no tests.rs fails and names the expected path" {
  track src/model.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/model.rs"* ]]
  [[ "$output" == *"src/model/tests.rs"* ]]
}

@test "<m>/mod.rs with logic and no tests.rs names <m>/tests.rs" {
  track src/cli/mod.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/cli/tests.rs"* ]]
}

@test "lib.rs with logic and no tests.rs names the sibling tests.rs" {
  track languages/ci/nix/src/lib.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"languages/ci/nix/src/tests.rs"* ]]
}

@test "a tests.rs its module never declares fails as unwired" {
  track src/model.rs
  track src/model/tests.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/model/tests.rs"* ]]
  [[ "$output" == *"#[cfg(test)] mod tests;"* ]]
}

@test "mod tests; without #[cfg(test)] is not wired" {
  track src/model.rs 'pub fn f() {}
mod tests;'
  track src/model/tests.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/model/tests.rs"* ]]
}

@test "an inline mod tests {} does not count" {
  track src/model.rs 'pub fn f() {}
#[cfg(test)]
mod tests {
    #[test]
    fn t() {}
}'
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/model/tests.rs"* ]]
}

@test "an orphan tests.rs with no module fails and names the module it expected" {
  track src/gone/tests.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/gone/tests.rs"* ]]
  [[ "$output" == *"src/gone.rs"* ]]
}

@test "a file with no fn body is outside the rule" {
  track languages/api/src/site/mod.rs 'pub trait Site {
    fn name(&self) -> &str;
}

pub struct S;'
  run_guard
  [ "$status" -eq 0 ]
}

@test "a lib.rs of pure wiring is outside the rule" {
  track src/lib.rs '//! the crate.
//! pub fn in_a_doc_comment() { is not a body }
pub mod cli;
pub use cli::main;
pub const VERSION: &str = "1";'
  run_guard
  [ "$status" -eq 0 ]
}

@test "a fn whose signature spans lines still has a body" {
  track src/wide.rs 'pub fn wide<T>(
    t: T,
) -> T
where
    T: Clone,
{
    t
}'
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/wide/tests.rs"* ]]
}

@test "a main.rs shim (fn main only) is exempt" {
  track src/main.rs 'fn main() -> std::process::ExitCode {
    xenolith::cli::main()
}'
  run_guard
  [ "$status" -eq 0 ]
}

@test "a main.rs with logic beyond fn main fails: it has no mirror of its own" {
  track src/main.rs 'fn main() {
    run();
}

fn run() {}'
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/main.rs"* ]]
  [[ "$output" == *"shim"* ]]
}

@test "build.rs is exempt at any depth" {
  track build.rs
  track languages/ci/pkl/build.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "the pkl grammar FFI shim is exempt" {
  track languages/ci/pkl/src/grammar.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "root tests/ integration files are exempt" {
  track Cargo.toml '[package]'
  track tests/skeleton.rs
  track tests/support/tests.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "a crate's tests/ integration files are exempt" {
  track languages/ci/pkl/Cargo.toml '[package]'
  track languages/ci/pkl/tests/host.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "a tests/ directory that is not a crate's is not exempt" {
  track src/cli/tests/helper.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/cli/tests/helper/tests.rs"* ]]
}

@test "vendored code is exempt at any depth" {
  track vendor/upstream/src/parser.rs
  track languages/ci/pkl/vendor/x.rs
  run_guard
  [ "$status" -eq 0 ]
}

@test "an untracked file is not the guard's business" {
  mkdir -p "${REPO}/src"
  printf 'pub fn f() {}\n' >"${REPO}/src/scratch.rs"
  run_guard
  [ "$status" -eq 0 ]
}

@test "reports every violation, not only the first" {
  track src/a.rs
  track src/b.rs
  track src/c/tests.rs
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"src/a/tests.rs"* ]]
  [[ "$output" == *"src/b/tests.rs"* ]]
  [[ "$output" == *"src/c/tests.rs"* ]]
}

@test "outside a git repository it fails rather than passing vacuously" {
  REPO="${BATS_TEST_TMPDIR}/nogit"
  mkdir -p "$REPO"
  run_guard
  [ "$status" -ne 0 ]
  [[ "$output" == *"git"* ]]
}

# tests:V150, tests:B1. git exports GIT_DIR and GIT_INDEX_FILE to every hook,
# and hk runs this suite from pre-commit and pre-push. The environment beats
# `git -C`, so unless setup drops it, every fixture write above lands in the
# repository the hook is running for. Staged here over a sentinel repo: setup
# runs again under a hook-shaped environment, the fixture does its usual git
# work, and the sentinel must come out exactly as it went in.
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
  setup
  track src/model.rs "$WIRED"
  track src/model/tests.rs
  run_guard
  [ "$status" -eq 0 ]
  sentinel_untouched
}
