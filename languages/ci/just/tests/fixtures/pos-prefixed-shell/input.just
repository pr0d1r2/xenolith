# `set shell` naming its interpreter with a shell-expanded `x"…"` string:
# the argv is known only once the string is expanded, so the dialect is
# not statically readable (`languages/ci/just:V179`). The site is still
# found; its env stays empty and its extraction is a judgement.

set shell := [x"bash", "-cu"]

build:
    cargo build
