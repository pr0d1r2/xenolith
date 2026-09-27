# Two lines are two commands, each in its own shell: a script, flagged
# (`languages/ci/just:V180`). Blank lines inside a body are skipped by
# just, and so by the body the guest reads.

release:
    cargo build --release

    cp target/release/app dist/
