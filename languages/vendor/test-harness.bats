#!/usr/bin/env bats

setup() {
  harness="${BATS_TEST_DIRNAME}/harness.sh"
  fixture="${BATS_TEST_TMPDIR}/grammar"
  mkdir -p "${fixture}/src"
  printf '%s\n' '[package]' 'name = "fixture-grammar"' 'version = "0.1.0"' 'edition = "2021"' 'license = "MIT"' 'include = ["Cargo.toml", "build.rs", "src/**", "UPSTREAM", "LICENSE"]' >"${fixture}/Cargo.toml"
  printf '%s\n' '[build-dependencies]' 'cc = "1"' >>"${fixture}/Cargo.toml"
  printf '%s\n' 'fn main() { cc::Build::new().file("src/parser.c").compile("fixture-grammar"); }' >"${fixture}/build.rs"
  printf '%s\n' 'int fixture_parser(void) { return 0; }' >"${fixture}/src/parser.c"
  printf '%s\n' 'repo: https://example.invalid/tree-sitter-fixture' 'rev: deadbeef' 'license: MIT' >"${fixture}/UPSTREAM"
  printf '%s\n' 'MIT License' >"${fixture}/LICENSE"
}

@test "vendoring harness accepts a packaged C grammar" {
  run "${harness}" "${fixture}"
  [ "${status}" -eq 0 ]
}

@test "vendoring harness rejects a missing revision" {
  sed -i '/^rev:/d' "${fixture}/UPSTREAM"
  run "${harness}" "${fixture}"
  [ "${status}" -ne 0 ]
  [[ "${output}" == *"rev"* ]]
}
