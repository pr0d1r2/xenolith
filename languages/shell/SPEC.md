# SPEC

## §G GOAL

crate `lydite-lang-shell` (feature `lang-shell`): tree-sitter-bash; bash as host (heredoc to interpreter, `-c`/`-e` args), single-command classifier shared by ∀ shell sink, shell extract load idiom, default linters shellcheck + shfmt.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
self|languages/shell|bash parser & host sinks, single-command classifier, shell linters
sib|languages/nix|nix parser, sinks, load idiom
sib|languages/pkl|pkl parser, hk step sinks, load idiom

## §V INVARIANTS


## §T TASKS

id|status|task|cites

## §B BUGS

id|date|cause|fix
