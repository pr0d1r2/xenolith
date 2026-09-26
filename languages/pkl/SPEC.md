# SPEC

## §G GOAL

crate `xenolith-lang-pkl` (feature `lang-pkl`): pkl grammar; hk step sinks, `bash scripts/hk/x.sh {{files}}` load idiom.

## §I INTERFACES

- sinks: hk step `check`, `fix`, `shell`, `check_diff`, `check_list_files` → guest shell; load after extract: `bash scripts/hk/x.sh {{files}}`.

## §V INVARIANTS

V52: pkl host placement: hk step site → name = step key, dir = `scripts/hk`, load = `bash scripts/hk/<name>.sh {{files}}` (hk passes files through).

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T13, T54, T147 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T13|x|host pkl (vendored `apple/tree-sitter-pkl`, Apache-2.0, `languages:V121`): hk step sinks, fixtures|`languages:V2`,`languages/shell:V3`,`tests:V14`,`tests:V15`
T54|.|pkl `Host::placement` for hk steps + fixture (`{{files}}` forwarded)|V52
T147|.|`src:C139` backfill: `languages/pkl/src/host/tests.rs`, `languages/pkl/src/string/tests.rs`|`src:C139`,`scripts/guard:V140`

## §B BUGS

id|date|cause|fix
