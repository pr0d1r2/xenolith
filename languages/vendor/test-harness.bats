#!/usr/bin/env bats

setup() {
  fixture="${BATS_TEST_TMPDIR}/grammar"
  mkdir -p "${fixture}/src"
  mkdir -p "${fixture}/vendor/tree-sitter-fixture"
  printf '%s\n' '[package]' 'name = "fixture-grammar"' 'version = "0.1.0"' 'edition = "2021"' 'license = "MIT"' 'include = ["Cargo.toml", "build.rs", "src/**", "vendor/**"]' >"${fixture}/Cargo.toml"
  printf '%s\n' '[build-dependencies]' 'cc = "1"' >>"${fixture}/Cargo.toml"
  printf '%s\n' 'fn main() { cc::Build::new().file("src/parser.c").compile("fixture-grammar"); }' >"${fixture}/build.rs"
  printf '%s\n' 'pub fn fixture_parser() {}' >"${fixture}/src/lib.rs"
  printf '%s\n' 'int fixture_parser(void) { return 0; }' >"${fixture}/src/parser.c"
  printf '%s\n' 'repo: https://example.invalid/tree-sitter-fixture' 'rev: deadbeef' 'license: MIT' >"${fixture}/vendor/tree-sitter-fixture/UPSTREAM"
  printf '%s\n' 'MIT License' >"${fixture}/vendor/tree-sitter-fixture/LICENSE"
}

vendoring_harness() {
  if (($# != 1)); then
    echo "usage: vendoring_harness CRATE_DIR" >&2
    return 2
  fi
  crate=$1
  for file in Cargo.toml build.rs; do
    [[ -f "${crate}/${file}" ]] || {
      echo "vendoring: missing ${file}" >&2
      return 1
    }
  done
  grammar=${crate}/vendor/tree-sitter-fixture
  [[ -f "${grammar}/UPSTREAM" ]] || {
    echo "vendoring: missing vendor/tree-sitter-fixture/UPSTREAM" >&2
    return 1
  }
  for field in repo rev license; do
    rg -q "^${field}:[[:space:]]*[^[:space:]]" "${grammar}/UPSTREAM" || {
      echo "vendoring: UPSTREAM missing ${field}" >&2
      return 1
    }
  done
  rg -q '(^|[^[:alnum:]_])cc::Build([^[:alnum:]_]|$)' "${crate}/build.rs" || {
    echo "vendoring: build.rs does not use cc::Build" >&2
    return 1
  }
  find "${grammar}" -maxdepth 1 -type f -iname 'LICENSE*' -print -quit | grep -q . || {
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
  tar -tf "${crate_file}" | grep -q '/vendor/tree-sitter-fixture/UPSTREAM$' || {
    echo "vendoring: packaged crate does not contain grammar metadata" >&2
    return 1
  }
  tar -xf "${crate_file}" -C "${work}"
  unpacked=$(find "${work}" -mindepth 1 -maxdepth 1 -type d -name '*-*' -print -quit)
  [[ -n "${unpacked}" ]] || {
    echo "vendoring: could not unpack crate" >&2
    return 1
  }
  (cd "${unpacked}" && cargo test --offline --target-dir "${work}/unpacked-target" >/dev/null) || {
    echo "vendoring: unpacked crate tests failed" >&2
    return 1
  }
  echo "vendoring: ${crate} metadata, cc build, and packaged C passed"
}

@test "vendoring harness accepts a packaged C grammar" {
  run vendoring_harness "${fixture}"
  [ "${status}" -eq 0 ]
}

@test "vendoring harness rejects a missing revision" {
  sed -i '/^rev:/d' "${fixture}/vendor/tree-sitter-fixture/UPSTREAM"
  run vendoring_harness "${fixture}"
  [ "${status}" -ne 0 ]
  [[ "${output}" == *"rev"* ]]
}

@test "vendoring harness rejects failing unpacked crate tests" {
  printf '%s\n' '#[cfg(test)] mod tests { #[test] fn fails() { panic!("fixture failure"); } }' >>"${fixture}/src/lib.rs"
  run vendoring_harness "${fixture}"
  [ "${status}" -ne 0 ]
  [[ "${output}" == *"unpacked crate tests failed"* ]]
}
