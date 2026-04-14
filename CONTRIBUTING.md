# Contributing to fsel

fsel is hosted at `https://github.com/Mjoyufull/fsel/`.

This file is the contributor guide. It is not the maintainer workflow manual.
Maintainers are expected to already know and follow both `PROJECT_STANDARDS.md` and
`CODE_STANDARDS.md` in full.

## Read This First

Before opening a code PR, read:

- [CONTRIBUTING.md](./CONTRIBUTING.md) for the contributor workflow in this repo
- [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for branch and PR rules
- [CODE_STANDARDS.md](./CODE_STANDARDS.md) for implementation standards

If you are changing Rust code, reading and following `CODE_STANDARDS.md` is required.

## Where To Talk

Use GitHub issues and pull requests:

- Bugs: [Issues](https://github.com/Mjoyufull/fsel/issues)
- Code and docs: [Pull requests](https://github.com/Mjoyufull/fsel/pulls)

## Quick Rules

- Code branches from `dev` and PRs back to `dev`.
- Docs-only branches from `main` and PRs back to `main`.
- If a `feat/*` or `fix/*` changes user-visible behavior, update the relevant user docs in the same PR.
- Keep changes focused. Do not mix unrelated refactors into a bug fix or feature PR.
- Enable "Allow edits by maintainers" on your PR.

## Setup

fsel currently targets stable Rust `1.90+`.

```sh
git clone https://github.com/YOUR_USERNAME/fsel.git
cd fsel
git remote add upstream https://github.com/Mjoyufull/fsel.git

cargo run -- --help
```

If you have direct access and do not need a fork:

```sh
git clone https://github.com/Mjoyufull/fsel.git
cd fsel
```

## Current Repo Map

The current `main` branch is still fairly flat. Do not assume older refactor trees match this branch.

- `src/main.rs`: entry point, mode routing, top-level terminal setup/shutdown
- `src/cli.rs`: CLI parsing and runtime option shaping
- `src/config.rs`: config structs, defaults, and config/env loading
- `src/core/`: cache, database, debug logging, shared runtime state
- `src/desktop/`: desktop entry parsing and application discovery
- `src/modes/app_launcher/`: main launcher behavior, search, run, launch flow
- `src/modes/cclip/`: clipboard mode
- `src/modes/dmenu/`: dmenu mode
- `src/ui/`: TUI state, rendering, input, keybinds, graphics helpers
- `src/process.rs`: process and lockfile-related helpers
- `src/strings.rs`: shared string parsing helpers

## Module Responsibility Notes

Keep changes near the boundary that owns the behavior:

- CLI flags, parsing, and help text belong in `src/cli.rs`
- Config file and env override behavior belong in `src/config.rs`
- `.desktop` parsing and discovery belong in `src/desktop/`
- Mode-specific behavior belongs under that mode in `src/modes/`
- Shared UI behavior belongs in `src/ui/`
- Cross-cutting helpers should stay small and justified

If your change starts spreading across many unrelated modules, stop and tighten the scope.

## Branch Flow

### Code Changes

Create code branches from `dev`:

```sh
git fetch upstream
git checkout dev
git merge upstream/dev
git checkout -b feat/your-change
```

Use `feat/`, `fix/`, `refactor/`, or `chore/` as appropriate.

Before opening the PR:

```sh
git fetch upstream
git rebase upstream/dev
cargo fmt --all
cargo test --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
```

Then push and open a PR targeting `Mjoyufull/fsel:dev`.

### Docs-Only Changes

Docs-only branches come from `main`:

```sh
git fetch upstream
git checkout main
git merge upstream/main
git checkout -b docs/your-doc-change
```

Then push and open a PR targeting `Mjoyufull/fsel:main`.

Docs-only means the change is documentation and doc assets only. If the change ships new user-visible behavior from a feature or fix, update the docs in that code PR instead of splitting them out.

## User Docs To Update

When behavior changes for users, update whichever of these apply:

- [README.md](./README.md)
- [USAGE.md](./USAGE.md)
- [config.toml](./config.toml)
- [fsel.1](./fsel.1)

Do not leave user-facing behavior changes undocumented.

## Code Expectations

Read `CODE_STANDARDS.md` before touching code. A few rules matter constantly here:

- Prefer small, focused modules and functions.
- Keep interfaces explicit and honest about side effects.
- Avoid "boolean trap" APIs and ambiguous helper signatures.
- Handle errors deliberately; do not silently swallow failures without a reason.
- Add or update tests when behavior changes.
- Do not rewrite unrelated parts of the repo just because you are already in the file.

## Pull Requests

Keep PRs reviewable and concrete.

PRs should include:

- what changed
- why it changed
- what you tested
- linked issues when relevant

Use a draft PR if you want early direction before the implementation is done.

Conventional commit style for PR titles and final commits is preferred:

```text
feat(scope): short description
fix(scope): short description
docs(scope): short description
refactor(scope): short description
```

## Need Help?

If you are unsure where a change belongs or whether it fits the project, open an issue or a draft PR.
That is better than guessing and sending a large off-target patch.
