# What an extraction leaves behind, and what the fleet already writes:
# one line running a script. `loads` reads each back for `xnl graph`,
# arguments and all; a shebang recipe's lines are not loads.

build:
    @sh ./scripts/just/build.sh

test *args:
    bash scripts/test.sh {{args}}
    zsh scripts/zsh-only.zsh

script:
    #!/usr/bin/env bash
    bash scripts/inside.sh
