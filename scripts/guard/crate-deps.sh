#!/usr/bin/env bash
#
# The workspace's dependency shape (`languages/api:V32`,
# `scripts/guard:T47`), read from `cargo metadata` rather than from review:
#
#   * `xenolith-lang-api` depends on `xenolith-shebang` and nothing else,
#     and declares no feature. The api is the contract every language
#     implements; a grammar or a feature there would reach every crate.
#   * a language crate (`xenolith-lang-*`, the api aside) reaches the
#     workspace only through the api: never the root crate, never another
#     language crate, and `xenolith-shebang` only via the api.
#   * its grammar is its OWN: a `tree-sitter-*` grammar crate appears in
#     at most one language crate. The `tree-sitter` runtime is not a
#     grammar and is shared by design.
#
# Every dependency kind counts -- normal, build and dev. A dev-dependency
# on another language crate is still a crate that cannot be tested alone.
# What V32 calls "std-ish" is an open set and is not checked here.
#
# The rule itself is `crate-deps.jq` beside this file: jq quoted into shell
# is the embed this project exists to remove.
#
# Silent and 0 when the shape holds; one line per violation and 1
# otherwise. A cargo or jq that could not run is a failure too: nothing was
# checked, and saying nothing would read as a pass.
set -euo pipefail

# Parameter expansion rather than `dirname`: the tool check below must be
# the first thing that can fail on a bare PATH.
here="${BASH_SOURCE[0]}"
case "$here" in
*/*) here="${here%/*}" ;;
*) here="." ;;
esac
rule="${here}/crate-deps.jq"

for tool in cargo jq; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "crate-deps: ${tool} is not on PATH -- re-enter the dev shell" \
      "(\`direnv reload\`, or \`nix develop\`); nothing was checked." >&2
    exit 1
  fi
done

# `--no-deps`: the rule is about the workspace's own manifests, so the
# registry is never resolved and the run stays offline (`.:C3`).
if ! metadata="$(cargo metadata --format-version 1 --no-deps --offline)"; then
  echo "crate-deps: cargo metadata failed -- nothing was checked," \
    "which is a failure rather than a pass." >&2
  exit 1
fi

if ! findings="$(printf '%s' "$metadata" | jq -r -f "$rule")"; then
  echo "crate-deps: jq could not read the cargo metadata -- nothing was" \
    "checked, which is a failure rather than a pass." >&2
  exit 1
fi

if [ -n "$findings" ]; then
  printf '%s\n' "$findings" >&2
  exit 1
fi
