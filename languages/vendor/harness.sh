#!/usr/bin/env bash
set -euo pipefail

if (($# != 1)); then
  echo "usage: $0 CRATE_DIR" >&2
  exit 2
fi
crate=$1
for file in Cargo.toml build.rs UPSTREAM; do
  [[ -f "${crate}/${file}" ]] || {
    echo "vendoring: missing ${file}" >&2
    exit 1
  }
done
for field in repo rev license; do
  rg -q "^${field}:[[:space:]]*[^[:space:]]" "${crate}/UPSTREAM" || {
    echo "vendoring: UPSTREAM missing ${field}" >&2
    exit 1
  }
done
rg -q '(^|[^[:alnum:]_])cc::Build([^[:alnum:]_]|$)' "${crate}/build.rs" || {
  echo "vendoring: build.rs does not use cc::Build" >&2
  exit 1
}
find "${crate}" -maxdepth 1 -type f -iname 'LICENSE*' -print -quit | grep -q . || {
  echo "vendoring: missing license file" >&2
  exit 1
}
work=$(mktemp -d)
trap 'rm -rf "${work}"' EXIT
(cd "${crate}" && cargo package --allow-dirty --offline --target-dir "${work}/target" >/dev/null)
crate_file=$(find "${work}" -name '*.crate' -print -quit)
[[ -n "${crate_file}" ]] || {
  echo "vendoring: cargo package produced no crate" >&2
  exit 1
}
tar -tf "${crate_file}" | grep -q '/src/parser.c$' || {
  echo "vendoring: packaged crate does not contain src/parser.c" >&2
  exit 1
}
tar -xf "${crate_file}" -C "${work}"
unpacked=$(find "${work}" -mindepth 1 -maxdepth 1 -type d -name '*-*' -print -quit)
[[ -n "${unpacked}" ]] || {
  echo "vendoring: could not unpack crate" >&2
  exit 1
}
(cd "${unpacked}" && cargo test --offline --target-dir "${work}/unpacked-target" >/dev/null)
echo "vendoring: ${crate} metadata, cc build, and packaged C passed"
