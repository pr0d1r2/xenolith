# echo a; echo b -- a comment, not a site (`languages:V2`)

set shell := ["bash", "-c"]

# Strings and backticks in expressions are just, evaluated by just.
version := `git describe --tags; echo fallback`
greeting := "echo hi && echo there"

alias b := build

# A recipe with no body runs its dependencies only.
build: fmt lint

fmt:

lint:

# `[script]` bodies take `set script-interpreter`, which the spec leaves
# open: no site until it is decided.
[script]
gen:
    echo one
    echo two

# An interpreter the api does not know is no site.
tcl:
    #!/usr/bin/env tclsh
    puts one
    puts two
