# `set windows-shell` with no `set shell` beside it: which shell runs a
# line depends on the OS, so it is not statically readable
# (`languages/ci/just:V179`). The site is still found; its env stays
# empty and its extraction is a judgement.

set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

deploy:
    ./build.sh
    ./upload.sh
