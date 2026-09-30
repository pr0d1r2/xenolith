# Working here

For agents and humans. Read this before changing anything; read `SPEC.md` for
what must hold and what to build next.
[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) explains the same loop at more
length; this page is the checklist.

xenolith finds code of one language embedded in a file of another — a shell
script in a Nix attribute, in a justfile recipe, in an hk step — and moves it
into its own file, rewriting the host to run that file. The binary is `xnl`.
The README says what works today.

## Start here: the gate

```sh
hk check --all
```

That is the whole gate, defined once in [`hk.pkl`](hk.pkl) and run the same
way by the git hooks and by CI. Entering the dev shell — `nix develop`, or
`direnv allow` — provides every tool and installs the hooks; do not install
tools some other way, the flake pins them. The hooks REFUSE when `hk` is not
on `PATH` rather than skipping: a gate that cannot run has not passed.

[`docs/INTEGRATION.md`](docs/INTEGRATION.md) has the whole flow: which steps
run at each stage, which files each stage examines, and why the cargo steps
are chained.

The repository also checks itself: `xnl check` runs over this tree as a gate
step, so this repository may not contain the kind of embed the tool exists to
remove.

## The rule

**Never `--no-verify`**, on commit or on push. A failing step is either a
real problem in the change or a wrong step; a wrong step gets fixed in its
own commit, with its reason. Skipping it hides both.

The subtler versions count too: weakening a test, raising a ceiling in
`.context-limits`, lowering the floor in `.coverage`, or adding an `#[allow]`
to get past clippy. A raise, when it is right, is its own commit with its own
`Why:`.

**Report what you examined**, not only what failed. A vacuous pass and a real
one look identical.

## Reproduce a verdict

```sh
hk check --all --check              # the whole gate, as CI runs it
hk check --all --check -S clippy    # one step
```

Every step is one plain command. If `hk` is not to hand, read the command out
of `hk.pkl` and run it directly — the gate never depends on hk to be
reproducible.

## Reading a SPEC.md

Caveman-encoded; symbols are load-bearing.

```
→ leads to    ∴ therefore    ∀ for all      ! must
⊥ never       ? optional     ≤ at most      ∈ in
```

Sections: `§G` goal · `§F` federation · `§N` navigation · `§C` constraints ·
`§I` interfaces · `§R` research · `§V` invariants · `§T` tasks · `§B` bugs.
`§R` and `§B` hold the evidence.

Read the chain: [`SPEC.md`](SPEC.md) at the root, then the `SPEC.md` of every
directory between the root and the one you are changing. The root's `§F`
table says which node owns what; each node's `§N` table links its parent,
siblings and children. Ids are node-scoped: cite across nodes with the
namespaced, backticked form `` `src/cli:V24` ``.

## Writing a SPEC.md

- The spec is the source of truth; the code follows it. When code and spec
  disagree, that is a bug in one of them — say which, do not quietly pick.
- Where the spec is silent, make the most conservative choice and write it
  down as a spec change, in its own commit, **before** the test that depends
  on it.
- **§T states remaining work, never history** (sherd's `src/fed:V9`). Delete a
  finished row in the same change. Its id remains retired and must never be
  reused, so citations to it remain historical references in existing
  records. A measurement or decision the row carried goes to `§R` first.
- **§B for every defect**, at the node that owns it: date, cause, fix. Prefer
  adding a §V that catches recurrence over a bug row alone.
- **Ids are monotonic and never reused.** Append; inserting moves every
  citation below it.
- Never edit a `§N` table by hand; `sherd sync` generates it.
- Each node's spec chain has a token ceiling in `.context-limits`. Do not
  raise one to make a change fit. Move rows to the node that uses them, or
  stop and ask.

## Test first, then code

Every change to behaviour is a RED commit followed by a GREEN one:

1. `test:` — the failing test, fixture or bats file, and nothing else.
2. `feat:` or `fix:` — the code that makes it pass.
3. `refactor:` — optional, behaviour unchanged.

`scripts/guard/tdd-order.sh` checks the order in history. Keep the mirrors
whole: every Rust file with logic has a sibling `tests.rs` wired with
`#[cfg(test)] mod tests;`, and every script under `scripts/` has a bats file
at the mirrored path under `tests/unit/`.

## Generated files

The README's `badges` and `langs` blocks and all of
`docs/THIRD-PARTY-NOTICES.md` are rendered by `xenolith-dev` (`dev/`, never
published) from the files that own each fact. Never edit them by hand: change
the owner, then `cargo run -q -p xenolith-dev -- --fix` (or `hk fix`).

## Commits

- One logical change per commit.
- [Conventional Commits](https://www.conventionalcommits.org/) subject:
  `type(scope): summary`.
- A body that explains the change in prose, with a line starting `Why:` that
  cites the spec id it serves. The commit-msg hook refuses a body without
  one.
- Never name a private repository — in code, fixtures, docs or commit
  messages. Fixtures are synthetic or anonymised.

## Pull requests

- One change per pull request.
- Say which spec rows it serves and what you ran.
- After opening it, **stop and wait for review.** Do not stack further
  changes on an unreviewed one, merge it yourself, or push over review
  comments without answering them.
