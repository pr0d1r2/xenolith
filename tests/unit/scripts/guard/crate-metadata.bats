#!/usr/bin/env bats
#
# Mirror of `scripts/guard/crate-metadata.sh` (scripts:C13).
#
# The rule (nix:V112, scripts/guard:T346): every published crate carries
# the metadata crates.io and docs.rs show a consumer, read from `cargo
# metadata` rather than from review, and names what its tarball ships
# (`exclude` or `include`, read per manifest with `taplo get`, because
# cargo metadata does not carry either). `publish = false` is exempt.
#
# Each test hands the guard a `cargo` stub printing a metadata document
# built here and a `taplo` stub that answers from a list of manifests
# declaring a packing rule. One test runs the real tools over this repo.

setup() {
  SCRIPT="${BATS_TEST_DIRNAME}/../../../../scripts/guard/crate-metadata.sh"
  ROOT="$(cd "${BATS_TEST_DIRNAME}/../../../.." && pwd)"
  BIN="${BATS_TEST_TMPDIR}/bin"
  META="${BATS_TEST_TMPDIR}/metadata.json"
  ARGS="${BATS_TEST_TMPDIR}/cargo-args"
  PACKED="${BATS_TEST_TMPDIR}/packed"
  mkdir -p "$BIN"
  : >"$PACKED"
  PACKAGES=""
  cat >"${BIN}/cargo" <<STUB
#!/usr/bin/env bash
printf '%s\n' "\$*" >"${ARGS}"
if [ -n "\${STUB_FAIL:-}" ]; then
  echo "error: could not find Cargo.toml" >&2
  exit 101
fi
cat "${META}"
STUB
  # `taplo get -f MANIFEST KEY`: succeeds when "MANIFEST KEY" is listed.
  cat >"${BIN}/taplo" <<STUB
#!/usr/bin/env bash
grep -qxF "\$3 \$4" "${PACKED}"
STUB
  chmod +x "${BIN}/cargo" "${BIN}/taplo"
}

# pkg NAME [FIELD=JSON...] -- one package, every V112 field set unless a
# FIELD overrides it (`publish=[]` for an unpublished one, `readme=null`
# for a missing one). Its manifest declares `exclude` unless `packed=no`.
pkg() {
  local name="$1" manifest="/w/${1}/Cargo.toml" packed="package.exclude" kv
  shift
  local -A f=(
    [publish]=null
    [description]='"d"'
    [license]='"MIT"'
    [repository]='"https://github.com/o/r"'
    [homepage]='"https://github.com/o/r"'
    [documentation]="\"https://docs.rs/${name}\""
    [readme]='"README.md"'
    [keywords]='["k"]'
    [categories]='["development-tools"]'
    [rust_version]='"1.95"'
    [features]='{}'
    [metadata]=null
  )
  for kv in "$@"; do
    case "$kv" in
    packed=no) packed="" ;;
    packed=include) packed="package.include" ;;
    *) f[${kv%%=*}]="${kv#*=}" ;;
    esac
  done
  [ -z "$packed" ] || printf '%s %s\n' "$manifest" "$packed" >>"$PACKED"
  local one="{\"name\":\"${name}\",\"manifest_path\":\"${manifest}\""
  for kv in "${!f[@]}"; do
    one="${one},\"${kv}\":${f[$kv]}"
  done
  PACKAGES="${PACKAGES:+${PACKAGES},}${one}}"
}

DOCSRS='{"docs":{"rs":{"features":["lang-a","lang-b"]}}}'

healthy() {
  pkg xenolith features='{"default":[],"lang-a":[],"lang-b":[]}' metadata="$DOCSRS"
  pkg xenolith-lang-a packed=include
  pkg xenolith-dev publish='[]' documentation=null readme=null keywords='[]' packed=no
}

run_guard() {
  printf '{"packages":[%s],"workspace_members":[],"version":1}\n' "$PACKAGES" >"$META"
  PATH="${BIN}:${PATH}" run "$BASH" "$SCRIPT"
}

@test "the healthy workspace passes silently" {
  healthy
  run_guard
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}

@test "it asks cargo for the workspace only, offline" {
  healthy
  run_guard
  [[ "$(cat "$ARGS")" == *"metadata"* ]]
  [[ "$(cat "$ARGS")" == *"--no-deps"* ]]
  [[ "$(cat "$ARGS")" == *"--offline"* ]]
  [[ "$(cat "$ARGS")" == *"--format-version 1"* ]]
}

@test "each missing field fails, naming the crate, the field and the rule" {
  local field
  for field in description license repository homepage documentation readme rust_version; do
    PACKAGES=""
    : >"$PACKED"
    healthy
    pkg xenolith-lang-b "${field}=null"
    run_guard
    [ "$status" -eq 1 ]
    [[ "$output" == *"xenolith-lang-b"*"${field//_/-}"*"nix:V112"* ]]
  done
}

@test "empty keywords or categories fail" {
  healthy
  pkg xenolith-lang-b keywords='[]' categories='[]'
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-b"*"keywords"* ]]
  [[ "$output" == *"xenolith-lang-b"*"categories"* ]]
}

@test "documentation that is not the crate's docs.rs page fails" {
  healthy
  pkg xenolith-lang-b documentation='"https://example.org/doc"'
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-b"*"https://docs.rs/xenolith-lang-b"* ]]
}

@test "a crate naming neither exclude nor include fails" {
  healthy
  pkg xenolith-lang-b packed=no
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith-lang-b"*"exclude"*"include"* ]]
}

@test "the root crate without docs.rs features fails" {
  pkg xenolith features='{"lang-a":[]}'
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"xenolith"*"package.metadata.docs.rs"* ]]
}

@test "the root crate whose docs.rs features miss a lang feature fails, naming it" {
  pkg xenolith features='{"lang-a":[],"lang-b":[],"lang-c":[]}' metadata="$DOCSRS"
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"lang-c"* ]]
  [[ "$output" != *"lang-a"* ]]
}

@test "an unpublished crate is exempt" {
  pkg xenolith features='{}' metadata='{"docs":{"rs":{"features":[]}}}'
  pkg xenolith-dev publish='[]' description=null documentation=null packed=no
  run_guard
  [ "$status" -eq 0 ]
}

@test "every violation is reported, not only the first" {
  healthy
  pkg xenolith-lang-b license=null packed=no
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"license"* ]]
  [[ "$output" == *"include"* ]]
}

@test "a cargo that cannot read the workspace fails, not passes" {
  healthy
  STUB_FAIL=1 run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "a workspace with no published crate fails rather than passing vacuously" {
  pkg xenolith-dev publish='[]' packed=no
  run_guard
  [ "$status" -eq 1 ]
  [[ "$output" == *"no published crate"* ]]
}

@test "a missing taplo is a missing tool, not a pass" {
  healthy
  rm "${BIN}/taplo"
  printf '{"packages":[%s]}\n' "$PACKAGES" >"$META"
  mkdir -p "${BATS_TEST_TMPDIR}/jq-only"
  ln -s "$(command -v jq)" "${BATS_TEST_TMPDIR}/jq-only/jq"
  PATH="${BIN}:${BATS_TEST_TMPDIR}/jq-only" run "$BASH" "$SCRIPT"
  [ "$status" -eq 1 ]
  [[ "$output" == *"taplo"* ]]
  [[ "$output" == *"nothing was checked"* ]]
}

@test "this repository's workspace passes" {
  cd "$ROOT" || return 1
  run "$BASH" "$SCRIPT"
  [ "$status" -eq 0 ]
  [ "$output" = "" ]
}
