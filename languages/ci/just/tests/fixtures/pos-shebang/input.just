# A `#!` first line makes the whole body one script for the interpreter
# it names: python here, bash below. An interpreter the api does not know
# is no site, never "shell, probably".

report:
    #!/usr/bin/env python3
    import sys

    print(sys.argv)

setup:
    #!/usr/bin/env bash
    set -euo pipefail
    echo {{{{ literal }}

tcl:
    #!/usr/bin/env tclsh
    puts hi
