# `set shell` is read statically (`languages/ci/just:V179`): bash, with
# `-u` from the argv and `-e` from just's stop at the first failing line.

set shell := ["bash", "-uc"]

lint:
    shellcheck scripts/*.sh
    shfmt --diff scripts
