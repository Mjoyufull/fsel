# fsel Refactor Plan for Rust 2026

## Why this exists

`fsel` is not broken in the stupid obvious way.
It builds.
Tests pass.
Clippy is clean.
Docs build.

That is good.
It is also not the point.

The real problem is structural.
Too much of the code is still welded together in files that are too large, do too many jobs, and
make ordinary changes more annoying and more risky than they should be.

I wrote a lot of this before I was strict enough about file trees, module boundaries, cleanup
discipline, and explicit ownership.
This document is me fixing that on purpose instead of pretending the shape is fine because the
binary still compiles.

This is a refactor plan for bringing `fsel` into compliance with `CODE_STANDARDS.md` while keeping
the app usable the whole time.

## Current snapshot

Date: 2026-04-07

Toolchain observed:

- `rustc 1.94.0 (4a4ef493e 2026-03-02)`
- `stable-x86_64-unknown-linux-gnu`

Repo state observed now:

- `Cargo.toml` uses `edition = "2024"` and `rust-version = "1.94"`.
- `rustfmt.toml` sets `style_edition = "2024"`.
- CI uses stable Rust and locked verification.
- `cargo test --locked` passes.
- `cargo clippy --locked --all-targets --all-features -- -D warnings` passes.
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps` passes.
- `src/lib.rs` exists.
- `src/main.rs` is now a thin shell at 9 lines.
- terminal lifecycle is centralized in `src/ui/terminal.rs`.
- runtime and config path construction is centralized in `src/app/paths.rs`.
- `src/cli.rs` has been split into `src/cli/`.
- `src/config.rs` has been split into `src/config/`.
- ranking logic is extracted under `src/core/ranking/`.
- desktop parsing and discovery are split into `src/desktop/parse.rs` and `src/desktop/discover.rs`.
- `src/ui/dmenu_ui.rs` has been split into `src/ui/dmenu_ui/`.
- integration tests now exist under `tests/` (`tests/cli_behavior.rs` plus fixtures).
- there are no `std::process::exit(...)` calls remaining under `src/`.

So the refactor is not hypothetical anymore.
Some of the shell, terminal, and path work is already landed on the `refactor` branch.
The remaining problem is still the same one, though: the big files and mixed responsibilities are
still here.

This refactor should be judged by these outcomes:

- smaller and clearer modules
- fewer ambient assumptions
- less duplication
- more honest ownership boundaries
- better test surfaces
- easier future changes

## Workflow integration

This refactor is being done on one coordinated `refactor` branch.
Full stop.
I am not pretending this is going to be ten neat little long-lived refactor branches just so the
Git graph looks polite.

That does not lower the discipline.
It moves the discipline inside the branch.

Rules for this effort:

- keep the branch green
- move in explicit phases
- do not mix compatibility changes with unrelated cleanup unless the coupling is real
- update this plan when reality changes
- do not sneak refactor work into release branches
- do not use hotfix flow as a shortcut around the refactor

For repo workflow compliance:

- the `refactor` branch is the working branch for this effort
- release branches stay release-only
- hotfix branches stay hotfix-only
- docs-only changes can still take the docs path when that makes sense

The branch may be large.
That is fine.
The work inside it still needs to be testable, deliberate, and revertable in sane chunks.

## Evidence from the current codebase

### Size hotspots

`src` currently totals 12,347 lines.

Largest files right now:

- `src/modes/cclip/run.rs`: 1,338 lines
- `src/modes/app_launcher/run.rs`: 483 lines
- `src/common/item.rs`: 460 lines
- `src/config/env.rs`: 427 lines
- `src/core/ranking/query.rs`: 413 lines
- `src/modes/app_launcher/search.rs`: 409 lines
- `src/core/cache.rs`: 380 lines
- `src/cli/parse.rs`: 364 lines

That is still too much mass in too few files.
`main.rs` is fixed.
The actual maintenance traps are not.

### Architectural hotspots

1. `src/modes/cclip/run.rs`
   - still carries too much orchestration, tag flow, preview lifecycle, and event policy in one file
   - remains the largest hotspot by a wide margin

2. `src/modes/app_launcher/run.rs` and `src/modes/app_launcher/search.rs`
   - launcher responsibilities are split but still broad
   - startup/session policy and direct-launch behavior still need tighter boundaries

3. `src/core/state.rs`
   - ranking extraction has improved boundaries, but state policy is still denser than ideal
   - search-quality changes are safer than before, but further isolation is still useful

4. `src/config/env.rs`
   - environment override policy is now isolated but still large
   - merge/normalization boundaries should continue to tighten

5. `src/common/item.rs`
   - now one of the larger remaining files
   - likely still carrying too many representational concerns in one module

6. `src/process.rs`
   - process behavior is cleaner but still not yet under a dedicated `platform/` boundary

### Duplication and policy drift

Some drift is already fixed:

- `main.rs` is thin
- terminal setup is centralized
- path construction is centralized
- CLI/config monoliths are split into module trees
- ranking logic has a dedicated module surface
- desktop parsing/discovery split has landed

What still needs work:

- launcher runtime is still too broad
- cclip run loop is still too large
- process/platform boundaries are not fully isolated
- desktop cache/model boundaries are not fully split
- ADR docs for architectural decisions are still missing

### Test coverage signal

Current tests are real but still thin:

- unit tests exist across modules
- integration tests now exist under `tests/`
- the crate now has a library target, which removes the old excuse for not adding better
  black-box and integration coverage

## What "Rust standards of 2026" means here

As of 2026-04-07, the latest stable Rust edition is still 2024.
So for this repo, "Rust standards of 2026" does not mean chasing made-up future language magic.
It means using stable Rust properly and writing the code like a grown engineer instead of like a
guy hoping the compiler will forgive the structure.

For this repo, that means:

1. Use the current stable edition and formatting model.
2. Keep toolchain policy explicit and verified.
3. Use a normal library-backed crate layout with a thin binary shell.
4. Keep startup, path, lock, and process behavior behind explicit boundaries.
5. Prefer typed config and typed errors over string parsing and deep exits.
6. Keep unsafe and platform-specific code tiny and obvious.
7. Refactor in controlled phases even if the work is happening on one branch.

This is not a rewrite.
This is a teardown of bad structure and a rebuild of good structure while preserving behavior by
default.

## Decision record status

This plan makes real architectural calls.
Per `CODE_STANDARDS.md`, those are ADR-grade decisions whether I like paperwork or not.

Minimum ADR set for this refactor:

- library-backed crate structure with thin `main.rs`
- target module topology and file tree
- typed config and merge precedence policy
- path, lock, and process centralization policy
- any persistence or storage-compatibility decision that affects on-disk behavior

The ADRs do not need to be novels.
They do need to capture:

- context
- decision
- alternatives considered
- compatibility or migration impact
- rollback constraints

If the plan changes materially, I should record the change instead of silently letting the codebase
drift and pretending that was always the plan.

## Target standards

### Manifest and toolchain

This part is mostly already in place on the branch, and it stays part of the contract:

- keep `edition = "2024"`
- keep `rust-version = "1.94"` unless there is a deliberate policy change
- keep `rustfmt.toml` with `style_edition = "2024"`
- treat `Cargo.lock` as part of the reproducible-build contract
- keep CI and release verification on locked resolution
- keep toolchain policy explicit in docs, not implied by luck

### Lint policy

Standards here are simple:

- formatting clean
- Clippy clean on all targets
- rustdoc warnings treated as real warnings
- no random `allow(...)` creep
- no business-logic `unsafe`

Practical repo policy:

- keep manifest-level lint config in `Cargo.toml`
- keep `cargo clippy --locked --all-targets --all-features -- -D warnings`
- keep `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`
- if an `allow` exists, it should be narrow and justified

### Module size and ownership rules

Hard rules for this refactor:

- `src/main.rs` stays a thin shell
- no file over 500 lines when the refactor is done
- most files should land around 150-350 lines
- any function around 80-120 lines is suspicious by default
- no `std::process::exit` outside shell-level code
- no `ProjectDirs::from(...)` outside dedicated path code
- no terminal raw-mode / alternate-screen setup outside shared terminal code
- no unsafe outside dedicated process/platform code, with a short safety comment every time

### Compatibility and rollout rules

This refactor is allowed to move code aggressively.
It is not allowed to create silent behavioral drift and then hand-wave it as cleanup.

Unless a phase explicitly says otherwise, preserve:

- current CLI flag names and top-level mode behavior
- current config precedence and semantics
- current database path and on-disk compatibility
- current lock-file behavior unless the replacement includes compatibility handling
- current dmenu and cclip user-visible behavior

If a phase must break compatibility, it needs:

- an ADR or equivalent decision note
- a migration note
- a rollback or roll-forward note
- a regression test or fixture that proves the intended new behavior

Rollout rule for risky work:

- compatibility-first changes land before cleanup-only removal
- irreversible storage or config changes do not get bundled with unrelated cleanup
- one branch does not mean one giant uncontrolled jump

### Dependency and footprint rules

Refactor work does not get to ignore dependency health or binary size.
Check these regularly during the branch:

- `cargo audit`
- `cargo outdated --workspace`
- `cargo tree -d`
- `cargo bloat --release --crates`
- `cargo bloat --release -n 20`

The goal is:

- no newly introduced vulnerable or obviously dead dependencies without a reason
- no accidental duplicate dependency mess
- no major size regression without a reason

### Test strategy and evidence

This repo has enough parser-heavy and user-visible behavior that plain unit tests are not enough by
themselves.

During this refactor, use:

- unit tests for pure logic and invariants
- integration and black-box tests for CLI and mode behavior
- snapshot-style tests for stable help text and other intentional text output
- property or fuzz-style coverage where malformed input is a realistic risk

High-value targets:

- CLI help and error text snapshots
- config parsing and merge precedence
- dmenu item parsing
- desktop entry parsing
- ranking invariants

## Target file tree

This is the end-state shape I am aiming for.
The exact names can move a little.
What matters is the boundary, not me fetishizing folder names.

```text
src/
  lib.rs
  main.rs
  app/
    mod.rs
    dispatch.rs
    paths.rs
    lock.rs
    runtime.rs
  cli/
    mod.rs
    args.rs
    help.rs
    parse.rs
    validate.rs
    color.rs
  config/
    mod.rs
    schema.rs
    defaults.rs
    file.rs
    env.rs
    merge.rs
  core/
    mod.rs
    state.rs
    ranking.rs
    database.rs
    history.rs
  desktop/
    mod.rs
    discover.rs
    parser.rs
    cache.rs
    model.rs
  modes/
    mod.rs
    launcher/
      mod.rs
      run.rs
      direct.rs
    dmenu/
      mod.rs
      run.rs
      io.rs
      render.rs
      selection.rs
    cclip/
      mod.rs
      run.rs
      commands.rs
      preview.rs
      tags.rs
      model.rs
  ui/
    mod.rs
    terminal.rs
    input.rs
    layout.rs
    theme.rs
    list.rs
  platform/
    mod.rs
    process.rs
  strings.rs

tests/
  cli_help.rs
  launcher_smoke.rs
  dmenu_smoke.rs
  cclip_smoke.rs
  config_merge.rs
  ranking.rs
```

## Refactor priorities

### Priority 1: finish the library-backed app shell

Why:

- `main.rs` is fixed, but launcher and cclip bootstrap are still too heavy higher up the tree
- the shell needs to be explicit, thin, and boring

Do this:

- keep `main.rs` as nothing but top-level error handling
- keep moving app dispatch/bootstrap into `app/`
- stop depending on crate-root glue for runtime policy

Result:

- application entry stays obvious
- future integration testing gets easier
- shell code stops leaking everywhere

### Priority 2: split CLI parsing from config loading and validation

Why:

- the split from `src/cli.rs` landed, but policy is still unevenly distributed
- parsing, validation, config mapping, and help behavior are cleaner, but still need polish

Do this:

- separate argument types, parser, help, validation, and color parsing
- return typed errors instead of deep exits
- move help/version display policy to the shell layer

Result:

- CLI becomes testable and boring
- config merge logic stops being hidden inside a giant parser file

### Priority 3: replace stringly typed config with typed schema

Why:

- strings like `"fuzzy"`, `"ranking"`, and `"top"` are still a bad contract
- invalid values still have too much room to degrade behavior silently

Do this:

- use typed enums for mode and layout settings
- keep config precedence explicit: defaults, file, env, CLI
- parse colors once at the config boundary instead of all over the place

Result:

- fewer hidden config bugs
- fewer fallback weirdness paths
- clearer merge and validation logic

### Priority 4: make ranking pure

Why:

- ranking is still buried too close to state mutation
- changing search quality still risks unintended side effects

Do this:

- extract pure ranking logic
- make one place own normalization, tiering, scoring, and breakdown generation
- leave `State` responsible for state, not ranking policy

Result:

- ranking can be tested directly
- performance work gets safer later

### Priority 5: finish extracting shared TUI infrastructure

Why:

- terminal setup is centralized now, but layout/render infrastructure still is not where it should be
- dmenu and cclip still each carry too much TUI-specific policy

Do this:

- move duplicated layout helpers into shared UI code
- keep shared terminal/session behavior in one place
- pull common TUI behavior out of mode files where it truly is shared

Result:

- less copy-paste
- smaller mode files
- cleaner render and interaction boundaries

### Priority 6: isolate process, path, and lock behavior fully

Why:

- paths are centralized now, but lock/session behavior is not fully centralized yet
- launcher still owns too much singleton and startup policy

Do this:

- keep path policy in one place
- extract lock/session behavior into dedicated code
- keep process checks and signals inside explicit wrappers

Result:

- no more startup policy smeared through launcher code
- platform behavior stops polluting mode code

### Priority 7: split desktop parsing from desktop discovery

Why:

- desktop has been split, but cache/model boundaries are still incomplete

Do this:

- separate discovery, parsing, cache, and model logic
- keep parser behavior testable on its own

Result:

- parser bugs become isolated
- discovery and cache behavior become independently tunable

## Suggested phase plan

These phases are checkpoints inside the one `refactor` branch.
They are not a promise of separate feature branches.

### Phase 0: freeze behavior and create baselines

Status: partly done, keep refreshing it

Do:

- capture CLI help output
- capture ranking fixtures
- capture dmenu item parsing fixtures
- capture cclip tag behavior
- record startup and error-path behavior for launcher, dmenu, and cclip
- baseline dependency health and size
- write the initial ADRs this plan depends on

Deliverables:

- fixtures under `tests/fixtures/`
- first real integration-test scaffold
- first snapshot baseline for stable text output
- dependency and size baseline notes
- ADR stubs

### Phase 1: edition/toolchain upgrade

Status: done

Done:

- Rust 2024 edition
- `rustfmt` style edition 2024
- stable CI
- locked verification
- rustdoc verification

Acceptance:

- `cargo test --locked`
- `cargo clippy --locked --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps`

### Phase 2: introduce `lib.rs` and move the app shell out of `main.rs`

Status: largely done

Done:

- `src/lib.rs` exists
- `main.rs` is thin
- app shell work has started in `app/`
- top-level shell/error behavior is centralized

Still left:

- finish shrinking launcher bootstrap/runtime responsibilities out of `modes/app_launcher/run.rs`

Acceptance:

- `main.rs` has no business logic
- top-level CLI behavior stays stable
- app dispatch/bootstrap is explicit and easy to find

### Phase 3: CLI/config split

Status: largely done, polishing remains

Do:

- finish hardening the `src/cli/` split
- finish hardening the `src/config/` split
- remove deep exit behavior
- introduce typed config and validation errors

Acceptance:

- parsing and config merge have focused tests
- malformed-input coverage improves
- existing config files still load unless a documented migration says otherwise

### Phase 4: launcher domain split

Status: started and materially progressed

Do:

- pull ranking/history/model logic out of `core/state.rs`
- reduce launcher runtime responsibilities
- make ranking pure

Acceptance:

- ranking behavior is fixture-backed
- `core/state.rs` gets materially smaller
- behavior drift is intentional and documented if it happens

### Phase 5: TUI infrastructure extraction

Status: started, not finished

Done:

- shared terminal lifecycle exists
- shared panel-layout helpers exist

Still left:

- shared render utilities where they are truly shared

Acceptance:

- no duplicate terminal setup paths
- no duplicate `effective_content_height`
- startup and shutdown behavior stays stable

### Phase 6: mode decomposition

Status: started, not finished

Do:

- split `dmenu`
- split `cclip`
- keep run, render, and command logic separate

Progress:

- `dmenu` has been split into dedicated modules (`events`, `options`, `render`, `parse`)
- `cclip` has partial decomposition (`commands`, `items`, `state`, `session`)
- `cclip/run.rs` is still the primary oversized runner and remains the biggest mode-level task

Acceptance:

- no mode runner above roughly 400-500 lines
- render code and command logic can be tested more directly
- dmenu and cclip behavior remains fixture-backed

### Phase 7: desktop and platform split

Status: started, not finished

Do:

- split `desktop/app.rs`
- finish centralizing paths, locks, and process wrappers
- keep compatibility unless a migration note says otherwise

Progress:

- `desktop/app.rs` has been removed in favor of `desktop/parse.rs` and `desktop/discover.rs`
- path policy is centralized in `app/paths.rs`
- launcher and cclip lock/session ownership now have dedicated session modules
- process behavior still needs a dedicated `platform/` module boundary

Acceptance:

- desktop parse, discover, cache, and model logic live in separate files
- path and lock policy comes from one place
- rollback constraints are documented for any persistence or path change

## Concrete rules for this repo

These are the rules I am enforcing during this refactor:

1. No new file over 400 lines without a written reason.
2. No new helper may call `std::process::exit`.
3. New config fields should be typed, not raw `String`, unless the domain is actually free-form.
4. New ranking behavior needs fixture-based tests.
5. Any new duplication between dmenu and cclip counts as failure, not progress.
6. Unsafe does not belong in UI, config, ranking, or mode orchestration code.
7. Paths, locks, and process handling belong in dedicated modules only.
8. Any compatibility break gets documented before it lands.
9. One branch does not mean no discipline.

## Done criteria

This refactor is done when all of this is true:

- the repo is fully on Rust 2024 policy and keeps passing locked verification
- `main.rs` is thin and stays thin
- the biggest hotspot files have been split
- no file exceeds 500 lines
- there is a real `tests/` integration suite
- terminal setup is centralized
- path and lock logic are centralized
- ranking is pure and tested
- mode orchestration is separate from rendering and command handling
- dependency and size baselines have been rechecked
- parser-heavy and text-output-heavy behavior has stronger regression coverage
- config, path, lock, and persistence compatibility policy is documented
- ADRs exist for the architectural decisions that reshaped the crate

## What not to do

Do not:

- treat the `refactor` branch like an excuse to go feral
- change behavior and structure at the same time with no tests
- keep slapping `allow(...)` on things just to keep moving
- preserve bad file boundaries out of nostalgia
- optimize ranking performance before the ranking logic is isolated
- hide compatibility breaks inside "cleanup"

## First implementation steps

From the current branch state, the next order is:

1. Keep the branch green at all times.
2. Finish splitting `modes/cclip/run.rs` into focused event/action/render helpers.
3. Continue shrinking `modes/app_launcher/run.rs` and `modes/app_launcher/search.rs`.
4. Continue ranking/state boundary cleanup and add stronger ranking fixtures.
5. Finish process/platform isolation (`platform/process.rs` shape) and remove remaining scatter.
6. Continue desktop decomposition by separating cache/model concerns.
7. Expand integration and snapshot coverage (help text, config merge, mode behavior).
8. Add ADR stubs for the architectural decisions already made on this branch.

## References

Official Rust references used for this plan:

- Rust 2024 Edition Guide:
  https://doc.rust-lang.org/edition-guide/rust-2024/index.html
- Transitioning an existing project to a new edition:
  https://doc.rust-lang.org/stable/edition-guide/editions/transitioning-an-existing-project-to-a-new-edition.html
- Cargo: Rust-version aware resolver:
  https://doc.rust-lang.org/stable/edition-guide/rust-2024/cargo-resolver.html
- Rustfmt style edition 2024:
  https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html
- Cargo `rust-version` reference:
  https://doc.rust-lang.org/cargo/reference/rust-version.html
- Cargo manifest `[lints]` reference:
  https://doc.rust-lang.org/cargo/reference/manifest.html#the-lints-section
- Cargo target/layout conventions:
  https://doc.rust-lang.org/stable/cargo/reference/cargo-targets.html

Local repo evidence used for this plan:

- `Cargo.toml`
- `rustfmt.toml`
- `.github/workflows/rust.yml`
- `src/main.rs`
- `src/lib.rs`
- `src/app/mod.rs`
- `src/app/paths.rs`
- `src/cli/mod.rs`
- `src/config/mod.rs`
- `src/core/state.rs`
- `src/core/ranking/mod.rs`
- `src/modes/app_launcher/run.rs`
- `src/modes/dmenu/run.rs`
- `src/modes/cclip/run.rs`
- `src/desktop/parse.rs`
- `src/desktop/discover.rs`
