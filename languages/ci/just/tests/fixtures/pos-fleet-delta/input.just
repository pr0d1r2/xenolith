# The delta to the fleet hook `xnl` replaces (`languages/ci/just:V181`,
# `languages/ci/just:R178`), one recipe per direction. The hook passes a
# body only when EVERY line matches its allowlist (`just --list`,
# `bash scripts/…`, `bats tests/…`, `expect tests/…`, `ssh -t u@h …`);
# `xnl` passes one simple command, whatever it is (`languages/ci/just:V180`).
# T185 keeps the delta as the default: `[threshold.just] max_lines`
# (default 1) is the one knob, and it is never a command allowlist.

# LOOSER here: one command off the allowlist -- the hook flags it, `xnl`
# keeps it inline.
build:
    cargo build --release

# STRICTER here: every line on the allowlist -- the hook passes it, `xnl`
# flags two commands as a script at the default ceiling of one line.
check:
    bash scripts/lint.sh
    bats tests/unit

# LOOSER here: a shebang line is off the allowlist, but a shebang and one
# command is one simple command to the shell guest.
hello:
    #!/usr/bin/env bash
    echo hello
