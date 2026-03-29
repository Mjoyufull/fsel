# Code Standards

> A Manual for Writing Rust That Ages Well Instead of Exploding Into 1,000-Line Files

**Document Version:** 1.4.0  
**Last Updated:** 2026-03-24  
**Audience:** Future me, collaborators, contributors, and any poor bastard touching my code later  
**Scope:** Cross-project standards for Rust-first codebases, with some general engineering rules that apply anywhere

---

## Table of Contents

1. [Philosophy & Principles](#philosophy--principles)
2. [Scope & Use](#scope--use)
3. [Related References](#related-references)
4. [General Engineering Principles](#general-engineering-principles)
5. [Toolchain & Language Policy](#toolchain--language-policy)
6. [Project Layout & File Tree Standards](#project-layout--file-tree-standards)
7. [Module, File, and Function Size Standards](#module-file-and-function-size-standards)
8. [Code Style & Readability](#code-style--readability)
9. [API, Types, and Data Modeling](#api-types-and-data-modeling)
10. [Error Handling & Failure Policy](#error-handling--failure-policy)
11. [Testing Standards](#testing-standards)
12. [Documentation Standards](#documentation-standards)
13. [Architecture & Decision Records](#architecture--decision-records)
14. [Warnings, Lints, and Hygiene](#warnings-lints-and-hygiene)
15. [Change Size, Reviews, and Delivery](#change-size-reviews-and-delivery)
16. [Dependency Health, Features, and Workspaces](#dependency-health-features-and-workspaces)
17. [Performance, Allocation, and Concurrency](#performance-allocation-and-concurrency)
18. [Observability, Operability, and Incident Learning](#observability-operability-and-incident-learning)
19. [Unsafe, FFI, and Platform Boundaries](#unsafe-ffi-and-platform-boundaries)
20. [Code Review Checklist](#code-review-checklist)
21. [What Not To Do](#what-not-to-do)
22. [Example Commands & Automation](#example-commands--automation)
23. [Summary](#summary)

---

## Philosophy & Principles

### Core Tenets

1. **Clarity beats cleverness.** Code is for humans first. The compiler already understands nonsense.
2. **Make illegal states unrepresentable.** Push invariants into types, constructors, and module boundaries.
3. **Small surfaces win.** Thin APIs, thin modules, thin files, thin functions.
4. **Behavior first, implementation second.** Tests and docs should describe what the software promises, not only how it happens to work today.
5. **Warnings are debt, not wallpaper.** A warning that stays around becomes background noise, and then real problems hide inside it.
6. **Refactor before rot.** If a file is obviously getting bad, split it before it becomes a 1,500-line swamp.
7. **Prefer safe defaults.** Safe Rust first, safe APIs over unsafe internals, typed boundaries over stringly chaos.
8. **The standard library is the baseline.** Reach for external crates when they clearly earn their keep, not because cargo makes it easy.
9. **No hidden control flow.** Inputs, exits, allocations, and mutations should be visible where they matter.
10. **Correctness beats ergonomic sugar.** Convenience is good only when it preserves contracts, local reasoning, and honest semantics.

### Design Intent

These standards exist to keep code:

- easy to read after a month away
- easy to change without breakage
- easy to test without black magic
- easy to split into crates or workspaces when growth demands it
- aligned with current stable Rust practices, not stale habits from older codebases
- explicit about ownership, allocation, failure, and system boundaries
- hostile to ambient globals, shadowing, and APIs that hide cost behind cute names

This is a standards document, not a language tutorial.
If a rule needs to be broken, break it deliberately and document why.

---

## Scope & Use

This document is **not project-specific**.
It is the default coding standard for most of my Rust projects and for most general engineering work unless a repo explicitly documents different rules.

### Start Here

If you are touching the code day-to-day and do not need the whole handbook at once, start with:

- [Philosophy & Principles](#philosophy--principles)
- [Module, File, and Function Size Standards](#module-file-and-function-size-standards)
- [Code Style & Readability](#code-style--readability)
- [API, Types, and Data Modeling](#api-types-and-data-modeling)
- [Error Handling & Failure Policy](#error-handling--failure-policy)
- [Testing Standards](#testing-standards)
- [Warnings, Lints, and Hygiene](#warnings-lints-and-hygiene)
- [Code Review Checklist](#code-review-checklist)

Read the rest when the change touches architecture, delivery, operations, unsafe code, or release
behavior.

Use it for:

- application code
- library code
- CLI tools
- TUI/GUI tools
- services
- utilities
- scripts that grew up and became real software

If a project has both `PROJECT_STANDARDS.md` and `CODE_STANDARDS.md`, then:

- `PROJECT_STANDARDS.md` governs repo workflow, branching, releases, and contribution flow
- `CODE_STANDARDS.md` governs code structure, quality, testing, and implementation style

### Workflow Integration

This document is meant to be applied through the workflow defined in `PROJECT_STANDARDS.md`, not beside it.

In repos that use the `main` / `dev` / release-branch model:

- code changes follow the code branch and PR flow into `dev`
- documentation-only changes follow the docs flow into `main`
- release branches are for version bumps, release docs, and final verification, not surprise refactors
- hotfixes are minimal emergency exceptions, not a shortcut around normal review discipline

If you are the primary maintainer, these standards still apply.
Solo maintenance is not a reason to skip review thinking, testing, rollback planning, or release hygiene.

Not every section applies equally to every project.
The observability, rollout, and incident sections scale with the software:

- a small library or CLI still needs good errors, sane logging where relevant, and clear behavior
- a daemon, service, networked app, or long-running system needs the full operational treatment

---

## Related References

This document is based primarily on official Rust references plus the Rust API Guidelines.

- Rust Style Guide
- The Rust Programming Language
- The Rust Reference
- The Cargo Book
- The rustdoc book
- Clippy documentation
- Rust API Guidelines

Exact links are listed in the [Summary](#summary) section.

---

## General Engineering Principles

These are broader than Rust and should shape how systems are built, changed, and maintained.

### Simplicity Is a Reliability Feature

Complex systems fail in harder-to-debug ways.
Simplicity is not aesthetic minimalism; it is operational leverage.

Rules:

- prefer the simplest design that satisfies current requirements
- aggressively remove accidental complexity
- treat grab-bag abstractions as design debt
- do not confuse more layers with more architecture

If a simpler design gives the same business result, the simpler design wins.

### Prefer Boring, Proven Technology

"Boring" is a compliment in production engineering.

Prefer:

- stable tools over fashionable ones
- proven libraries over novelty crates
- clear control flow over clever machinery
- mature protocols and formats over custom inventions

Novelty is justified only when it clearly buys something important.

### Every New Line Is a Liability

New code creates:

- maintenance cost
- review cost
- test cost
- bug surface
- operational surface

Deletion is a feature.
If code is dead, gated forever, commented out, or replaced, remove it.
Source control remembers what was deleted.

### Optimize for Local Reasoning

An engineer should be able to understand a unit of code without loading the whole universe into their head.

Prefer:

- explicit inputs and outputs
- narrow interfaces
- isolated side effects
- single-purpose modules
- dependency direction that is easy to trace

Avoid:

- spooky action at a distance
- hidden global state
- helpers with invisible preconditions
- broad shared mutable state

### Prefer Explicit Boundaries Over Ambient Context

Important inputs should be visible in function signatures, constructors, and module APIs.

Prefer:

- passing config, clocks, RNGs, handles, paths, and capabilities explicitly
- reading environment variables, current directory, process args, and other ambient state once at
  the boundary, then passing typed values inward
- boundary modules that translate OS/process state into domain types

Avoid:

- deep business logic that reaches into process-global state
- thread-locals or singletons as invisible dependencies
- helpers that only work because of undocumented ambient setup

### No Hidden Control Flow or Work

Sugar is acceptable only when it is information-preserving and predictable.

Do not hide:

- allocation behind cheap-sounding APIs
- I/O, network access, or process termination behind innocent helpers
- mutation behind names that imply a read-only view
- major control flow behind macros or callback stacks unless the abstraction clearly earns it

When a convenience feature removes only tautological repetition and keeps semantics obvious, it is
fine. When it hides meaning, it is not.

### Prefer Reversible Decisions

Early decisions should be cheap to revisit.

Prefer:

- adapters over hard wiring
- versioned interfaces over lockstep rewrites
- configuration over forks
- migrations that can be rolled forward or back

When a choice is hard to reverse, document it and treat it as an architectural decision.

### Refactor in Small Safe Steps

Refactoring is not "big rewrite energy."
It is deliberate improvement in small behavior-preserving steps.

Rules:

- keep tests green while refactoring
- separate refactoring from feature work when possible
- do not combine whole-file formatting churn with behavioral changes
- prefer many safe steps over one dramatic step

If the system is broken for days, that is not refactoring. That is restructuring under risk.

---

## Toolchain & Language Policy

### Stable First

- Use stable Rust unless the project has a written reason not to.
- New projects should use the latest stable Rust edition.
- Existing projects should migrate editions intentionally, not accidentally.
- `rust-version` must be explicit in `Cargo.toml` if the project is meant to be shared, published, or maintained for more than a weekend.

### Edition Policy

- New projects default to `edition = "2024"` or the latest stable edition available.
- Existing projects should migrate using `cargo fix --edition`, then run tests, Clippy, and docs.
- Formatting should use the matching style edition via `rustfmt.toml`:

```toml
style_edition = "2024"
```

### Manifest Policy

At minimum, a serious project should have:

- explicit `edition`
- explicit `rust-version`
- description
- repository
- readme
- license

If the project is published or intended to be publishable, metadata should be complete enough for someone else to understand what the crate is and where to find its source/docs.

### Reproducible Builds

- CI and release verification should prefer `--locked`.
- Applications, binaries, and serious workspaces should commit `Cargo.lock`.
- Git dependencies should pin exact revisions or tags, not float on a branch name.
- Release artifacts should record the toolchain version, target triple, and build profile used to
  produce them.

---

## Project Layout & File Tree Standards

### Default Package Shape

For most non-trivial binaries:

```text
project/
  Cargo.toml
  rustfmt.toml
  README.md
  src/
    lib.rs
    main.rs
  tests/
```

Why:

- Cargo naturally supports a package with both `src/lib.rs` and `src/main.rs`
- keeping logic in `lib.rs` makes testing and reuse easier
- `main.rs` should mostly parse args, call into the library, map exit status, and print errors

### Thin `main.rs` Rule

If the binary is more than a toy:

- put real logic in `lib.rs`
- keep `main.rs` thin
- do not bury business logic in argument parsing or bootstrap glue

### Directory Layout by Responsibility

Prefer directories and modules that reflect domain boundaries, not vague buckets.

Good:

```text
src/
  config/
  cli/
  parser/
  ui/
  db/
  protocol/
```

Bad:

```text
src/
  utils.rs
  helpers.rs
  misc.rs
  stuff.rs
```

`utils.rs` is allowed only if it is truly small, generic, and stable. Most of the time it is a junk drawer and should be split by actual purpose.

### When to Create a New File

Create a new file when **one** of these is true:

- the file contains more than one responsibility
- the file is growing beyond the size limits below
- a concept has become important enough to deserve a name
- the tests for a concept would be clearer beside their own module
- the module has child modules and wants a stable top-level API
- code navigation is getting slower because everything lives in one place

Do **not** create a new file when:

- it would contain one tiny helper with no meaningful boundary
- it would force readers to jump through five files just to understand one function
- the split is by syntax only, not by responsibility

### How to Structure Modules

Preferred module style:

- use `foo.rs` for a module
- use `foo/` for the module's children when needed
- use `mod.rs` only when it is genuinely the cleanest fit or tool constraints require it

Examples:

```text
src/
  cli.rs
  cli/
    parse.rs
    help.rs
    validate.rs
```

or:

```text
src/
  cli/
    mod.rs
    parse.rs
    help.rs
```

Either is fine. Pick one style and stay consistent within the repo.

### When to Create a Workspace

Start with one package.
Split into a workspace only when there is a real boundary.

Create a workspace when:

- multiple crates have distinct responsibilities
- a library is reused by more than one binary
- build times or feature isolation justify separation
- different packages need separate publishing/versioning paths
- multiple packages should share one lockfile, target dir, lint policy, and CI surface

Do **not** create a workspace just because a single crate has become moderately large.
Large crates should first be fixed with better module boundaries.

---

## Module, File, and Function Size Standards

These are **engineering limits**, not language limits.
They exist to preserve readability.

### Soft Limits

| Unit | Soft Limit | Hard Smell Threshold |
|------|------------|----------------------|
| File | 250-350 lines | 500+ lines |
| Function | 20-40 lines | 80-100+ lines |
| `impl` block | One concern | Mixed unrelated behavior |
| Match arm | A few lines | mini-program inside each arm |

These are not mathematical laws.
A parser table or protocol state machine may need more room.
But once you cross the smell threshold, you need a conscious justification.

### File Budget Rules

- One file should usually hold one coherent concept.
- One file may contain multiple small helpers if they support the same concept.
- A file that mixes types, parsing, runtime orchestration, rendering, and tests is already wrong.
- A file over 500 lines must be considered guilty until proven innocent.

### Function Budget Rules

Break functions when:

- nesting gets deep
- control flow is hard to scan
- variable lifetimes become long and confusing
- more than one level of abstraction is mixed together
- the function both decides policy and performs mechanics

Good extractions:

- a named validation step
- a data normalization step
- a boundary call to filesystem/network/db
- a pure scoring/transformation helper
- a constructor or parser for a domain type

Bad extractions:

- `do_part_1`, `do_part_2`, `helper2`, `misc_step`
- helpers with names that reveal no domain meaning

### Line Width and Keeping Code Short

Rust's style guide sets the max line width at **100 characters**.
Follow it.

To keep width and length under control:

1. Prefer block indentation and trailing commas.
2. Use `where` clauses when bounds get long.
3. Name important intermediate values with `let`.
4. Use small structs or option types instead of too many parameters.
5. Use helper methods when a concept repeats.
6. Prefer early returns and guard clauses over nested pyramids.
7. Prefer `match`, `if let`, `let-else`, and small enums over boolean soup.
8. Split config/state bags into sub-structs by concern.

Do **not** chase short code by making it cryptic.
Senior code is concise because it is well-factored, not because it is compressed.

---

## Code Style & Readability

### Formatting

- Use `rustfmt`.
- Use the default Rust style unless the repo has a written exception.
- Use spaces, not tabs, ik kill me.
- Use 4-space indentation.
- Keep code lines at 100 chars max.
- Keep comment-only lines around 80 chars when practical.
- Prefer `///` doc comments and `//` line comments.
- Avoid block comments except for license text or generated content that tooling requires.

### Comments

Comments should explain:

- why a decision exists
- what invariant must hold
- what tradeoff is being made
- what is surprising or non-obvious

Comments should **not** narrate obvious syntax.
Comments should also **not** contain meta-commentary about the authoring process.
Do not write comments that explain what the AI was thinking, what the programmer debated,
or that talk directly to the user like prose in a chat window.
Prefer comment forms with clear attachment (`//!`, `///`, `//`) over large floating comment blocks.

Good:

```rust
// Keep the parsed form so repeated matches do not re-tokenize the input.
```

Bad:

```rust
// Increment i by 1.
i += 1;
```

### Names

Follow Rust naming conventions:

- modules: `snake_case`
- functions: `snake_case`
- methods: `snake_case`
- types and traits: `UpperCamelCase`
- constants/statics: `SCREAMING_SNAKE_CASE`

Additional standards:

- getters should usually be `name()` and `name_mut()`, not `get_name()`
- constructors should usually be `new` for the primary path
- if ownership or cost matters, names should say so:
  `as_*` for cheap borrowed views,
  `to_*` for copy/allocate,
  `into_*` for consuming conversion,
  `from_*` for construction from another representation,
  `with_*` for configured construction
- avoid names that hide allocation, mutation, or ownership transfer behind cheap-sounding verbs
- conversion traits should prefer `From`, `TryFrom`, `AsRef`, `AsMut`
- do not implement `Into` or `TryInto` directly when `From` / `TryFrom` is appropriate
- iterator methods should use `iter`, `iter_mut`, `into_iter`

### Readability Rules

Write code so a strong engineer can scan it top-to-bottom without mentally simulating a maze.

That means:

- one abstraction level at a time
- small and explicit data flows
- minimal hidden mutation
- obvious ownership
- names that reveal intent
- tight dependency locality
- no magic booleans when an enum or struct would be clearer
- no shadowing in maintained code; use a new name or a smaller scope when the value meaning changes

Prefer:

```rust
enum LaunchMode {
    Tty,
    Detached,
    Scoped,
}
```

over:

```rust
fn launch(item: &Item, detach: bool, tty: bool, scoped: bool)
```

### Idioms to Prefer

Prefer these when they improve clarity:

- `?` for error propagation
- `if let` and `let-else` for focused control flow
- `matches!` for boolean pattern checks
- `Iterator` adapters when they are clearer than manual loops
- explicit loops when iterator chains become unreadable
- small enums/newtypes for domain state
- tuple/struct returns instead of out-parameters

### Idioms to Avoid

- huge chained iterator pipelines that nobody can debug
- boolean trap arguments
- over-generic APIs with unreadable bounds
- macros used to avoid writing normal Rust
- exposing internal representation prematurely
- smart-pointer `Deref` tricks unless you are actually implementing a smart pointer
- shadowing bindings to smuggle in a state transition
- ambient globals, thread-locals, or process state as hidden inputs
- helper names that hide allocation, I/O, or mutation cost

---

## API, Types, and Data Modeling

### Public Surface Area

- Keep public APIs small.
- Make fields private by default.
- Expose behavior and invariants through methods and constructors.
- Public fields are acceptable for passive data structs, not for invariant-heavy types.

### Explicit Inputs Over Ambient Context

- Pass dependencies explicitly when they affect behavior.
- Read environment, time, process args, current directory, and similar ambient state at the shell
  or boundary layer, then pass typed values inward.
- Avoid APIs that quietly fetch global state from deep inside business logic.
- If a dependency must be ambient, document the contract and keep the boundary narrow.

### Encode Invariants in Types

Prefer:

- enums over free-form strings
- newtypes over naked primitives when units/meaning matter
- validated constructors over "trust me" structs
- distinct input types over comments pretending two `String`s mean different things

Example: `UserId(String)` is often better than "the first string argument is the user id."

### Type-State and Phase-Aware APIs

When an API has distinct phases, encode them in the type system when doing so keeps the API honest.

Good candidates:

- builders with required fields
- validated vs unvalidated configuration
- connected vs disconnected clients
- open vs closed resources
- state machines with legal transition rules

Use the type-state pattern when it removes whole classes of runtime misuse.
Do not force it onto tiny APIs where a simpler constructor or enum is clearer.

### Global State Policy

- Module-level mutable global state is banned by default.
- If shared global state is truly required, it must be synchronized, initialization-safe, wrapped
  in a tiny API, and justified in writing.
- Prefer explicit state objects, contexts, or handles over singletons and hidden lazy globals.

### Common Trait Policy

Types should eagerly implement the common traits that make sense:

- `Debug`
- `Clone`
- `Eq` / `PartialEq`
- `Ord` / `PartialOrd`
- `Hash`
- `Default`
- `Display`
- `Serialize` / `Deserialize` when the type is actually data

Do not derive or implement traits blindly.
Each trait should be semantically correct, not just convenient.

### Public API Evolution

If a crate is shared, published, or treated as a stable internal dependency:

- document the MSRV and bump it intentionally
- treat public APIs and feature flags as compatibility contracts
- use `#[must_use]` when dropping a return value is likely a bug
- consider `#[non_exhaustive]` for public enums and structs that are likely to grow
- run semver checks in CI for published crates or other semver-sensitive libraries

Do not make accidental breaking changes because "it was easy to refactor locally."

### Trait Bounds

- Put bounds where they are needed, not everywhere.
- Avoid trait bounds on struct definitions unless required by the data model.
- Prefer bounds on `impl` blocks or functions over the type declaration itself.

This preserves future flexibility and avoids over-constraining callers.

### Methods vs Functions

Use a method when the receiver is clear and central to the operation.
Use a free function when no receiver is privileged.

Prefer:

```rust
impl Config {
    pub fn validate(&self) -> Result<(), ConfigError> { ... }
}
```

over:

```rust
pub fn validate_config(config: &Config) -> Result<(), ConfigError> { ... }
```

unless there is a strong reason otherwise.

### Builders and Parameter Objects

Use a builder or parameter struct when:

- a constructor takes many arguments
- several arguments are optional
- boolean flags begin to pile up
- readability improves by naming fields at the call site

Do not use a builder for trivial two-field construction.

### Return Values

- Return values should carry meaningful information.
- If there are two or more logically-related outputs, return a tuple or struct.
- Do not use out-parameters unless interfacing with an existing API that requires them.

---

## Error Handling & Failure Policy

### Recoverable vs Unrecoverable

Rust distinguishes recoverable failure (`Result`) from unrecoverable failure (`panic!`).
Follow that distinction.

Use `Result` when:

- input can be invalid
- I/O can fail
- parsing can fail
- user/environment/config problems can occur
- the caller can reasonably decide what to do

Use `panic!` only when:

- an invariant is broken
- a state is impossible if the surrounding code is correct
- continuing would be nonsense
- the bug should be loud

### Library vs Binary Policy

Libraries:

- should almost never call `std::process::exit`
- should rarely panic for user/input/environment errors
- should return typed errors

Binaries:

- may convert top-level errors into exit codes
- may print user-facing error messages in the shell layer
- should keep that behavior near `main.rs`, not spread through helpers

### Termination Must Be Intentional

Choose deliberately between:

- normal return
- recoverable failure via `Result`
- invariant failure via `panic!`
- process exit in the top-level binary shell

Do not hide termination behavior in utility helpers, deep callbacks, or cleanup code.

### `unwrap` / `expect`

Rules:

- allowed in tests
- acceptable in tiny prototypes
- acceptable in startup code only when the invariant is truly hard
- not acceptable as normal error handling in maintained library code

If you use `expect`, the message must explain the invariant:

Good:

```rust
.expect("validated config always contains an output directory")
```

Bad:

```rust
.expect("oops")
```

### Error Types

Public error types should:

- implement `std::error::Error`
- preserve useful context
- not leak internal junk
- be precise enough to act on

Do not collapse everything into `String` at public boundaries unless the crate is intentionally tiny and private.

For error helpers:

- typed errors are preferred at library boundaries
- `thiserror` is a good fit for library/app error enums
- `anyhow`/`eyre`-style opaque errors are acceptable in top-level binary orchestration and one-off tools
- do not expose opaque catch-all error types as a public library contract unless that tradeoff is intentional

### Validate Early

Validate arguments and state as close to the boundary as possible.
Prefer static enforcement through types.
If static enforcement is not practical, validate once and convert to a validated type.

---

## Testing Standards

### Test Categories

Rust has three useful test layers. Use all three when appropriate.

1. **Unit tests**
   - live beside the code
   - may test private helpers
   - verify small logic and invariants

2. **Integration tests**
   - live under `tests/`
   - use only the public API
   - verify that components work together

3. **Documentation tests**
   - live in rustdoc examples
   - prove examples compile and run
   - keep docs honest

### Unit Test Rules

- Put focused unit tests beside the module they test.
- Test one behavior at a time.
- Use descriptive names:
  - `parses_empty_input_as_none`
  - `rejects_duplicate_keys`
  - `sorts_pinned_items_before_recent_items`
- Private functions may be tested if doing so meaningfully isolates behavior.

### Integration Test Rules

- Every non-trivial library should have `tests/`.
- Integration tests should exercise public behavior, not private structure.
- Each bug fix should add a regression test when practical.
- CLI and protocol projects should have at least a few black-box tests.

### Documentation Test Rules

- Public examples should compile.
- Prefer examples that demonstrate why the API exists, not only how syntax works.
- Fallible examples should use `?`, not `unwrap`.
- Use hidden lines in doc tests when setup is necessary but not relevant.

### Advanced Test Techniques

Use stronger tools when the surface area justifies them:

- property testing for invariants, parser round-trips, and algorithmic edge cases
- snapshot testing for CLI output, diagnostics, rendered text, or other stable human-facing output
- fuzzing for parsers, protocol handlers, file formats, and untrusted-input boundaries

Rules:

- review snapshots like code, not as magic blessed files
- keep property tests targeted enough to debug failures
- start fuzzing anywhere malformed input could become a crash, hang, or memory issue

### Test Structure

Use `Result`-returning tests when setup is fallible and `?` improves readability.

Use custom assertion messages when failure context matters.

Prefer:

```rust
assert!(
    rendered.contains("Carol"),
    "rendered output did not include the requested name: {rendered}"
);
```

over a failure that tells you nothing useful.

### What to Test

Test:

- domain invariants
- parsing and validation
- error behavior
- boundary conditions
- serialization round-trips
- sorting/ranking/scoring rules
- user-visible behavior
- regressions for previously fixed bugs

Do not over-invest in tests that pin trivial implementation details with no behavioral value.

### Test Data

- Keep fixtures small and readable.
- Put reusable fixtures under `tests/fixtures/`.
- Name fixtures after what they model, not where they came from.
- Use builders/helpers in tests when setup repetition obscures intent.
- Treat tests as executable behavior docs, not just breakage alarms.

---

## Documentation Standards

### Minimum

Every serious project should have:

1. `README.md`
2. additional user documentation when the project needs it
3. a license
4. crate/package metadata in `Cargo.toml`
5. public API docs if there is a public API

### Crate-Level Docs

Library crates should have crate-level docs that explain:

- what the crate is for
- when to use it
- the main entrypoints
- a minimal example

### Public Item Docs

Public items should be documented when they are part of an intended API.

Good docs include:

- a one-line summary first
- details after the summary
- an example when helpful
- `# Errors` when returning `Result`
- `# Panics` when panic behavior matters
- `# Safety` for unsafe functions

### Examples

Rules:

- examples must compile when practical
- examples should demonstrate real use
- prefer concise examples over giant tutorials
- keep examples synced by running tests

### Docs Lints

For library-ish projects, strongly consider:

- `missing_docs`
- `rustdoc::broken_intra_doc_links`
- `rustdoc::private_intra_doc_links`

Docs should not quietly rot.

---

## Architecture & Decision Records

Code alone does not preserve the reasoning behind important decisions.
For significant design choices, create a decision record.

### When to Write a Decision Record

Write a decision record for changes that affect:

- architecture or system shape
- dependencies and frameworks
- APIs and published contracts
- storage formats and migrations
- operational constraints
- security or compliance posture
- build, release, or development process

### Minimum Record Format

Every decision record should capture:

- context
- decision
- alternatives considered
- consequences and tradeoffs
- status
- owner

### Decision Record Rules

- keep records short and readable
- store them with the repo, typically under `docs/adr/` or `decisions/`
- accepted records are not silently rewritten; create a new one that supersedes the old one
- link code reviews and follow-up changes back to relevant records

ADRs are not bureaucracy.
They prevent the same arguments from being re-fought every three months.

---

## Warnings, Lints, and Hygiene

### Zero-Warning Policy

The target state is:

- no compiler warnings
- no Clippy warnings in CI
- no rustdoc warnings in CI

Warnings are either:

- fixed immediately
- intentionally suppressed in a narrow scope with a reason

They are not ignored.

### Lint Configuration

Prefer configuring lints centrally in `Cargo.toml` or workspace root.

Example:

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
dbg_macro = "deny"
todo = "deny"
```

For workspaces, prefer `workspace.lints` and opt member crates into it.

### `allow` vs `expect`

Use `#[allow(...)]` sparingly.

If a suppression is temporary and should disappear once the code changes, prefer `#[expect(...)]`.
That way the compiler tells you when the lint no longer fires and the suppression can be removed.

### Warning Removal Policy

When touching old code:

- remove nearby warnings while you are there if the fix is low-risk
- do not introduce new warnings
- do not leave "temporary" warning suppressions without explanation

When upgrading toolchains or editions:

- run `cargo fix`
- run `cargo fix --edition` for edition migrations
- rerun tests and Clippy after fixes land

### Hygiene Rules

- no commented-out dead code
- no debug prints left behind
- no `todo!()` or `unimplemented!()` in shipped paths
- no stale feature flags
- no unused dependencies
- no placeholder names like `data2`, `tmp3`, `thing`, `stuff`

---

## Change Size, Reviews, and Delivery

### Keep Changes Small

Thin code matters.
Thin diffs matter too.

Small changes are:

- reviewed faster
- reviewed more thoroughly
- easier to reason about
- easier to roll back
- less likely to collide with other work

Rules:

- one concern per change
- formatting-only changes separate from functional changes
- renames/moves separate from logic changes when practical
- large work should be staged through intermediate safe steps

### Improve Overall Code Health

A change does not need to be perfect.
It does need to leave the codebase better than it found it.

Do not block progress chasing theoretical perfection.
Do not accept changes that quietly worsen maintainability.

### Delivery Rules

- prefer smaller releases over giant batches
- make risky changes observable
- stage migrations whenever possible
- define rollback or roll-forward strategy before shipping risky infrastructure or data changes
- treat deployability as a feature, not an afterthought

### Mapping to the Repo Workflow

When a repo follows `PROJECT_STANDARDS.md`:

- feature, fix, refactor, and chore branches should carry small reviewable changes into `dev`
- docs-only updates to standards, guides, and examples should follow the docs-only path to `main`
- code standards changes that also touch CI, lints, toolchains, or build behavior are code/process changes and belong in the normal code flow
- release branches should only receive versioning, release-note, and final verification work
- hotfixes should stay narrow, then be followed by regression coverage on `dev`

### Review Behavior

Review should focus on:

- correctness
- design
- complexity
- tests
- naming
- documentation
- operational impact

Review comments should explain why, not just what.
Pure style comments that are not grounded in a documented standard should not block good changes.

---

## Dependency Health, Features, and Workspaces

### Dependency Policy

Before adding a crate, ask:

1. Does `std` already solve this well enough?
2. Is the crate maintained and widely used?
3. Is the compile-time / transitive dependency cost worth it?
4. Am I adding a crate to avoid writing 30 lines of clear code?
5. Does it lock the project into a design I may regret?

New dependencies should be justified in review, especially foundational ones.

### Dependency Health Policy

Dependencies are part of the codebase.
If they are insecure, stale, unmaintained, duplicated, or bloated, that is your problem too.

Regularly check for:

- known vulnerabilities
- unmaintained crates
- version drift
- duplicate transitive versions
- license/source issues where relevant
- binary size impact

Recommended tools:

- `cargo audit` for RustSec advisories, including some unmaintained notices
- `cargo outdated` for dependency drift
- `cargo tree -d` for duplicate versions
- `cargo deny check` for advisories, bans, licenses, and source policy
- `cargo bloat` for binary-size inspection

Treat "unmaintained" as a real engineering signal, not trivia.
If the standard library or a healthier crate now covers the same use case, prefer migrating.

### Maintenance Cadence

For maintained projects, run a regular hygiene pass rather than waiting for rot:

- update dependencies in a controlled branch
- review advisories and unmaintained notices
- inspect duplicate transitive versions
- prune dependencies and stale features
- re-check binary size when the project ships binaries

Quarterly is a good default for active projects.
More often is reasonable for security-sensitive or fast-moving repos.

### Feature Flag Policy

Cargo features should be:

- additive
- meaningful
- named after what they enable
- not used as a negative toggle like `no-x`

Good:

- `serde`
- `cli`
- `std`
- `image`

Bad:

- `use-serde`
- `with-cli`
- `no-std-support`

Features are for real optional capability, platform support, or expensive dependencies.
They are not a bandage for API instability.

### Published Crate Policy

For published crates or semver-sensitive internal libraries:

- document MSRV in the README and `Cargo.toml`
- bump MSRV intentionally and mention it in release notes when it changes
- check semver compatibility before release
- prefer additive evolution over surprise breakage
- do not expose dependencies in the public API unless you are willing to version with them

### Workspace Policy

Use a workspace when the package boundary is real.
When using a workspace:

- centralize lint policy
- centralize dependency versions when it reduces drift
- share one lockfile
- keep crate ownership boundaries clear

Do not hide a bad module structure by scattering everything into many tiny crates.

---

## Performance, Allocation, and Concurrency

### General Rule

Do not optimize blindly.
Measure first.

That said, some bad patterns are obvious and should be avoided by default:

- unnecessary `clone()`
- repeated parsing of the same input
- repeated allocation in tight loops
- needless `String` ownership when `&str` or `Cow` is enough
- converting back and forth between types without reason

### Allocation Rules

- Borrow when ownership is not needed.
- Use owned types when ownership clarifies lifetime and API behavior.
- Avoid sprinkling clones to silence the borrow checker.
- If the borrow checker is fighting you, the design may be wrong.

### Iterator Rules

- Use iterators when they improve clarity.
- Use loops when iterators get too dense.
- Do not turn simple control flow into unreadable combinator soup.

### Concurrency Rules

- Prefer ownership and message passing over shared mutable state where practical.
- Use shared state only when it is the clearest correct option.
- Keep lock scopes small.
- Never hold a lock across code that can block or across `.await`.
- Concurrency is not a substitute for better structure.

### Async and Blocking Policy

Async should be a small delta on top of otherwise normal Rust.

Rules:

- keep core business logic synchronous unless concurrency or I/O is the point
- use async primarily at I/O, protocol, and service boundaries
- do not perform blocking I/O or heavy CPU work on an async executor thread
- use `spawn_blocking`, dedicated worker threads, or separate processes when blocking work is unavoidable
- prefer bounded queues/channels and explicit backpressure over unbounded accumulation
- cancellation should leave state consistent and resources releasable
- test async code with the real runtime you expect to ship, not only mocked helpers

Choose runtime shape deliberately.
Do not add executor complexity or cross-runtime abstractions without a real need.

### Perf Review Trigger

If code is on a hot path and not obviously cheap:

- add a benchmark or measurement note
- record the reason for changes
- do not accept "felt faster" as evidence

### Benchmarking Rules

When performance matters:

- benchmark the code before and after a change
- benchmark representative workloads, not toy inputs only
- use release builds for meaningful measurements
- track tail behavior, not just averages, when latency matters
- use profilers when timing alone does not explain the result
- measure at the right layer:
  - microbenchmarks for local algorithm changes
  - integration/load tests for system behavior

Do not use microbenchmarks to justify system-level conclusions.
Do not merge "performance improvements" that have no measurements behind them.

### Performance Budgets

Where a system is latency-sensitive, size-sensitive, or resource-sensitive, define budgets.

Examples:

- startup time budget
- memory budget
- p95 or p99 latency budget
- binary size budget
- dependency count budget

Budgets do not need to be elaborate.
They do need to exist if performance or footprint is part of the product value.

### Size Awareness

Binary size, compile time, and dependency count are engineering concerns, not vanity metrics.

If a project ships binaries, periodically inspect:

- top crates by code size
- top functions by `.text` contribution
- whether a dependency is worth its size cost
- whether features can be disabled

Use `cargo bloat` to learn where size is actually coming from before guessing.
Use `cargo flamegraph` or an equivalent profiler when CPU cost needs attribution rather than intuition.

---

## Observability, Operability, and Incident Learning

Good engineering does not stop at code compiling.
It includes being able to run, observe, debug, and improve the system in reality.

Apply this section proportionally.
Do not cargo-cult service practices into a tiny crate, but do not use "it's just a tool" as an
excuse to ship software that is impossible to debug in real use.

### Observability Rules

For software that runs beyond a trivial local script:

- emit logs that are useful to humans and machines
- record enough context to debug novel failures
- expose metrics where latency, throughput, queue depth, error rate, or saturation matter
- add tracing or correlation identifiers when request flows cross boundaries

Monitoring tells you that something is wrong.
Observability should help you understand why.

### Minimum Operational Baseline

For services, daemons, APIs, queues, and other long-running systems, the minimum baseline is:

- logs
- health/readiness signal where applicable
- release/build identity
- latency measurement
- error measurement
- saturation/resource pressure measurement

For user-facing or networked systems, the default baseline should follow the four golden signals:

- latency
- traffic
- errors
- saturation

If the system is asynchronous, also track queue depth, backlog age, retries, and drop rate where relevant.

### Black-Box and White-Box Monitoring

Use both when the system matters.

- black-box monitoring tells you whether the user-visible surface works
- white-box monitoring tells you what the internals are doing

Black-box checks catch customer pain.
White-box signals speed up diagnosis.
Neither replaces the other.

### Logging Rules

- logs should be structured when the system is non-trivial
- do not log secrets
- do not bury useful fields inside giant free-form strings
- favor event-style logs over essay-style logs
- for service-style software, prefer writing to stdout/stderr and let the environment handle routing and storage
- log enough identifiers to correlate related events
- include version/build identity in startup logs for deployed software

### Metrics and Tracing Rules

- metrics should use stable names and clear units
- counters, gauges, and histograms should reflect real domain events
- latencies should usually be tracked as distributions, not just means
- traces should exist when a request crosses multiple boundaries and debugging would otherwise be guesswork
- do not emit high-cardinality labels blindly; cardinality is an operational cost

### Alerting Rules

- page only on conditions that are urgent, actionable, and user-visible or imminently user-visible
- every alert should have an owner
- every page-level alert should link to a runbook, dashboard, or both
- alerts that are routinely ignored, muted, or hand-waved are candidates for removal or redesign
- if nobody should wake up for it, it should not be a pager alert

Alerting noise is an engineering bug.

### Dashboard Rules

Every important system should have a default dashboard that answers:

- is it healthy right now?
- what changed recently?
- where is the failure surface?
- what version is running?

Dashboards are for fast orientation, not for cramming in every metric that exists.

### Config and Environment Rules

- deployment-specific config should not be hard-coded
- secrets do not belong in the repository
- configuration should be validated at startup
- development and production environments should be as close as practical for anything important
- distinguish build-time config from runtime config
- configuration should have clear ownership and defaults where appropriate
- invalid config should fail fast and loudly
- secrets should be rotatable without heroics
- effective configuration should be inspectable in a safe redacted form when debugging complex systems

### Release and Rollout Safety

Risky changes should be shipped in a way that limits blast radius.

Preferred tools:

- staged rollout
- canary deployment
- feature flags
- kill switches
- schema-first or compatibility-first rollout sequencing

Rules:

- release artifacts should be reproducible and traceable to source
- do not ship unique snowflake builds
- know how to roll back or roll forward before deployment starts
- deployment speed is good, but recoverability matters more

### Migrations and Compatibility

If a change touches persistent data, public APIs, on-disk state, or wire formats:

- define compatibility expectations explicitly
- write a migration plan
- document rollback constraints
- test against realistic old data or protocol examples
- separate irreversible migrations from unrelated feature work

When possible:

- deploy compatibility first
- migrate data second
- remove legacy support last

### Health Checks and Lifecycle Contracts

If a process exposes health or readiness checks:

- readiness should mean "safe to receive work"
- liveness should not mask brokenness as health
- dependency failures should be reflected honestly
- checks should be simple and reliable

Lifecycle contracts should be explicit:

- what must exist before startup succeeds
- what happens during degraded mode
- what guarantees are made during shutdown
- how long shutdown is allowed to take

### Startup and Shutdown Rules

Long-running programs should:

- start quickly
- fail fast when mandatory config is missing
- shut down gracefully
- handle termination signals predictably when the platform expects it
- stop accepting new work before teardown where applicable
- flush or drain critical buffers when required
- bound shutdown time with explicit timeouts
- make startup and shutdown behavior testable where practical

### Incident Learning

When a significant incident happens:

- write it down
- quantify impact
- identify contributing causes, not just the final trigger
- create concrete follow-up actions
- assign owners and priorities
- review and share the result

Postmortems should be blameless and system-focused.
The point is to improve the system and the process, not to shame the person closest to the blast.

### Incident Writeup Standards

A useful incident writeup should include:

- timeline
- customer or business impact
- detection method
- severity
- root cause
- contributing factors
- mitigation
- follow-up actions

Action items should have:

- an owner
- a priority
- a due date or tracking ticket

Repeated incidents without structural fixes are a process failure, not bad luck.

---

## Unsafe, FFI, and Platform Boundaries

### Unsafe Policy

Unsafe Rust is allowed only when it is justified.

Rules:

- keep unsafe blocks as small as possible
- isolate unsafe inside dedicated modules
- expose safe abstractions over unsafe internals whenever possible
- document every unsafe block with the invariant it relies on
- test unsafe-backed behavior aggressively
- prefer pure Rust for pure computation; FFI is for true boundary crossings, not laziness

### `// SAFETY:` Comments

Every non-trivial unsafe block should explain why it is sound.

Example shape:

```rust
// SAFETY: `ptr` comes from `buffer.as_ptr()`, remains valid for `len` bytes,
// and this function does not outlive `buffer`.
unsafe { ... }
```

### FFI Rules

- Keep FFI declarations localized.
- Prefer pure Rust when the problem is just computation, parsing, data structure work, or other
  logic that does not require a foreign boundary.
- Translate foreign types into Rust domain types at the boundary.
- Do not leak raw C concepts deep into business logic.
- If a function is unsafe to call, its docs must say exactly what the caller must uphold.

### Platform Rules

Platform-specific behavior belongs in platform modules.
Do not spread OS checks, `cfg`s, or syscall details across unrelated logic.

---

## Code Review Checklist

Before merging, ask:

### Correctness

- Does the code do what it claims?
- Are edge cases handled?
- Are invariants encoded or merely hoped for?

### Structure

- Is the file boundary sane?
- Is the function length sane?
- Is the abstraction level consistent?
- Is there a missing module split?
- Does async code avoid holding locks, guards, or broad mutable state across `.await`?

### API Quality

- Are names idiomatic?
- Are public fields necessary?
- Are common traits implemented where appropriate?
- Does the caller get a clear contract?

### Failure Handling

- Are recoverable errors returned, not panicked?
- Are panic cases documented where relevant?
- Is `unwrap`/`expect` justified?

### Tests

- Is there enough test coverage for the change?
- Does the change need a regression test?
- Are public-facing behaviors tested from the outside?

### Docs

- Are docs updated for public behavior changes?
- Do examples still compile?
- Are `Errors`, `Panics`, and `Safety` sections present where needed?

### Operational Impact

- Are logs, metrics, traces, and alerts adequate for the change?
- Does the change affect startup, shutdown, readiness, or health checks?
- Does it need a migration, rollout plan, or rollback plan?
- Is the release/build identity still traceable?

### Performance

- Is there a performance cost or gain worth measuring?
- Are benchmarks or measurements present where they should be?
- Does the dependency or feature choice carry a size or compile-time cost?

### Hygiene

- Any new warnings?
- Any broad lint suppressions?
- Any dead code, debug output, or TODOs left behind?

---

## What Not To Do

**Absolutely Forbidden**

- giant junk-drawer files because splitting "feels like work"
- pushing warnings downstream for later
- `std::process::exit` deep in library code
- public fields on invariant-heavy types without a strong reason
- boolean trap APIs
- blanket `allow` attributes to quiet code smells
- leaving failing or stale doc examples
- leaking unsafe/FFI concerns into normal business logic
- holding locks or guards across `.await` in maintained async code

**Strongly Discouraged**

- `utils.rs` as a permanent storage bin
- giant iterator chains that need a decoder ring
- macros for ordinary code reuse
- panic-driven control flow
- cargo-cult patterns copied from old blog posts
- premature workspace splits
- clever lifetimes used to avoid designing a cleaner ownership model

---

## Example Commands & Automation

### Daily Local Loop

```bash
cargo check
cargo test
```

### Workflow Fit

For repos following `PROJECT_STANDARDS.md`:

- run the local loop while developing on feature/fix/refactor/chore branches
- run the full PR checks before opening a code PR to `dev`
- use the docs-only flow for markdown-only standards or documentation updates
- rerun final verification on release branches even when no source code changed there

### Dependency and Size Health Loop

Run these regularly, not only when something is already broken:

```bash
cargo audit
cargo outdated --workspace
cargo tree -d
cargo bloat --release -n 20
cargo bloat --release --crates
```

If the repo cares about licenses, sources, duplicate crates, or advisory policy:

```bash
cargo deny check
```

For maintained shared crates, also consider:

```bash
cargo semver-checks check-release
```

### Locked Verification

If the repo commits `Cargo.lock`, use locked resolution for CI and release verification:

```bash
cargo build --locked
cargo test --locked
RUSTDOCFLAGS="-D warnings" cargo doc --locked --no-deps
```

### Before Opening a PR

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

### Warning and Edition Cleanup

```bash
cargo fix
cargo fix --edition
```

### Advanced Inspection

Use these when the problem deserves deeper tooling:

```bash
cargo flamegraph
cargo expand
cargo +nightly miri test
```

- `cargo flamegraph` for real CPU hotspots
- `cargo expand` for macro expansion inspection
- Miri for unsafe, aliasing, and low-level undefined-behavior checks

### Example `rustfmt.toml`

```toml
style_edition = "2024"
```

### Example `Cargo.toml` Lints

```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
dbg_macro = "deny"
todo = "deny"
unwrap_used = "deny"
```

Use stricter Clippy lints deliberately.
Do **not** enable all of `clippy::restriction` as a blanket rule.

### Example CI Shape

```yaml
name: CI
on: [push, pull_request]

jobs:
  checks:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          toolchain: stable
          components: rustfmt, clippy
      - run: cargo fmt --all -- --check
      - run: cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
      - run: cargo test --locked --workspace --all-features
      - run: cargo doc --locked --workspace --no-deps
        env:
          RUSTDOCFLAGS: -D warnings
```

---

## Summary

This standard boils down to this:

- use stable, current Rust
- keep `main.rs` thin
- keep files and functions small
- keep changes and releases small
- split by responsibility, not by accident
- prefer explicit inputs over ambient magic
- encode invariants in types
- use typestate when it removes real misuse
- keep names honest about ownership, allocation, and mutation cost
- return errors, do not panic casually
- make exits and failure modes intentional
- keep async at the boundary unless concurrency is the point
- test at unit, integration, and doc levels
- reach for property tests, snapshots, and fuzzing when the surface demands them
- document public behavior
- treat MSRV and public APIs as compatibility contracts
- record important technical decisions
- keep warnings at zero
- keep dependencies healthy
- choose boring, reversible designs when possible
- instrument systems so they can be debugged in reality
- learn from incidents without blame theater
- isolate unsafe and platform-specific code
- avoid shadowing and hidden global state
- add dependencies only when they earn their cost

Write code that future you can scan in one pass without cursing your own name.

### Standards Basis

Primary references used to shape this document:

- Rust Style Guide  
  https://doc.rust-lang.org/style-guide/
- Rust Edition Guide: rustfmt style edition  
  https://doc.rust-lang.org/edition-guide/rust-2024/rustfmt-style-edition.html
- The Rust Programming Language: packages, crates, and modules  
  https://doc.rust-lang.org/book/ch07-00-managing-growing-projects-with-packages-crates-and-modules.html
- The Rust Programming Language: error handling  
  https://doc.rust-lang.org/book/ch09-00-error-handling.html
- The Rust Programming Language: test organization  
  https://doc.rust-lang.org/book/ch11-03-test-organization.html
- The Rust Programming Language: writing tests  
  https://doc.rust-lang.org/book/ch11-01-writing-tests.html
- The Rust Programming Language: publishing and docs  
  https://doc.rust-lang.org/book/ch14-02-publishing-to-crates-io.html
- The Rust Programming Language: unsafe Rust  
  https://doc.rust-lang.org/book/ch20-01-unsafe-rust.html
- Cargo Book: manifest and lints  
  https://doc.rust-lang.org/cargo/reference/manifest.html
- Cargo Book: workspaces  
  https://doc.rust-lang.org/cargo/reference/workspaces.html
- Cargo Book: cargo fix  
  https://doc.rust-lang.org/cargo/commands/cargo-fix.html
- rustdoc book: how to write docs  
  https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html
- rustdoc book: documentation tests  
  https://doc.rust-lang.org/rustdoc/write-documentation/documentation-tests.html
- rustdoc book: lints  
  https://doc.rust-lang.org/rustdoc/lints.html
- Clippy documentation  
  https://doc.rust-lang.org/clippy/
- Rust API Guidelines  
  https://rust-lang.github.io/api-guidelines/
- Rust API Guidelines: naming  
  https://rust-lang.github.io/api-guidelines/naming.html
- Rust API Guidelines: documentation  
  https://rust-lang.github.io/api-guidelines/documentation.html
- Rust API Guidelines: interoperability  
  https://rust-lang.github.io/api-guidelines/interoperability.html
- Rust API Guidelines: predictability  
  https://rust-lang.github.io/api-guidelines/predictability.html
- Rust API Guidelines: flexibility  
  https://rust-lang.github.io/api-guidelines/flexibility.html
- Rust API Guidelines: type safety  
  https://rust-lang.github.io/api-guidelines/type-safety.html
- Rust API Guidelines: dependability  
  https://rust-lang.github.io/api-guidelines/dependability.html
- Rust API Guidelines: debuggability  
  https://rust-lang.github.io/api-guidelines/debuggability.html
- Rust API Guidelines: future proofing  
  https://rust-lang.github.io/api-guidelines/future-proofing.html
- Google Engineering Practices: code review standard  
  https://google.github.io/eng-practices/review/reviewer/standard.html
- Google Engineering Practices: what to look for in review  
  https://google.github.io/eng-practices/review/reviewer/looking-for.html
- Google Engineering Practices: small CLs  
  https://google.github.io/eng-practices/review/developer/small-cls.html
- Google SRE: simplicity  
  https://sre.google/sre-book/simplicity/
- Google SRE: monitoring distributed systems  
  https://sre.google/sre-book/monitoring-distributed-systems/
- Google SRE: postmortem culture  
  https://sre.google/sre-book/postmortem-culture/
- Google SRE: release engineering  
  https://sre.google/sre-book/release-engineering/
- AWS Well-Architected: operational excellence  
  https://docs.aws.amazon.com/wellarchitected/latest/framework/operational-excellence.html
- AWS Prescriptive Guidance: architectural decision records  
  https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/welcome.html
- AWS Prescriptive Guidance: ADR process  
  https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/adr-process.html
- AWS Prescriptive Guidance: ADR best practices  
  https://docs.aws.amazon.com/prescriptive-guidance/latest/architectural-decision-records/best-practices.html
- The Twelve-Factor App  
  https://12factor.net/
- The Twelve-Factor App: config  
  https://12factor.net/config
- The Twelve-Factor App: disposability  
  https://12factor.net/disposability
- The Twelve-Factor App: logs  
  https://12factor.net/logs
- Martin Fowler: definition of refactoring  
  https://martinfowler.com/bliki/DefinitionOfRefactoring.html
- Martin Fowler: code smell  
  https://martinfowler.com/bliki/CodeSmell.html
- Martin Fowler: opportunistic refactoring  
  https://martinfowler.com/bliki/OpportunisticRefactoring.html
- RustSec Advisory Database  
  https://rustsec.org/
- cargo-deny  
  https://github.com/EmbarkStudios/cargo-deny
- cargo-outdated  
  https://github.com/kbknapp/cargo-outdated
- cargo-bloat  
  https://github.com/RazrFalcon/cargo-bloat
- Tokio tutorial: shared state  
  https://tokio.rs/tokio/tutorial/shared-state
- cargo-semver-checks  
  https://github.com/obi1kenobi/cargo-semver-checks
- flamegraph / cargo flamegraph  
  https://github.com/flamegraph-rs/flamegraph
- cargo-fuzz  
  https://github.com/rust-fuzz/cargo-fuzz
- cargo-expand  
  https://github.com/dtolnay/cargo-expand
- Miri  
  https://github.com/rust-lang/miri
- proptest  
  https://github.com/proptest-rs/proptest
- insta  
  https://github.com/mitsuhiko/insta

---

End of Standard
