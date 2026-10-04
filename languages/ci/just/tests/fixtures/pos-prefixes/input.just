# `@` (do not echo) and `-` (ignore failure) are just's, not shell: the
# guest reads each line without them. A `-` line merges as `... || true`;
# an all-`@` body keeps its `@` on the load; `@` on some lines only is a
# judgement, since one load line cannot say it.

clean:
    -rm -rf dist
    -rm -rf target
    mkdir dist

quiet:
    @echo building
    @-cargo build

mixed:
    @echo one
    echo two
