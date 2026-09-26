# SPEC

## §G GOAL

the ROUND TRIP: `rewrite`/`inline`, `unescape`/`escape`, runtime base, `laws::check` & the lens laws every language crate runs.

## §N NAV

rel|path|lens
up|.|-
up|languages|1 crate + node per language behind `lang-<lang>`: parser, sinks, load idiom, default linter
up|languages/api|contract crate: `Host`/`Guest` traits, `LangId`, shared types, lens law harness
up|languages/api/src|api modules: site, lens, holes; hub root `lib.rs` & `shebang.rs` re-export
self|languages/api/src/lens|rewrite/inline, escape, runtime base, laws harness
sib|languages/api/src/site|Site/Delim/GuestEnv types, placement, claims, candidates
sib|languages/api/src/holes|param naming, param refs, hole advice

## §I INTERFACES

- fn `laws::check::<H: Host>(fixtures: &Path)` → panics w/ fixture path & broken law; called from each language crate's `cargo test`.
- `Host::unescape(&Delim, raw) -> Result<String>` (strip common indent, host escapes like nix `''$`; raw w/ holes already replaced by the engine's placeholder; `Err` ∀ `DelimKind` ⊥ this host's → `Unsupported`) & `Host::escape(&Delim, body) -> Result<String>` inverse (default `Unsupported`, `languages/api:V37`); `rewrite`/`inline` go through them.
- fn `lens::escape_law(&H, &Delim, raw)`: `b = unescape(raw)` ⇒ `unescape(escape(b)) == b` byte-exact; any `Err` = break (V39). `check`: `Err` → site flagged `unparseable <host> string` (`languages:V77` spirit), ⊥ silently raw.
- `Host::runtime_base(&Site) -> Base` ∈ `HostDir` (default) \| `RepoRoot` \| `Dir(path)`: directory the host's runtime resolves load paths from.

## §V INVARIANTS

V34: lens laws ∀ host, ∀ site `s` of fixture `x`, `y = rewrite(x, s, guest.invoke(p), p)`: (a) `inline(y, load, unescape(s.delim, s.delim.body))` ≡ `x` normalized whitespace (`src/extract:V4`); (b) `sites(y)` ∌ `s`; (c) `loads(y)` ∋ load of `p`; (d) `rewrite` on host w/ ⊥ sites = identity (`src/extract:V5`); (e) inverse of `languages/api/src/holes:V40`: `inline` reads `NAME=<hole>` pairs from load & replaces `param_refs` in body by original hole text ∴ (a) holds for sites w/ holes. enforced by `laws::check` ∀ language crate, ⊥ per-crate hand tests.
V39: body text for guest = `unescape(delim, raw)`; law V34(a) holds through `unescape`/`escape` round-trip; ∀ `DelimKind` ∃ fixture w/ indent + escape cases.
V63: extract file = `shebang::wrap(body, prelude)`; inline from disk = `shebang::strip_strict(file, &guest.prelude(&site.env))` ∴ V34(a) holds over file ON DISK, ⊥ only in-memory body. ∀ guest property: `strip_strict(wrap(body, p), p) == body`; vectors shared w/ nix-shebang.
V66: load path in `rewrite` & `LoadRef.path` relative to site's runtime base — default HOST FILE dir (`./sub/x.sh`), else `Host::runtime_base` | rule `base` (`src/extract:V45`); ⊥ cwd relative. placement paths (`src/extract:V46`) stay repo-root relative; engine converts.

## §T TASKS

| id | scope | tasks | done-when |
|----|-------|-------|-----------|
| M1 | nix + pkl + shell end-to-end | T43, T45, T90 | `xnl check`/`extract`/`graph`/`lint` green on this repo for nix, pkl & shell (`.:V19`) |

id|status|task|cites
T43|.|`laws::check` harness + fixture loader over calling crate's `tests/fixtures/<case>/`; RED on toy host in api tests|V34,`tests:V14`
T45|.|`Delim`/`DelimKind`/holes + `unescape`/`escape` round-trip property in harness; fixtures ∀ kind incl. indent, escapes, holes|`languages/api/src/site:V38`,V39,`languages/api/src/holes:V40`,`tests:V15`
T90|.|param inverse in `inline` + law (e) in harness; fixture: 2 holes round-trip byte-equal|V34,`languages/api/src/holes:V40`

## §B BUGS

id|date|cause|fix
