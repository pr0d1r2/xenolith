#!/usr/bin/env bats

setup() {
  fixture="${BATS_TEST_TMPDIR}/grammar"
  mkdir -p "${fixture}/src"
  printf '%s\n' '[package]' 'name = "fixture-grammar"' 'version = "0.1.0"' 'edition = "2021"' 'license = "MIT"' 'include = ["Cargo.toml", "build.rs", "src/**", "UPSTREAM", "LICENSE"]' >"${fixture}/Cargo.toml"
  printf '%s\n' '[build-dependencies]' 'cc = "1"' >>"${fixture}/Cargo.toml"
  printf '%s\n' 'fn main() { cc::Build::new().file("src/parser.c").compile("fixture-grammar"); }' >"${fixture}/build.rs"
  printf '%s\n' 'int fixture_parser(void) { return 0; }' >"${fixture}/src/parser.c"
  printf '%s\n' 'repo: https://example.invalid/tree-sitter-fixture' 'rev: deadbeef' 'license: MIT' >"${fixture}/UPSTREAM"
  printf '%s\n' 'MIT License' >"${fixture}/LICENSE"
}

vendoring_harness() {
  if (($# != 1)); then
    echo "usage: vendoring_harness CRATE_DIR" >&2
    return 2
  fi
  crate=$1
  for file in Cargo.toml build.rs UPSTREAM; do
    [[ -f "${crate}/${file}" ]] || {
      echo "vendoring: missing ${file}" >&2
      return 1
    }
  done
  for field in repo rev license; do
    rg -q "^${field}:[[:space:]]*[^[:space:]]" "${crate}/UPSTREAM" || {
      echo "vendoring: UPSTREAM missing ${field}" >&2
      return 1
    }
  done
  rg -q '(^|[^[:alnum:]_])cc::Build([^[:alnum:]_]|$)' "${crate}/build.rs" || {
    echo "vendoring: build.rs does not use cc::Build" >&2
    return 1
  }
  find "${crate}" -maxdepth 1 -type f -iname 'LICENSE*' -print -quit | grep -q . || {
    echo "vendoring: missing license file" >&2
    return 1
  }
  work=$(mktemp -d)
  trap 'rm -rf "${work}"' RETURN
  (cd "${crate}" && cargo package --allow-dirty --offline --target-dir "${work}/target" >/dev/null)
  crate_file=$(find "${work}" -name '*.crate' -print -quit)
  [[ -n "${crate_file}" ]] || {
    echo "vendoring: cargo package produced no crate" >&2
    return 1
  }
  tar -tf "${crate_file}" | grep -q '/src/parser.c$' || {
    echo "vendoring: packaged crate does not contain src/parser.c" >&2
    return 1
  }
  tar -xf "${crate_file}" -C "${work}"
  unpacked=$(find "${work}" -mindepth 1 -maxdepth 1 -type d -name '*-*' -print -quit)
  [[ -n "${unpacked}" ]] || {
    echo "vendoring: could not unpack crate" >&2
    return 1
  }
  (cd "${unpacked}" && cargo test --offline --target-dir "${work}/unpacked-target" >/dev/null)
  echo "vendoring: ${crate} metadata, cc build, and packaged C passed"
}

@test "vendoring harness accepts a packaged C grammar" {
  run vendoring_harness "${fixture}"
  [ "${status}" -eq 0 ]
}

@test "vendoring harness rejects a missing revision" {
  sed -i '/^rev:/d' "${fixture}/UPSTREAM"
  run vendoring_harness "${fixture}"
  [ "${status}" -ne 0 ]
  [[ "${output}" == *"rev"* ]]
}
