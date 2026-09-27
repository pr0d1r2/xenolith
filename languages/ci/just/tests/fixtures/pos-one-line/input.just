# One simple command per recipe: a site each, and trivial to the shell
# guest (`languages/shells/shell:V3`), so `xnl check` leaves them inline.

default:
    just --list

build:
    cargo build --release
