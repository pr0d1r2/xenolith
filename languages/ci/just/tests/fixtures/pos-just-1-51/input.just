# just 1.51 syntax the grammar at tree-sitter-just 0.2.0 rejected
# (`languages/ci/just:B3`): each construct below made the whole file a
# `host-parse-error` (`languages:V78`), so none of its recipes was read.
# The recipes are the sites; the rest is the syntax around them.

set unstable

name := "world"

# `f` format strings, their `{{...}}` interpolations and `{{{{` escape.
greeting := f"hello {{name}}"
literal := f'{{{{name}} is {{name + "!"}}'
block := f"""
  dear {{name}}
"""

# `x` strings, shell-expanded.
bin := x"~/bin"
home := x'$HOME'

# `&&`, `||` and `assert`.
both := greeting && "yes"
either := "" || "fallback"
checked := assert(name == "world", "unexpected name")

# `eager`, an attribute on an assignment, and `unexport`.
eager stamp := "now"

[private]
hidden := "h"

unexport SECRET

# `[arg]` with a key-value `pattern`, which just needs raw (`'\d+'`).
[arg("count", pattern='\d+')]
repeat count:
    echo {{ count }}

# A format string inside a recipe line is part of one interpolation.
hello:
    echo {{ f"hi {{name}}" }}
