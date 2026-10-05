# `{{...}}` is just's interpolation, a hole (`languages/api/src/holes:V40`).
# How a load passes one is not decided yet, so extraction refuses them.
# `x{{{{y}}` is a literal `x{{y}}` to just, whatever the grammar reads in
# it (`languages/ci/just:B2`): no hole, and it merges as the text it is.

test target="all":
    cargo test {{target}}
    echo done {{ if target == "all" { "everything" } else { target } }}

braces:
    echo x{{{{y}}
    echo z
