# Each line runs in a fresh shell, so a `cd` on one line does not move the
# next (`languages/ci/just:V180`). Merged into one script it would: the
# extraction is a judgement. On the last line nothing reads it, so it
# merges.

docs:
    cd docs
    mdbook build

env:
    export RUST_LOG=debug
    cargo run

last:
    cargo build
    cd target
