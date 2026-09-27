# AGENTS.md

Working rules for anyone — a coding agent or a person — changing this
repository. [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) explains the same
loop at more length; this page is the checklist.

## What this is

xenolith finds code of one language embedded in a file of another — a shell
script in a Nix attribute, in a justfile recipe, in an hk step — and moves it
into its own file, rewriting the host to run that file. The binary is `xnl`.
The README says what works today.

## Before you change anything

- **Enter the dev shell.** `nix develop` (or `direnv allow`). It provides
  every tool and installs the git hooks. Do not install tools some other
  way; the flake pins them.
- **Read the spec chain.** [`SPEC.md`](SPEC.md) at the root, then the
  `SPEC.md` of every directory between the root and the one you are
  changing. The root's `§F` table says which node owns what. Each node's
  `§N` table links its parent, siblings and children.
- **Find the rows.** The change you are about to make serves a `§V`
  invariant or a `§T` task somewhere in that chain. If it does not, the
  spec comes first.

## The spec comes first

- The spec is the source of truth; the code follows it. When code and spec
  disagree, that is a bug in one of them — say which, do not quietly pick.
- Where the spec is silent, make the most conservative choice and write it
  down as a spec change, in its own commit, **before** the test that
  depends on it.
- Cite ids with their node: `src/cli:V24`, `languages/ci/just:V180`.
- A bug found gets a `§B` row: date, cause, fix.
- Finish a task by flipping its row from `.` to `x` in the same change.
- Never edit a `§N` table by hand; `sherd sync` generates it.
- Each node's spec chain has a token ceiling in `.context-limits`. Do not
  raise one to make a change fit. Move rows to the node that uses them, or
  stop and ask. A raise, when it is right, is its own commit with its own
  `Why:`.

## Test first, then code

Every change to behaviour is a RED commit followed by a GREEN one:

1. `test:` — the failing test, fixture or bats file, and nothing else.
2. `feat:` or `fix:` — the code that makes it pass.
3. `refactor:` — optional, behaviour unchanged.

`scripts/guard/tdd-order.sh` checks the order in history. Keep the mirrors
whole: every Rust file with logic has a sibling `tests.rs` wired with
`#[cfg(test)] mod tests;`, and every script under `scripts/` has a bats file
at the mirrored path under `tests/unit/`.

## Commits

- One logical change per commit.
- [Conventional Commits](https://www.conventionalcommits.org/) subject:
  `type(scope): summary`.
- A body that explains the change in prose, with a line starting `Why:`
  that cites the spec id it serves. The commit-msg hook refuses a body
  without one.
- Never name a private repository — in code, fixtures, docs or commit
  messages. Fixtures are synthetic or anonymised.

## The gate

```sh
hk check --all
```

That is the whole gate, defined once in [`hk.pkl`](hk.pkl) and run the same
way by the git hooks and by CI. It must be green before you push.

**Never use `--no-verify`**, on commit or on push. A failing step is either a
real problem in the change or a wrong step; a wrong step gets fixed in its
own commit, with its reason. Skipping it hides both.

The repository also checks itself: `xnl check` runs over this tree as a gate
step, so this repository may not contain the kind of embed the tool exists
to remove.

## Pull requests

- One change per pull request.
- Say which spec rows it serves and what you ran.
- After opening it, **stop and wait for review.** Do not stack further
  changes on an unreviewed one, merge it yourself, or push over review
  comments without answering them.
