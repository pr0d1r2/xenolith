#!/usr/bin/env bats
#
# Mirror of `scripts/nix/closure.sh` (scripts:C13).
#
# The script is the body of the flake's closure checks (`nix:T38`,
# `nix:V29`, `nix:V250`): given the store paths of a package's closure
# (closureInfo's `store-paths`, the sandboxed `nix path-info -r`) and a
# list of package names, fail naming every path whose name is one of
# them, and produce the check's output only when none is.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/nix/closure.sh"
  PATHS="${BATS_TEST_TMPDIR}/store-paths"
  OUT="${BATS_TEST_TMPDIR}/out"
}

# One store path per argument, as closureInfo writes them.
closure() {
  : >"$PATHS"
  for name in "$@"; do
    echo "/nix/store/0123456789abcdfghijklmnpqrsvwxyz-${name}" >>"$PATHS"
  done
}

@test "a closure holding none of the names passes and produces the output" {
  closure xenolith-0.1.0 shellcheck-0.11.0 bash-5.3p3
  run bash "$SCRIPT" "$PATHS" "$OUT" cargo hk itok
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "a name at a version is found, named, and fails the check" {
  closure xenolith-0.1.0 cargo-1.95.0
  run bash "$SCRIPT" "$PATHS" "$OUT" cargo hk
  [ "$status" -eq 1 ]
  [[ "$output" == *"-cargo-1.95.0"* ]]
  [ ! -e "$OUT" ]
}

@test "a name with no version is found too" {
  closure xenolith-0.1.0 itok
  run bash "$SCRIPT" "$PATHS" "$OUT" itok
  [ "$status" -eq 1 ]
  [[ "$output" == *"-itok"* ]]
}

@test "a split output of a named package is found" {
  closure xenolith-0.1.0 libxml2-2.14.5-bin
  run bash "$SCRIPT" "$PATHS" "$OUT" libxml2
  [ "$status" -eq 1 ]
  [[ "$output" == *"-libxml2-2.14.5-bin"* ]]
}

@test "a longer name sharing the prefix is not the named package" {
  closure xenolith-0.1.0 cargo-hack-0.6.37 hkdf-1.0 gitleaks-8.0
  run bash "$SCRIPT" "$PATHS" "$OUT" cargo hk git
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "every offender is reported, not only the first" {
  closure hk-1.2.0 xenolith-0.1.0 sherd-0.3.0
  run bash "$SCRIPT" "$PATHS" "$OUT" hk sherd
  [ "$status" -eq 1 ]
  [[ "$output" == *"-hk-1.2.0"* ]]
  [[ "$output" == *"-sherd-0.3.0"* ]]
}

@test "no names at all is a closure with nothing forbidden" {
  closure xenolith-0.1.0
  run bash "$SCRIPT" "$PATHS" "$OUT"
  [ "$status" -eq 0 ]
  [ -e "$OUT" ]
}

@test "the wrong number of arguments is a usage error" {
  run bash "$SCRIPT" "$PATHS"
  [ "$status" -eq 2 ]
  [[ "$output" == *"usage"* ]]
  [ ! -e "$OUT" ]
}

@test "a store-paths file that does not exist fails and produces nothing" {
  run bash "$SCRIPT" "${BATS_TEST_TMPDIR}/missing" "$OUT" cargo
  [ "$status" -ne 0 ]
  [ ! -e "$OUT" ]
}
