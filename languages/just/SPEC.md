# SPEC

## §G GOAL

crate `xenolith-lang-just` (feature `lang-just`): tree-sitter-just; host: recipe bodies (shell by default, shebang recipe → shebang's guest), load idiom `bash <rel>/x.sh`.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/just|just parser, recipe sinks, load idiom
sib|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom
sib|languages/shell|bash parser & host sinks, single-command classifier, shell linters

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
