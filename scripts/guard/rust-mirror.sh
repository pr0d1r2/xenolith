#!/usr/bin/env bash
#
# Every Rust module with logic has its unit tests beside it, wired in, and
# every such test file has its module (`scripts/guard:V140`, `src:C139`;
# the twin of `bats-mirror.sh` and `scripts:C13`).
#
#   <d>/<m>.rs, <d>/<m>/mod.rs  <->  <d>/<m>/tests.rs
#   <d>/lib.rs                  <->  <d>/tests.rs
#
# "Wired" means `#[cfg(test)] mod tests;` in the module. THREE failures,
# because they fail differently: a module with no mirror is visible to
# anyone who looks; an ORPHAN `tests.rs` is dead text; and an UNWIRED one
# sits exactly where it belongs and compiles nothing, so its tests pass by
# not existing -- the worst of the three, since it looks done. An inline
# `mod tests {}` does not count: the rule is about where tests are found.
#
# "Logic" is a fn with a body (`src:C139`). A file of mod declarations,
# re-exports, consts and trait signatures is outside the rule. The test is
# textual, over the file with `//` comments dropped: `fn <name>` followed,
# before any `;`, by `{`. Block comments and string literals are not
# parsed; a `fn` in either is read as code.
#
# Tracked files only, as `bats-mirror.sh`: an untracked scratch file is not
# part of the repo. Run per branch, not per commit (`scripts/guard:B1`):
# the RED commit of a mirror precedes its module.
set -euo pipefail

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "rust-mirror: not inside a git work tree -- the file list comes from git, so nothing could be checked. This is a failure, not a pass." >&2
  exit 1
fi

status=0

fail() {
  echo "rust-mirror: $1" >&2
  status=1
}

tracked() {
  git ls-files --error-unmatch "$1" >/dev/null 2>&1
}

# The file, one line, `//` comments (and so doc comments) dropped.
flat() {
  awk '{ sub(/\/\/.*/, ""); printf "%s ", $0 }' "$1"
}

# Names of every fn defined with a body, one per line.
bodied_fns() {
  flat "$1" | awk '{
    s = $0
    while (match(s, /(^|[^A-Za-z0-9_])fn[ \t]+[A-Za-z_][A-Za-z0-9_]*[^;{]*[;{]/)) {
      m = substr(s, RSTART, RLENGTH)
      s = substr(s, RSTART + RLENGTH)
      if (substr(m, length(m), 1) != "{") continue
      sub(/^[^A-Za-z0-9_]?fn[ \t]+/, "", m)
      match(m, /^[A-Za-z_][A-Za-z0-9_]*/)
      print substr(m, RSTART, RLENGTH)
    }
  }'
}

wired() {
  flat "$1" | grep -qE '#\[cfg\(test\)\][[:space:]]*mod[[:space:]]+tests[[:space:]]*;'
}

# Crate roots: every directory holding a tracked Cargo.toml ("" = the repo
# root). Their `tests/` trees are integration tests.
crates=()
while IFS= read -r manifest; do
  [ -n "$manifest" ] || continue
  case "$manifest" in
  */*) crates+=("${manifest%/Cargo.toml}/") ;;
  *) crates+=("") ;;
  esac
done < <(git ls-files 'Cargo.toml' '*/Cargo.toml')

# THE exemption list -- closed, exactly `scripts/guard:V140`'s rows. A new
# entry is a row in that spec first, with its reason; never only here.
# The two rows that are about CONTENT, not path -- a `lib.rs` of pure
# wiring and a `main.rs` shim -- are decided by `bodied_fns` below, since
# a file with no fn body is outside the rule to begin with.
exempt() {
  local path="$1" crate
  case "$path" in
  # build script, not crate code
  build.rs | */build.rs) return 0 ;;
  # FFI shim over vendored C (`languages:V121`); covered by the crate's tests/
  languages/pkl/src/grammar.rs) return 0 ;;
  # verbatim upstream
  vendor/* | */vendor/*) return 0 ;;
  esac
  # integration tests: allowed, but they do not satisfy the rule
  case "$path" in
  tests/*) return 0 ;;
  esac
  for crate in "${crates[@]+"${crates[@]}"}"; do
    case "$path" in
    "${crate}tests/"*) return 0 ;;
    esac
  done
  return 1
}

is_mirror() {
  case "$1" in
  tests.rs | */tests.rs) return 0 ;;
  esac
  return 1
}

dir_of() {
  case "$1" in
  */*) printf '%s/' "${1%/*}" ;;
  *) printf '' ;;
  esac
}

# Direction one: every module with logic has its wired mirror.
while IFS= read -r file; do
  [ -n "$file" ] || continue
  exempt "$file" && continue
  # the mirror files themselves
  is_mirror "$file" && continue

  fns="$(bodied_fns "$file")"
  # no fn body: pure wiring, outside the rule (`src:C139`)
  [ -n "$fns" ] || continue

  name="${file##*/}"
  dir="$(dir_of "$file")"
  case "$name" in
  main.rs)
    # A shim: `fn main` and nothing else. Its `mod tests;` would resolve to
    # the library's `tests.rs`, so logic here has no mirror to live in.
    if [ "$fns" != "main" ]; then
      fail "${file} is not a shim: it defines fn $(printf '%s\n' "$fns" | paste -sd ' ' -), where only fn main belongs. main.rs has no mirror of its own -- its mod tests; would be the library's -- so move the logic into the library and leave fn main calling it (src:C139)."
    fi
    continue
    ;;
  lib.rs | mod.rs) expected="${dir}tests.rs" ;;
  *) expected="${file%.rs}/tests.rs" ;;
  esac

  if ! tracked "$expected"; then
    fail "${file} has logic and no unit tests. Expected them at ${expected}, wired by #[cfg(test)] mod tests; (scripts/guard:V140, src:C139)."
  elif ! wired "$file"; then
    fail "${expected} is never compiled: ${file} does not declare #[cfg(test)] mod tests; so its tests pass by not existing (scripts/guard:V140)."
  fi
done < <(git ls-files '*.rs')

# Direction two: every mirror has the module that wires it.
while IFS= read -r test_file; do
  [ -n "$test_file" ] || continue
  exempt "$test_file" && continue

  dir="$(dir_of "$test_file")"
  owner=""
  for candidate in "${dir}lib.rs" "${dir}mod.rs" "${dir%/}.rs"; do
    [ "$candidate" != ".rs" ] || continue
    if tracked "$candidate"; then
      owner="$candidate"
      break
    fi
  done

  if [ -z "$owner" ]; then
    if [ -n "$dir" ]; then
      fail "${test_file} tests nothing: neither ${dir%/}.rs nor ${dir}mod.rs nor ${dir}lib.rs exists. An orphan test passes forever -- delete it, or restore the module it covered (scripts/guard:V140)."
    else
      fail "${test_file} tests nothing: there is no lib.rs beside it (scripts/guard:V140)."
    fi
  elif ! wired "$owner"; then
    # Direction one already reported this for a module with logic; say it
    # here only for a module without, so each defect is named once.
    if [ -z "$(bodied_fns "$owner")" ]; then
      fail "${test_file} is never compiled: ${owner} does not declare #[cfg(test)] mod tests; (scripts/guard:V140)."
    fi
  fi
done < <(git ls-files 'tests.rs' '*/tests.rs')

exit "$status"
