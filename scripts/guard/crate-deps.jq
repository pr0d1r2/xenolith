# The rule `scripts/guard/crate-deps.sh` applies to `cargo metadata
# --no-deps` (`languages/api:V32`). Its own file rather than a quoted
# string inside the script: one language per file is this project's whole
# subject, and a jq program pasted into shell is exactly the embed `xnl`
# exists to find.
#
# Emits one line per violation, sorted, so the output is the same bytes on
# every machine; nothing when the shape holds.

def api: "xenolith-lang-api";
def shebang: "xenolith-shebang";
def cite: " (languages/api:V32).";
def kind: (.kind // "normal");
def lang: startswith("xenolith-lang-") and . != api;
# The `tree-sitter` runtime is shared by design; a grammar is the
# `tree-sitter-<language>` crate beside it.
def grammar: startswith("tree-sitter-");

[.packages[].name] as $ws
| [
    # No api at all: every rule below would hold vacuously.
    (if any($ws[]; . == api) then empty
      else "crate-deps: no \(api) package in the workspace -- the contract"
        + " crate is missing, so the shape cannot hold" + cite end),

    # The api: no feature, and `xenolith-shebang` as its only dependency.
    (.packages[] | select(.name == api) | .name as $n
      | ((.features // {}) | keys[]
          | "crate-deps: \($n) declares feature `\(.)`; the api declares none"
            + cite),
        (.dependencies[] | select(.name != shebang)
          | "crate-deps: \($n) depends on \(.name) (\(kind)); the api depends"
            + " on \(shebang) alone" + cite)),

    # A language crate reaches the workspace only through the api.
    (.packages[] | select(.name | lang) | .name as $n
      | .dependencies[] | .name as $d
      | select(any($ws[]; . == $d)) | select($d != api)
      | "crate-deps: \($n) depends on \($d) (\(kind)) -- "
        + (if $d == "xenolith" then "the root crate"
            elif ($d | lang) then "another language crate"
            else "a workspace crate other than the api" end)
        + "; a language crate reaches the workspace only through \(api)"
        + cite),

    # Its grammar is its own: one language crate per grammar crate. `unique`
    # first, so a crate naming its grammar under two kinds is not sharing.
    ([.packages[] | select(.name | lang) | .name as $n
        | .dependencies[] | select(.name | grammar) | {g: .name, c: $n}]
      | unique | group_by(.g)[] | select(length > 1)
      | "crate-deps: grammar \(.[0].g) is used by \([.[].c] | join(", "));"
        + " a language crate parses with its OWN grammar" + cite)
  ]
| sort | .[]
