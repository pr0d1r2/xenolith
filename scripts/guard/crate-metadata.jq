# The rule `scripts/guard/crate-metadata.sh` applies to `cargo metadata
# --no-deps` (`nix:V112`). Its own file rather than a quoted string inside
# the script: a jq program pasted into shell is exactly the embed `xnl`
# exists to find.
#
# `$packed` holds the manifest paths, one per line, that declare `exclude` or
# `include`, which the script reads with `taplo get`: cargo metadata does
# not carry either key.
#
# Emits one line per violation, sorted, so the output is the same bytes on
# every machine; nothing when every published crate is complete.

def cite: " (nix:V112).";
# `publish = false` is `[]` in cargo metadata; `null` means crates.io.
def published: .publish != [];
def root: "xenolith";
def unset: . == null or . == "" or . == [];

[.packages[] | select(published)] as $pub
| [
    (if ($pub | length) > 0 then empty
      else "crate-metadata: no published crate in the workspace -- nothing"
        + " would be checked" + cite end),

    ($pub[] | .name as $n
      | (("description", "license", "repository", "homepage",
          "documentation", "readme", "keywords", "categories",
          "rust_version") as $k
          | select(.[$k] | unset)
          | "crate-metadata: \($n) has no `\($k | sub("_"; "-"))`; crates.io"
            + " shows it to every consumer" + cite),
        (select(.documentation | unset | not)
          | select(.documentation != "https://docs.rs/\($n)")
          | "crate-metadata: \($n) documentation is \(.documentation), not"
            + " https://docs.rs/\($n)" + cite),
        (select(.manifest_path as $m | $packed | split("\n") | any(.[]; . == $m) | not)
          | "crate-metadata: \($n) names neither `exclude` nor `include`, so"
            + " its .crate ships whatever the directory holds" + cite)),

    ($pub[] | select(.name == root)
      | ([.features // {} | keys[] | select(startswith("lang-"))]) as $langs
      | (.metadata.docs.rs.features // null) as $docs
      | if $docs == null then
          "crate-metadata: \(root) has no [package.metadata.docs.rs]"
            + " features, so docs.rs documents the default build only" + cite
        else
          ($langs - $docs)[]
          | "crate-metadata: \(root) [package.metadata.docs.rs] features lack"
            + " `\(.)`" + cite
        end)
  ]
| sort | .[]
