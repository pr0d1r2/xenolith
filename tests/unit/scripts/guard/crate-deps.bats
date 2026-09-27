#!/usr/bin/env bats
#
# Mirror of `scripts/guard/crate-deps.sh` (scripts:C13).
#
# The rule (languages/api:V32, scripts/guard:T47): the dependency shape of
# the workspace, read from `cargo metadata` rather than from review. The
# api depends on `xenolith-shebang` alone and declares no feature; a
# language crate reaches the workspace only through the api -- never the
# root crate, never another language crate -- and its grammar is its own.
#
# Each test hands the guard a `cargo` stub that prints a metadata document
# built here, so the rule is exercised on shapes the real workspace does
# not (yet) have. One test runs the real `cargo metadata` over this repo.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/crate-deps.sh"
  ROOT="$(cd "${BATS_TEST_DIRNAME}/../../../.." && pwd)"
  BIN="${BATS_TEST_TMPDIR}/bin"
  META="${BATS_TEST_TMPDIR}/metadata.json"
  ARGS="${BATS_TEST_TMPDIR}/cargo-args"
  mkdir -p "$BIN"
  PACKAGES=""
  # The stub records how it was called and prints the document; `STUB_FAIL`
  # makes it behave like a cargo that could not read the workspace.
  cat >"${BIN}/cargo" <<EOF
#!/usr/bin/env bash
printf '%s\n' "\$*" >"${ARGS}"
if [ -n "\${STUB_FAIL:-}" ]; then
  echo "error: could not find Cargo.toml" >&2
  exit 101
fi
cat "${META}"
EOF
  chmod +x "${BIN}/cargo"
}

# pkg NAME FEATURES DEP... -- one workspace package. FEATURES is a JSON
# object; a DEP is `name` (normal), `name:dev` or `name:build`.
pkg() {
  local name="$1" features="$2" deps="" dep dname kind
  shift 2
  for dep in "$@"; do
    dname="${dep%%:*}"
    kind="null"
    case "$dep" in
    *:dev) kind='"dev"' ;;
    *:build) kind='"build"' ;;
    esac
    deps="${deps:+${deps},}{\"name\":\"${dname}\",\"kind\":${kind}}"
  done
  local one="{\"name\":\"${name}\",\"features\":${features},\"dependencies\":[${deps}]}"
  PACKAGES="${PACKAGES:+${PACKAGES},}${one}"
}

# The shape this repo has today: every rule holds.
healthy() {
  pkg xenolith '{"default":["lang-shell"],"lang-shell":["dep:xenolith-lang-shell"]}' \
    serde_json toml xenolith-lang-api xenolith-lang-shell xenolith-lang-nix
  pkg xenolith-lang-api '{}' xenolith-shebang
  pkg xenolith-shebang '{}' serde_json:dev
  pkg xenolith-lang-shell '{}' tree-sitter tree-sitter-bash xenolith-lang-api
  pkg xenolith-lang-nix '{}' rnix xenolith-lang-api
  pkg xenolith-lang-pkl '{}' tree-sitter xenolith-lang-api cc:build
}

run_guard() {
  printf '{"packages":[%s],"workspace_members":[],"version":1}\n' "$PACKAGES" >"$META"
  PATH="${BIN}:${PATH}" run "$BASH" "$SCRIPT"
}

@test "the healthy shape passes silently" {
  healthy
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "it asks cargo for the workspace only, offline" {
  healthy
  run_guard
  [ "$status" -eq 0 ]
  [[ "$(cat "$ARGS")" == *"metadata"* ]]
  [[ "$(cat "$ARGS")" == *"--no-deps"* ]]
  [[ "$(cat "$ARGS")" == *"--offline"* ]]
  [[ "$(cat "$ARGS")" == *"--format-version 1"* ]]
}

@test "an api feature fails" {
  healthy
  pkg xenolith-lang-api '{"fancy":[]}' xenolith-shebang
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-api"* ]]
  [[ "$output" == *"fancy"* ]]
  [[ "$output" == *"languages/api:V32"* ]]
}

@test "a grammar dependency of the api fails" {
  healthy
  pkg xenolith-lang-api '{}' xenolith-shebang tree-sitter-bash
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-api"*"tree-sitter-bash"* ]]
}

@test "any api dependency but xenolith-shebang fails, dev kind included" {
  healthy
  pkg xenolith-lang-api '{}' xenolith-shebang serde_json:dev
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"serde_json"*"dev"* ]]
}

@test "a language crate depending on the root crate fails" {
  healthy
  pkg xenolith-lang-xml '{}' tree-sitter xenolith-lang-api xenolith
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-xml"*"xenolith "*"root crate"* ]]
}

@test "a language crate depending on another language crate fails" {
  healthy
  pkg xenolith-lang-bats '{}' xenolith-lang-api xenolith-lang-shell
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-bats"*"xenolith-lang-shell"*"another language crate"* ]]
}

@test "a dev-dependency on another language crate fails too" {
  healthy
  pkg xenolith-lang-bats '{}' xenolith-lang-api xenolith-lang-shell:dev
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-shell (dev)"* ]]
}

@test "a language crate reaches shebang only through the api" {
  healthy
  pkg xenolith-lang-just '{}' xenolith-lang-api xenolith-shebang
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-just"*"xenolith-shebang"* ]]
}

@test "a grammar shared by two language crates fails, naming both" {
  healthy
  pkg xenolith-lang-bats '{}' xenolith-lang-api tree-sitter tree-sitter-bash
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"tree-sitter-bash"* ]]
  [[ "$output" == *"xenolith-lang-bats"* ]]
  [[ "$output" == *"xenolith-lang-shell"* ]]
}

@test "the tree-sitter runtime is not a grammar and may be shared" {
  healthy
  pkg xenolith-lang-tcl '{}' tree-sitter xenolith-lang-api cc:build
  run_guard
  [ "$status" -eq 0 ]
}

@test "one crate naming its grammar twice (normal and dev) is not sharing" {
  healthy
  pkg xenolith-lang-xml '{}' tree-sitter tree-sitter-xml tree-sitter-xml:dev xenolith-lang-api
  run_guard
  [ "$status" -eq 0 ]
}

@test "every violation is reported, not only the first" {
  healthy
  pkg xenolith-lang-api '{"fancy":[]}' xenolith-shebang
  pkg xenolith-lang-xml '{}' xenolith-lang-api xenolith
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"fancy"* ]]
  [[ "$output" == *"root crate"* ]]
}

@test "a cargo that cannot read the workspace fails, not passes" {
  healthy
  STUB_FAIL=1 run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "a workspace with no api package fails rather than passing vacuously" {
  pkg xenolith '{}' serde_json
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-api"* ]]
}

@test "a missing jq is a missing tool, not a pass" {
  healthy
  printf '{"packages":[%s]}\n' "$PACKAGES" >"$META"
  PATH="${BIN}" run "$BASH" "$SCRIPT"
  [ "$status" -eq 1 ]
  [[ "$output" == *"jq"* ]]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "this repository's workspace passes" {
  cd "$ROOT" || return 1
  run "$BASH" "$SCRIPT"
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}
