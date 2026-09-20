# Shared vectors

`vectors.json` is not written here. It is the `vectors` output of
[`nix-shebang`](https://github.com/pr0d1r2/nix-shebang), vendored verbatim
so this crate and the nix implementation answer identically for the same
input — the point of porting rather than reimplementing.

Refresh it with:

```sh
nix eval --json github:pr0d1r2/nix-shebang#vectors | jq -S . \
  > languages/shebang/tests/vectors.json
```

Vendored rather than fetched in the test run, because the gate never
reaches the network (`.:C3`). A refresh is therefore a commit, which is
the right shape: a behaviour change upstream should arrive as a diff
somebody reads, not as a test that quietly starts asserting something
else.

Each row pins every text function against one input, limits included —
`envSplitString` records that `parse` does not understand `env -S` and
treats `-S` as the resolved interpreter's argument list head. When this
crate disagrees with a row, one of the two implementations is wrong and
the disagreement is the finding.
