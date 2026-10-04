#!/usr/bin/env bash
#
# The metadata every published crate carries (`nix:V112`,
# `scripts/guard:T346`), read from `cargo metadata` rather than from
# review:
#
#   * `description`, `license`, `repository`, `homepage`, `documentation`,
#     `readme`, `keywords`, `categories` and `rust-version` are set, and
#     keywords and categories are not empty;
#   * `documentation` is the crate's own docs.rs page;
#   * the manifest names `exclude` or `include`, so the tarball ships what
#     was decided rather than whatever the directory holds. cargo metadata
#     does not carry either key, so each manifest is read with `taplo get`;
#   * the root crate's `[package.metadata.docs.rs] features` name every
#     `lang-*` feature, so docs.rs documents every language.
#
# `publish = false` (`xenolith-dev`) is exempt. The rule itself is
# `crate-metadata.jq` beside this file.
#
# Silent and 0 when every crate is complete; one line per violation and 1
# otherwise. A cargo, jq or taplo that could not run is a failure too:
# nothing was checked, and saying nothing would read as a pass.
set -euo pipefail

here="${BASH_SOURCE[0]}"
case "$here" in
*/*) here="${here%/*}" ;;
*) here="." ;;
esac
rule="${here}/crate-metadata.jq"

for tool in cargo jq taplo; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "crate-metadata: ${tool} is not on PATH -- re-enter the dev shell" \
      "(\`direnv reload\`, or \`nix develop\`); nothing was checked." >&2
    exit 1
  fi
done

# `--no-deps`: the rule is about the workspace's own manifests, so the
# registry is never resolved and the run stays offline (`.:C3`).
if ! metadata="$(cargo metadata --format-version 1 --no-deps --offline)"; then
  echo "crate-metadata: cargo metadata failed -- nothing was checked," \
    "which is a failure rather than a pass." >&2
  exit 1
fi

if ! manifests="$(printf '%s' "$metadata" | jq -r -f "${here}/crate-manifests.jq")"; then
  echo "crate-metadata: jq could not read the cargo metadata -- nothing" \
    "was checked, which is a failure rather than a pass." >&2
  exit 1
fi

packed=""
while IFS= read -r manifest; do
  [ -n "$manifest" ] || continue
  if taplo get -f "$manifest" package.exclude >/dev/null 2>&1 ||
    taplo get -f "$manifest" package.include >/dev/null 2>&1; then
    packed+="${manifest}"$'\n'
  fi
done <<<"$manifests"

if ! findings="$(printf '%s' "$metadata" | jq -r --arg packed "$packed" -f "$rule")"; then
  echo "crate-metadata: jq could not read the cargo metadata -- nothing" \
    "was checked, which is a failure rather than a pass." >&2
  exit 1
fi

if [ -n "$findings" ]; then
  printf '%s\n' "$findings" >&2
  exit 1
fi
