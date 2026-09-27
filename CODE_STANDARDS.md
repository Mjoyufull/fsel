# Code Standards

> A manual for writing Rust that ages well instead of exploding into 1,000-line files.
> Opinionated on purpose. If you wanted a document that agrees with every blog post, go read a blog post.

**Document Version:** 2.0.0  
**Last Updated:** 2026-08-26  
**Audience:** Future me, collaborators, contributors, AI agents writing code in my repos, and any poor bastard touching my code later  
**Scope:** Cross-project standards for Rust-first codebases, with general engineering rules that apply anywhere

---

## Read This First

This is not a suggestion box and it is not "one perspective among many." In my repos it outranks your habits, the Rust Book's teaching examples, whatever the popular crate does, and whatever an AI agent decided was idiomatic five seconds ago.

Three things have to land before any of the rules make sense.

**Compiling is not an achievement.** `rustc` accepting your code means you avoided a small, well-defined class of memory bugs. It says nothing about whether the code is correct, whether it leaks processes on cancellation, whether it redraws the screen sixty times a second while idle, whether it allocates a megabyte to answer a yes/no question, or whether anyone can read it in six months. Passing the borrow checker is the entry fee, not the prize.

**Idiomatic is not an argument.** "This is how Rust is written" is a description of a population, not a justification. Most Rust in the wild is over-abstracted, macro-poisoned, dependency-bloated, and slow to compile, and much of it is written by people cargo-culting patterns they never evaluated. If your defense of a design is that a book or a popular crate does it that way, you have not defended it. Tell me what it costs and what it buys.

**Effort is not quality.** A 900-line PR that took you all weekend and reinvents something already sitting in `Cargo.lock` is worse than the 12-line diff that deletes it. I do not grade on sweat. Nobody does.

The rest of this document is what I actually want instead.

---

## Table of Contents

1. [Philosophy & Principles](#philosophy--principles)
2. [How to Think About Code](#how-to-think-about-code)
3. [The Explicitness Doctrine](#the-explicitness-doctrine)
4. [Scope & Use](#scope--use)
5. [Related References](#related-references)
6. [General Engineering Principles](#general-engineering-principles)
7. [Toolchain & Language Policy](#toolchain--language-policy)
8. [Compile Time Is a Feature](#compile-time-is-a-feature)
9. [Project Layout & File Tree Standards](#project-layout--file-tree-standards)
10. [Module, File, and Function Size Standards](#module-file-and-function-size-standards)
11. [Code Style & Readability](#code-style--readability)
12. [Macros, Abstraction, and Invented Machinery](#macros-abstraction-and-invented-machinery)
13. [API, Types, and Data Modeling](#api-types-and-data-modeling)
14. [Error Handling & Failure Policy](#error-handling--failure-policy)
15. [Testing Standards](#testing-standards)
16. [Documentation Standards](#documentation-standards)
17. [Architecture & Decision Records](#architecture--decision-records)
18. [Warnings, Lints, and Hygiene](#warnings-lints-and-hygiene)
19. [Change Size, Reviews, and Delivery](#change-size-reviews-and-delivery)
20. [Review Standard](#review-standard)
21. [Dependency Health, Features, and Workspaces](#dependency-health-features-and-workspaces)
22. [Performance, Allocation, and Cost](#performance-allocation-and-cost)
23. [Async, Concurrency, and Cancellation](#async-concurrency-and-cancellation)
24. [Defect Classes That Keep Shipping](#defect-classes-that-keep-shipping)
25. [Observability, Operability, and Incident Learning](#observability-operability-and-incident-learning)
26. [Unsafe, FFI, and Platform Boundaries](#unsafe-ffi-and-platform-boundaries)
27. [Reading Real Code: Worked Examples](#reading-real-code-worked-examples)
28. [Code Review Checklist](#code-review-checklist)
29. [What Not To Do](#what-not-to-do)
30. [Example Commands & Automation](#example-commands--automation)
31. [Summary](#summary)

---

## Philosophy & Principles

### The Laws

These are not tips. When a rule later in this document seems to conflict with one of these, the law wins.

1. **Clarity beats cleverness.** Code is read under pressure, usually by someone who did not write it, usually while something is broken. The compiler already understands nonsense; humans do not.
2. **Explicit beats implicit, always, without exception, until it is proven tautological.** If the reader cannot see it in the source, it should not be happening. See [The Explicitness Doctrine](#the-explicitness-doctrine).
3. **Make illegal states unrepresentable.** Push invariants into types, constructors, and module boundaries. A comment saying "must be called after `init`" is a defect report you wrote yourself.
4. **Every line is a liability.** New code costs maintenance, review, tests, bug surface, compile time, and binary size — forever. Deletion is the highest-value change you can make.
5. **Reuse before you invent.** Before you write machinery, look in your own `Cargo.lock`. The thing you are about to build is usually already in your dependency tree, implemented better, tested harder, and free.
6. **No hidden control flow, no hidden work, no hidden cost.** Allocation, I/O, blocking, spawning, termination, mutation, and authority are all part of the contract. Name them.
7. **Correctness beats ergonomics.** Sugar that hides meaning is not sugar, it is a trap with a friendly name.
8. **Small surfaces win.** Thin APIs, thin modules, thin files, thin functions, thin diffs.
9. **Warnings are debt, not wallpaper.** A tolerated warning is a place where real problems will hide, and they will.
10. **The standard library is the baseline.** Reach outside it when the crate clearly earns its keep, not because `cargo add` is easy to type.
11. **Refactor before rot.** If a file is obviously going bad, split it now, while the change is small and boring.
12. **Compile time and runtime cost are features.** They are part of the product. They are not somebody else's problem and they are not an optimization you do later.
13. **Behavior first, implementation second.** Tests and docs describe what the software promises, not the accident of how it currently works.
14. **Prefer safe defaults.** Safe Rust first, safe APIs over unsafe internals, typed boundaries over stringly chaos.
15. **When a rule must be broken, break it deliberately and write down why.** An undocumented exception is not an exception, it is a mistake with confidence.

### Design Intent

These standards exist to keep code:

- readable after a month away
- changeable without breakage
- testable without black magic
- splittable into crates or workspaces when growth demands it
- fast to compile and fast to run, on purpose, not by luck
- explicit about ownership, allocation, blocking, failure, authority, and system boundaries
- hostile to ambient globals, shadowing, macro soup, and APIs that hide cost behind cute names

This is a standards document, not a language tutorial.

### On the Language Itself

I write a lot of Rust. I am not a fan of Rust. Those two facts are not in tension and this section exists so nobody has to guess where I stand.

Rust's ownership model is genuinely good and worth the trouble. Almost everything built on top of it — the syntax, the async story, the macro culture, the trait-abuse culture, the "just add a crate" reflex, the compile times, the guard-ceremony API style that turns four lines of intent into fourteen lines of noise — ranges from mediocre to actively hostile to reading. The ecosystem's dominant style optimizes for how clever the author felt while writing, not for the person debugging it at 2am.

So the rules in this document push in one direction: **use the good part, refuse the rest.**

- Take ownership, lifetimes, `Result`, exhaustive `match`, and the type system, and lean on them hard.
- Refuse the macro reflex, the abstraction reflex, the async-everything reflex, and the dependency reflex.
- Refuse the culture of "it's idiomatic" as a substitute for a reason.
- Where the language forces ceremony on you, keep the ceremony at the boundary and keep the intent visible in the middle. Do not let the language's noise become the shape of your program.

If a rule here fights an ecosystem norm, that is intentional and not an oversight. The norm has to earn its place like everything else.

---

## How to Think About Code

The rest of this document is mostly rules. Rules are downstream of a way of thinking, and if you only absorb the rules you will follow them into stupid places. This section is the thinking.

### Ask What This Code Costs

Every piece of code you add has a bill attached, and somebody pays it whether or not you looked at it:

| Cost | Paid by | Paid when |
| --- | --- | --- |
| Reading cost | every future maintainer | every time they touch the area |
| Review cost | your reviewers | now, and on every follow-up change |
| Test cost | you, then CI, forever | every run |
| Compile cost | everyone, including your own edit loop | every build |
| Runtime cost | users | every execution |
| Binary size | users, packagers, CI | every release |
| Dependency cost | the whole tree, including security posture | until you remove it |
| Coupling cost | the next person who tries to change something nearby | at the worst possible moment |

Before you write a thing, you should be able to say what it costs and what it buys. If you cannot state the benefit in one sentence, you are not ready to write it. If the benefit is "it felt cleaner," you are not ready either.

### Do the Reading Before You Do the Writing

Most bad code in my repos is not bad because someone wrote it badly. It is bad because someone wrote it *at all* — they built machinery for a problem their dependencies already solved, because building was faster than reading.

Before writing non-trivial code, you owe the following:

1. Read `Cargo.lock` and `cargo tree`. Know what is actually in the tree.
2. Read the docs for the crate you are about to fight. Not the README — the actual API docs for the types you are using.
3. Read the crate's execution model: what does it own, when does it redraw, what does it cache, what does it poll, when does it allocate, what does it do on drop.
4. Read `std`. It is bigger than you remember and it is already compiled.
5. Read the code you are about to change, all of it, including the parts you think are unrelated.

If you skip this, you will build a workaround on top of a mechanism that already worked, and the workaround will fight the mechanism, and the resulting bug will be subtle and will look like the library's fault. It never is.

### Think in Invariants, Not in Steps

Weak engineering describes a sequence: first we do this, then we do that, then we call this helper. Strong engineering describes a state: this is always true here, this becomes true at this boundary, this is the only place that can break it.

For any unit of code you should be able to answer, without reading the body:

- What is required to be true on the way in?
- What is guaranteed to be true on the way out?
- What can fail, and what state is left behind when it does?
- Who owns each resource involved, and where does it get released?
- What authority does this need (filesystem, network, clock, environment, process control), and where did that authority come from?

If your answer to any of these is "it depends on how it's called," fix the design, not the documentation.

### The Failure Path Is the Real Program

The happy path is the easy 20% and it is the part everyone reviews. Almost every serious bug I have shipped lives on a path nobody read:

- what happens on cancellation, halfway through
- what happens when the child process is still running
- what happens when the write succeeded and the commit didn't
- what happens when the input is empty, or zero-length, or valid-but-degenerate
- what happens when the same event arrives twice, quickly
- what happens when the previous operation is still in flight
- what happens when the resource is gone, but the handle isn't

**Read the error path before the happy path. Review the cancellation path before the success path.** If a code change adds concurrency, cancellation, or external processes, the failure path is the change; the happy path is the trivial part.

### Local Reasoning Is the Whole Game

A maintainer must be able to understand a change from the function, its signature, its module, and its tests. Nothing important should require knowing:

- a call order that is not enforced by types
- an initialization that happened in another file
- a global, thread-local, or lazily-initialized singleton
- a mutation performed by a helper whose name implies a read
- a convention that exists only in someone's head or in a PR comment from last year

Every time behavior depends on something the reader cannot see from where they are standing, you have converted a local problem into a whole-codebase problem.

### Duplication Is Cheaper Than the Wrong Abstraction

Two similar-looking things are not necessarily the same thing. Merging them because the code rhymes creates a shared abstraction that must now serve two masters, and it will grow a boolean parameter, then an enum, then a config struct, and then nobody can change either caller safely.

Rules of thumb I actually apply:

- Two occurrences: duplicate it. Watch it.
- Three occurrences with the *same reason to change*: abstract it, and name the reason.
- Three occurrences with *different reasons to change*: leave them alone, they are not the same thing.
- If the abstraction needs a flag to tell it which caller it is serving, it was never one abstraction.

A little duplication is a local problem. A wrong abstraction is a permanent tax on everyone.

### Abstraction Must Own Something

A layer earns its existence by owning an invariant or isolating a decision that is likely to change. It does not earn its existence by existing.

Delete any layer that only:

- forwards arguments to another function
- renames a call
- copies state from one struct into a nearly identical struct
- restates a rule that is already enforced somewhere else
- exists because a design pattern has a name

"Wrapper," "manager," "helper," "handler," "service," and "util" in a type name are all warnings that something owns nothing. Sometimes they are correct. Usually they mean the author had no model of the problem and reached for a noun.

### Name the Rule, Not the Mechanics

`process_data`, `handle_item`, `do_update`, `helper2`, and `run_step` tell the reader nothing. They are what you name a function when you do not yet understand what it does.

Names should state the rule being enforced or the transformation being performed: `reject_duplicate_keys`, `resolve_relative_to_config_dir`, `truncate_to_preview_limit`, `into_validated`. If you cannot name it, you do not understand it yet, and the function boundary is probably in the wrong place.

### Prefer the Boring Solution and Be Suspicious of Your Own Cleverness

If you are proud of how clever a piece of code is, that is a signal to look harder, not a signal that it is good. Clever code is code that will be misread. The moment you find yourself reaching for a trait to express a two-branch decision, a macro to avoid typing, a generic to serve one caller, or a channel to avoid a function argument, stop and write the boring version. Nine times out of ten the boring version is shorter, faster to compile, faster to run, and correct on the first read.

### Understand the Machine

You do not have to be a performance engineer, but you are not allowed to be ignorant of what your code makes the computer do.

Know, roughly, at all times:

- what allocates and how often
- what copies and how big
- what syscalls happen, and whether they happen in a loop
- what blocks, and what thread it blocks
- what is O(n) and what accidentally became O(n²) because it looked like one loop
- what runs on every frame, every tick, every keystroke, every request

"I'll profile it later" is fine for tuning. It is not fine as an excuse for shipping a redraw loop that runs 60 times a second while the user is doing nothing, or a function that allocates and discards a full `Vec` of decoded rows to answer a set-membership question. Those are not performance problems, they are design errors that happen to be visible in a profiler.

### Finish Things

Half-implemented features, TODOs that describe the real requirement, `unimplemented!()` in a path a user can reach, a config option that is parsed but ignored, an error variant that is constructed but never handled — these are worse than missing features, because they lie about what the software does.

Either it works, it is behind a flag that is off, or it is not in the tree.

---

## The Explicitness Doctrine

This is the center of how I want code written, and it is where I diverge hardest from mainstream Rust style. It is adapted from the design rules of my own language, which exists largely because I got tired of languages that hide things.

**The core claim: if a behavior is not visible in the source at the place it happens, it is a bug waiting for a maintainer.**

### The Tautology Rule

Total explicitness is not the goal — unreadable ceremony helps nobody. The line is precise:

> An operation may be implicit **only if** (1) it is uniquely determined by the types and the surrounding context, (2) every alternative interpretation would be a compile error, and (3) it introduces no control flow, allocation, side effect, or cost beyond what the explicit form already implies.

If all three hold, the implicit form is just the explicit form with redundant typing removed — take it. If any one of them fails, write it out.

Applied to Rust:

| Implicit thing | Allowed? | Why |
| --- | --- | --- |
| `?` on a `Result` with a `From` impl in scope | Yes | Uniquely determined, no hidden cost, control flow is visible in the `?` glyph |
| Type inference on locals (`let x = compute();`) | Yes | Uniquely determined by the expression |
| `.into()` where the target type is unambiguous and the conversion is free | Yes, sparingly | Prefer the named conversion when the reader would have to guess the target type |
| `.into()` that allocates or reformats | **No** | Fails condition 3; write `String::from`, `to_owned`, `to_vec`, or the named constructor |
| `Deref` on a smart pointer you actually wrote | Yes | That is what the trait is for |
| `Deref` used to fake inheritance or auto-forward a domain type's API | **Banned** | Hides which type is being called and where the cost lives |
| `Default` on a config struct | Yes, if every field's default is genuinely obvious | Otherwise the reader has to go read another file to know what they got |
| `Default` used to skip stating important values | **No** | Fails condition 1: the value is not determined by context, it is determined by a decision made elsewhere |
| Blanket `impl<T: Trait> OtherTrait for T` | Almost never | Makes it impossible to see, at a call site, what code is running |
| Ambient globals, thread-locals, lazy statics as inputs | **Banned** | Fails everything |
| Macro-generated control flow | **Banned** by default | See [Macros](#macros-abstraction-and-invented-machinery) |

### Make Cost Visible

The name and signature must not lie about cost. This is enforced at review.

- `as_*` is a cheap borrowed view. It does not allocate. Ever.
- `to_*` copies or allocates. Say so by using this prefix.
- `into_*` consumes. The caller gives up ownership and must see that.
- `from_*` constructs from another representation.
- `with_*` configures.
- A method named like a getter (`name()`, `len()`, `is_ready()`) must be cheap and must not mutate, block, allocate, do I/O, or spawn anything. If it does, it is not a getter and it needs a different name.
- Anything that can block gets `blocking_` in the name or an unmissable statement in its doc comment naming the condition it waits on.
- Anything that spawns background work that outlives the call must say so in the name, the return type (hand back a handle), or both. A function that leaves a task running after it returns, without telling you, is a resource leak with a smiling face.
- Anything that can terminate the process says so, and lives in the binary's top layer, never in a library or a helper.

### Make Authority Visible

Anything that touches the outside world is exercising authority, and authority is passed, not summoned.

- Environment variables, current directory, process args, the system clock, RNG seeds, terminal state, network access, and filesystem roots are read **once**, at the boundary, in code whose obvious job is to be that boundary.
- They are converted into typed values and passed inward as explicit parameters.
- Deep code does not call `std::env::var`, `SystemTime::now`, `rand::random`, or `current_dir`. Deep code takes what it needs as an argument.
- This is not testing dogma, though it does make testing trivial. It is so a reader can tell what a function can do by reading its signature.

If a function's signature takes only pure data, it must only do pure data things. That is a promise, and violating it is a serious review finding, not a nit.

### Make Resource Release Visible

Rust's `Drop` is convenient and it is also the language's largest source of invisible behavior. Convenience is not license.

- Anything with a meaningful shutdown — a process, a task, a connection, a terminal mode, a temp file, a lock held across a boundary — gets an explicit release path at the point the reader is looking.
- `Drop` impls may clean up. They must not be the only documentation that cleanup happens, and they must not do anything that can fail silently, block, or spawn.
- Never rely on drop order across module boundaries for correctness.
- `kill_on_drop`-style conveniences are not cancellation. They handle one narrow case (the direct child) and quietly fail the general one (its descendants). See [Defect Classes](#defect-classes-that-keep-shipping).

### Make Failure Visible

- No `unwrap()` in maintained non-test code. `expect()` only when the message states the invariant that makes it impossible, in words a stranger can check.
- No silent fallback. If you swallow an error and substitute a default, the reader must be able to see that decision at that spot, and it must be a decision, not a `unwrap_or_default()` reflex.
- No error type that erases the thing you will need at 2am. Preserve the path, the input, the exit status, the raw output.
- No catching a failure and re-deriving a worse one. The first failure is the evidence.

### Every File Says What It Handles

Every non-obvious file starts with a module doc comment that states what it owns. Not what functions are in it — what *responsibility* it holds, what invariants it maintains, and what it deliberately does not do.

```rust
//! Preview command execution for dmenu mode.
//!
//! Owns the lifetime of preview child processes: spawning them into their own
//! process group, cancelling stale ones when the selection changes, and bounding
//! captured output. Does not decode or render output — see `render.rs`.
//!
//! Invariant: at most one preview process group is live at a time; the previous
//! group is signalled before a new one is spawned.
```

That header costs thirty seconds and saves the next person twenty minutes. If the file's job is genuinely self-evident from its name and its ten lines of content, skip it. If you have to think about whether it is obvious, it is not obvious — write the header.

`main.rs` and `lib.rs` always get one, no exceptions.

### No Unstated Conventions

If the code depends on a rule, the rule is written down somewhere a reader will find it:

- an ordering requirement → encode it in types, or document it on the function that requires it
- a naming convention that tooling depends on → document it at the module that reads it
- a magic number, timeout, limit, or buffer size → named constant with a comment saying where the number came from
- a workaround for a specific upstream bug → comment with the crate, version, and issue link, plus the condition under which it can be deleted

"It's obvious" is not a reason to skip writing something down. It was obvious to you, once, briefly.

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
- when you're online and coding on a feature/fix branch, open a **draft PR early** and commit to it so others have visibility; don't work in silence on shared repos if it might overlap someone else's task

If you are the primary maintainer, these standards still apply.
Solo maintenance is not a reason to skip review thinking, testing, rollback planning, or release hygiene.

Not every section applies equally to every project.
The observability, rollout, and incident sections scale with the software:

- a small library or CLI still needs good errors, sane logging where relevant, and clear behavior
- a daemon, service, networked app, or long-running system needs the full operational treatment

---

## Related References

This document is grounded in official Rust references and the Rust API Guidelines, but it is not a summary of them and it disagrees with parts of the ecosystem's received wisdom on purpose.

**Official Rust material** — the baseline for syntax, tooling, and API conventions:

- The Rust Style Guide, The Rust Programming Language, The Rust Reference, The Cargo Book, the rustdoc book, Clippy's lint documentation, and the Rust API Guidelines.

**Engineering sources that shaped the stance here** — these are where the opinions come from, not decoration:

- Holzmann's *The Power of Ten* — rules small enough to remember and precise enough to check, bounded control flow, checked return values, warning-clean builds. The specific bans (no recursion, no dynamic allocation) belong to safety-critical avionics, not to general software; the discipline behind them travels.
- Parnas, *On the Criteria To Be Used in Decomposing Systems into Modules* — draw boundaries around decisions likely to change, not around the chronological steps of an algorithm. This is the source of the boundary-module rule and of most of what this document says about abstraction.
- SQLite's testing account — fault injection at successive allocation and I/O sites, compound failure, malformed state, retained regressions, mutation testing as a check on whether tests observe anything. Coverage is an evidence dimension, never an oracle.
- Google's engineering practices on code review and small changes — the "leave it healthier than you found it" bar rather than a perfection bar.
- Google SRE on simplicity, monitoring, release engineering, and blameless postmortems.
- Sean Parent's *No Raw Loops* and the surrounding C++ discipline about naming operations rather than open-coding them — with the Rust caveat that a named explicit loop beats an unreadable combinator chain.
- Logan Smith (`@_noisecode`), *How to write the perfect function* — signature-as-contract thinking; single responsibility applied at function granularity.
- Aria Beingessner's writing at faultlore.com — particularly on Rust's unsafe pointer story, memory models, type layout and ABI, linear types, and the Rust-specific bug classes. The most honest technical writing about Rust's rough edges that exists, and the reason this document is comfortable saying the language has rough edges.
- corrode's compile-time guide — the empirical basis for [Compile Time Is a Feature](#compile-time-is-a-feature).

**My own language work** — the explicitness rules in this document are adapted from the design constraints of a language I am building, where they are enforced by the compiler rather than by review:

- no hidden control flow: if it is not in the source, it is not happening
- everything is explicitly consumed; nothing is silently cleaned up
- the tautology rule for what may be implicit
- allocation, blocking, and authority are part of every API's contract
- no unstated compiler or library conventions

Rust cannot enforce most of that. This document is me enforcing what I can, by hand, because I got tired of not having it.

Exact links are in the [Standards Basis](#standards-basis) at the end.

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

### Minimize Invented Solutions and Maximize Dependency Leverage

Minimizing external dependencies is crucial, but minimizing *invented solutions* to problems your dependencies already solve is equally essential.

Before writing custom workarounds, wrappers, or state-management logic, inspect what your current dependency tree already provides. Reinventing functionality that an underlying crate already handles wastes cognitive thinking budgets, inflates code churn, and breeds subtle runtime bugs by fighting the library's internal execution model.

Rules:

- **Understand crate execution models:** Study the rendering, memory, caching, or concurrency guarantees of your dependencies before layering manual fixes over them.
- **Do not fight framework mechanisms:** For example, in a terminal UI application (`orbit-tui`), manually invoking explicit screen clears (`terminal.clear()`) inside a frame render loop when using a double-buffered TUI crate (such as `ratatui` or similar render engines) destroys double-buffering benefits, introduces visual flickering, and churns code—the rendering crate already diffs memory buffers and manages redrawing in the background.
- **Reuse existing tree capabilities:** Before implementing a custom cache, state tracker, or signal handler, check if `std` or existing workspace crates already expose that capability natively or via feature flags.
- **Protect thinking budgets and minimize churn:** Invented code is code that must be written, reviewed, tested, and maintained. Maximal reuse of established dependency primitives keeps PR diffs small and cognitive overhead low.

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

## Compile Time Is a Feature

Compile time is not a background inconvenience. It is the tick rate of your entire engineering loop. A project with a 90-second edit-check cycle gets worse code than the same project with a 5-second cycle, because the human stops running the loop, stops testing small things, batches changes, and starts guessing. Slow builds make people sloppy. That is the actual cost, and it dwarfs the wall-clock time.

Treat build time like latency: measure it, budget it, and regress on it.

### Budgets

Set these per project and check them when they start to hurt:

| Loop | Target | Alarm |
| --- | --- | --- |
| `cargo check` incremental, one file touched | under 2s | over 10s |
| `cargo build` incremental debug | under 10s | over 30s |
| `cargo test` incremental | under 30s | over 2min |
| Clean release build | project-specific, write it down | 2x last recorded |
| Direct dependency count | as few as do the job | growth with no removals |

Numbers are per project and per machine. The point is that a number exists and someone notices when it doubles.

### Measure Before You Guess

```bash
cargo build --timings                  # per-crate wall time, parallelism, critical path
cargo llvm-lines | head -30            # what is generating the most LLVM IR
cargo bloat --release --crates         # where binary size (and thus codegen) comes from
cargo tree --duplicate                 # same crate compiled twice at different versions
cargo +nightly rustc -- -Z time-passes # is the linker the bottleneck?
```

Nightly-only, worth it when you are actually hunting:

```bash
RUSTFLAGS="-Zmacro-stats" cargo +nightly build     # cost of each proc-macro
RUSTFLAGS="-Zprint-mono-items=yes" cargo +nightly build
cargo rustc -- -Zself-profile
```

Do not "optimize" build times by feel. Every one of these tools will tell you something surprising and most of the surprises are dependencies you forgot you added.

### The Rules

**Dependencies are the main cost.** Not your code. Your code is almost never the bottleneck in a young project.

- Audit regularly with `cargo machete`, `cargo shear`, and `cargo +nightly udeps`. Run more than one; they disagree.
- Prune default features aggressively. `default-features = false` and then add back what you use. `bindgen` pulling in a full argument parser, a serialization framework pulling in derive machinery you never invoke — this is normal and it is your job to notice.
- Prefer the lighter crate when it does the job: `ureq` over `reqwest` when you need three HTTP calls, `lexopt` over `clap` for a small CLI, hand-written parsing over a derive framework for two structs.
- One heavyweight dependency that appears early in the dependency graph poisons the critical path for the entire build. Look at `cargo build --timings` and find what everything is waiting on.
- Consolidate duplicate versions. Two versions of the same crate is two compilations and often two incompatible types.

**Proc-macros are a build-time tax you pay on every build.**

- Every proc-macro crate is compiled, then *executed*, on your critical path, and it blocks everything downstream.
- Gate them behind features in library crates: make `serde` optional, use `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]`, and enable it only in the leaf crate that actually serializes.
- Optimize the ones you cannot avoid: `[profile.dev.build-override] opt-level = 3` makes build scripts and proc-macros run faster.
- This is a build-time reason to distrust macros. There is also a readability reason and a debuggability reason. See the next section.

**Generics are monomorphization, and monomorphization is codegen.**

- A generic function is compiled once per instantiating type. A generic function that is large, and instantiated with eight types, is eight large functions in your binary and in your build.
- Use the outer-generic / inner-concrete pattern for anything nontrivial:

```rust
pub fn read_to_string(path: impl AsRef<Path>) -> io::Result<String> {
    fn inner(path: &Path) -> io::Result<String> {
        // the real work, compiled exactly once
    }
    inner(path.as_ref())
}
```

- `impl Trait` in argument position is fine; the body still monomorphizes, so keep the body thin.
- Check `cargo llvm-lines` when a build gets slow. The answer is usually one generic function you did not think was expensive.

**Structural rules.**

- Split into workspace crates when there is a real boundary *and* it buys parallelism — not to hide a bad module layout. A crate boundary is also a compile boundary, which is a real benefit, but a badly placed one costs you more in churn than it saves in seconds.
- Consolidate integration tests. Every file in `tests/` is a separate binary with its own link step. `tests/main.rs` with `mod` declarations for the rest builds and links once.
- Keep `main.rs` thin so the binary crate has almost nothing to recompile.

**Toolchain and machine.**

- Keep the toolchain current. `rustc` gets meaningfully faster every year for free.
- Use a fast linker. This is the single highest-leverage local change on Linux:

```toml
# .cargo/config.toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
```

  `lld` is the portable fallback. On macOS the current default linker is fine; on Linux, `mold` is not.
- Cut debug info in the loops that do not need it:

```toml
[profile.dev]
debug = 0            # or "line-tables-only" if you want usable backtraces
split-debuginfo = "unpacked"
```

- Use `cargo check` (and a watcher) for the edit loop. Build when you need to run it, not to see if it typechecks.
- Use `cargo nextest run` instead of `cargo test` on anything with a real test suite.
- Nightly `-Zthreads=8` (parallel frontend) and the Cranelift backend are legitimate options for local dev builds. Both are dev-loop tools only — release artifacts use the stable toolchain and LLVM.

**CI.**

- `CARGO_INCREMENTAL: 0` in CI. Incremental state is useless for a clean build and costs time and disk.
- Cache with `Swatinem/rust-cache@v2` or equivalent.
- Split `cargo test --no-run --locked` from the test run so compile failures and test failures are distinguishable at a glance.
- `RUSTFLAGS: -D warnings` in the environment instead of `#![deny(warnings)]` in source, so local builds are not hostile.
- For containers, `cargo-chef` to get dependency layers cached properly.

### The Compile-Time Review Question

When reviewing, if a diff adds a dependency, a proc-macro, a derive on a hot type, a new generic wrapper, or a new crate, the reviewer asks: **what did that do to the build?** "I don't know" is an acceptable answer exactly once — go run `cargo build --timings` and come back.

---

## Project Layout & File Tree Standards

### Default Package Shape

For most non-trivial binaries:

```text
project/
  Cargo.toml
  rustfmt.toml
  .cargo/config.toml      # linker, dev profile tweaks
  README.md
  src/
    lib.rs
    main.rs
  tests/
    main.rs               # one integration test binary, submodules inside
```

Why:

- a package can have both `src/lib.rs` and `src/main.rs`, and should
- logic in `lib.rs` is testable, reusable, and does not force a link of the binary to run a unit test
- `main.rs` parses args, calls into the library, maps errors to exit status, and does nothing else
- one integration test binary instead of ten means one link step instead of ten

### Thin `main.rs` Rule

If the binary is more than a toy:

- real logic lives in `lib.rs` and below
- `main.rs` stays thin
- no business logic hiding in argument parsing or bootstrap glue
- the only place `std::process::exit` may appear is here

### `mod.rs` Is Banned

Use the 2018-and-later module layout. A module `foo` is `src/foo.rs`, and its children live in `src/foo/`.

```text
src/
  cli.rs
  cli/
    parse.rs
    help.rs
    validate.rs
```

Not this:

```text
src/
  cli/
    mod.rs          # banned
    parse.rs
```

The reason is entirely practical: `mod.rs` gives you a project full of files that are all named `mod.rs`. Every editor tab says `mod.rs`. Every fuzzy-file search says `mod.rs`. Every stack trace, grep result, and PR diff header says `mod.rs`. You cannot tell them apart without reading the path, which is exactly the information the filename was supposed to give you for free.

Existing `mod.rs` files in inherited code are migrated when the module is touched for other reasons. Do not open a PR that does nothing but rename them — that is churn, and it breaks everyone's in-flight branches for no behavioral gain.

### Every File Declares Its Job

Every module file begins with a `//!` header stating what it owns, unless the file is genuinely trivial and self-evident from its name. `lib.rs` and `main.rs` always have one.

State:

- what responsibility this module holds
- what invariants it maintains
- what it deliberately does *not* do, and where that lives instead
- any non-obvious constraint (ordering, threading, platform, authority)

```rust
//! Configuration loading and validation.
//!
//! Owns the merge order for defaults, config file, environment, and CLI flags —
//! later sources win. All environment and filesystem access for configuration
//! happens here and nowhere else; the rest of the program receives a validated
//! `Config` value.
//!
//! Does not own runtime state or CLI parsing itself — see `cli.rs`.
```

This is not a documentation ritual. It is how someone opening the file at 2am finds out whether they are in the right file, in five seconds instead of five minutes. Write it in the author's voice, keep it to a few lines, and update it when the module's job changes.

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
  common.rs
  types.rs
```

`types.rs` deserves its own mention: a file that holds all the types, separated from all the behavior, is not organization. It is the data/behavior split that object orientation spent thirty years arguing about, imposed by accident. Types live with the code that maintains their invariants.

`utils.rs` is allowed if it is truly small, truly generic, and truly stable. It is on probation permanently. The moment its contents stop having anything in common, split it by actual purpose.

### When to Create a New File

Create a new file when **one** of these is true:

- the file holds more than one responsibility
- it is growing past the size limits below
- a concept became important enough to deserve a name
- the tests for a concept would be clearer beside their own module
- the module has children and wants a stable top-level API
- navigation is getting slow because everything lives in one place

Do **not** create a new file when:

- it would hold one tiny helper with no meaningful boundary
- it would force readers through five files to understand one function
- the split is by syntax (all the structs here, all the impls there) rather than by responsibility

### Boundary Modules

Any module that talks to the outside world — a database, a terminal, a foreign library, a wire protocol, the process environment — is a boundary module, and it has a specific job:

- absorb the external API's ceremony so nothing above it sees a guard, a handle, a raw byte slice, or a C type
- convert external representations into domain types at the edge
- own every failure translation for that boundary
- be the only place that touches that external thing

This is the single most valuable structural rule in this document. Get it right and the rest of the codebase stays readable regardless of how unpleasant the library is. Get it wrong and transaction guards, `unsafe`, `cfg` blocks, and `libc` types leak into your business logic and never leave.

### When to Create a Workspace

Start with one package. Split into a workspace when there is a real boundary:

- multiple crates with genuinely distinct responsibilities
- a library reused by more than one binary
- build-time parallelism or feature isolation that measurably helps
- separate publishing or versioning paths
- shared lockfile, target dir, lint policy, and CI across real components

Do **not** create a workspace because a single crate got moderately large. Fix the module boundaries first. A premature workspace split freezes a bad decomposition into crate boundaries, which are far more expensive to move than module boundaries.

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

### File Headers

Every module file opens with a `//!` header naming its responsibility and its invariants,
unless the file is genuinely trivial. `lib.rs` and `main.rs` always have one. See
[Every File Declares Its Job](#every-file-declares-its-job).

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
- `.collect::<Vec<_>>()` followed by a single `for` loop over the result
- boolean trap arguments
- over-generic APIs with unreadable bounds
- macros used to avoid writing normal Rust
- exposing internal representation prematurely
- smart-pointer `Deref` tricks unless you are actually implementing a smart pointer
- shadowing bindings to smuggle in a state transition
- ambient globals, thread-locals, or process state as hidden inputs
- helper names that hide allocation, I/O, or mutation cost

---

## Macros, Abstraction, and Invented Machinery

This section exists because it is where the most damage gets done, and because "it's idiomatic" has been used to defend all of it.

### Macros

**Default position: you do not write macros.**

A macro is a small, undocumented, unversioned language that only exists in your repo, that your IDE understands worse than real code, that your debugger cannot step through cleanly, that your reviewer must expand in their head, and that your compiler must expand on every build. The bar for creating one is correspondingly high.

**Banned outright:**

- macros to avoid typing (`impl_all_the_things!`, `make_struct!`)
- macros that generate control flow the caller cannot see
- macros that hide `return`, `?`, `break`, `continue`, or early exit
- macros that generate public API surface that is not visible in the source
- macros that exist because a function would have needed one more parameter
- `macro_rules!` used as a substitute for a trait, a generic, or an enum
- proc-macro crates written in-repo to solve a problem three call sites have

**Acceptable, with a written justification in the PR:**

- genuine variadics where Rust has no other answer (`vec!`-shaped, `format_args!`-shaped)
- test-only macros that build fixtures or run one assertion body across a table of cases
- `#[derive(...)]` for the standard derives on plain data (`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash`, `Default`, `PartialOrd`, `Ord`)
- established ecosystem derives on types that genuinely are data (`Serialize`/`Deserialize`), gated behind a feature in library crates
- declarative macros that eliminate a repetitive *pattern of definitions* — not a pattern of logic — where the expansion is small, local, and obviously mechanical

**If you write one anyway, it must:**

- live in one place, near what it serves, never in a `macros.rs` junk drawer
- have a doc comment showing the input and the expansion
- be checked with `cargo expand` during review — the reviewer reads the expansion, not just the invocation
- not be exported publicly unless the crate's entire purpose is that macro

**On derives:** every derive you add is codegen. `#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize)]` reflexively slapped on every type is a real cost in build time and binary size, and half of those impls are semantically wrong for the type anyway. Derive what the type actually means. If `PartialOrd` on your type has no meaningful ordering, do not derive it just because it was easy.

### Helpers

A helper function is justified when it **gives a rule a name** or **isolates a responsibility**. Moving lines out of a long function into `fn part_two()` is not decomposition; it is the same function with more scrolling.

Banned helper shapes:

- `do_x_part_1`, `do_x_part_2`, `step_three`, `helper`, `helper2`, `inner_impl`
- a helper called from exactly one place that does not name a rule and cannot be understood without reading the caller
- a helper whose name hides allocation, I/O, mutation, blocking, or process spawning
- a `utils` / `helpers` / `common` / `misc` module that has grown past a handful of genuinely generic, genuinely stable functions

`utils.rs` is allowed to exist and it is on probation from the day it is created. When it hits a few hundred lines, or when its contents stop having anything in common, it gets split by actual purpose.

### Traits

Traits are for polymorphism you actually need. They are not for organizing your code, expressing a namespace, or making a type feel object-oriented.

- Do not define a trait with one implementor. That is a struct with extra steps and an extra layer of indirection to read through.
- Do not define a trait so a future second implementor can exist. Write it when the second implementor exists.
- Do not use blanket impls (`impl<T: Foo> Bar for T`) unless you are writing a library whose entire purpose is that extension. They make it impossible to see, at a call site, what code runs.
- Do not use extension traits to attach methods to foreign types when a free function would do. `fn parse_duration(s: &str) -> Result<Duration>` is more honest than `"5s".parse_duration()`, because the reader can find it.
- Trait objects (`dyn Trait`) are fine and often better than generics: one compilation, smaller binary, faster builds. Use them when the dynamic dispatch cost is irrelevant, which is most of the time.
- Put bounds where they are used, not on the struct definition.

### Generics

- Do not make something generic for one caller. Write the concrete type. Generalize when the second caller shows up with a different type and the same logic.
- Generic parameters that only exist to accept `&str` and `String` should be `&str`, and the caller can deref.
- If your signature needs a `where` clause longer than the function body, the design is wrong.
- Every generic is monomorphized. See [Compile Time Is a Feature](#compile-time-is-a-feature).

### Reinventing What the Tree Already Does

This is the one that costs me the most review time and it is entirely avoidable.

Before you write custom machinery, you check:

1. **`std`.** It is bigger than you remember. `std::process`, `std::io::Read::take`, `BufRead`, `OnceLock`, `mpsc`, `Cow`, `binary_search_by`, `sort_unstable_by_key`, `retain`, `Entry` API, `iter::successors`, `array::from_fn`, `Path` manipulation — people reimplement all of these weekly.
2. **Your direct dependencies.** Read their docs. Not their README — their docs.
3. **Your transitive dependencies.** If `tokio` is already in your tree, you do not need a hand-rolled thread pool. If `rustix` is already there, you do not need to write a `libc` shim. If a TUI crate is there, it already handles double buffering, diffing, and redraw.
4. **Feature flags on crates you already depend on.** The capability is frequently already paid for and just switched off.

**Fighting a dependency's execution model is a defect, not a workaround.** Concrete, real examples:

- Calling `terminal.clear()` inside a render loop of a double-buffered TUI crate. The crate already diffs buffers. You just destroyed that and introduced flicker.
- Adding a mutex around a type that is already internally synchronized.
- Writing a manual cache in front of a client that already caches, with different invalidation rules, producing two sources of truth.
- Hand-rolling retry/backoff on top of a client that has a configurable retry policy.
- Polling for a condition that the library will hand you as an event.

When you find yourself adding "manual resets," "force clears," "just in case" flushes, or extra synchronization on top of a library abstraction, **stop**. You have misread the model. Go read the docs, find the mechanism, and delete your code. The fix is nearly always smaller than the workaround.

If 5 lines of library calls replace 80 lines of custom orchestration, you write the 5 lines. Every time. This is not a style preference — the 80 lines have bugs in them that you have not found yet.

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

A reasonable strict baseline for a maintained project:

```toml
[lints.rust]
unsafe_code = "forbid"          # "deny" if the crate genuinely needs unsafe
missing_docs = "warn"           # for library crates
unreachable_pub = "warn"

[lints.clippy]
dbg_macro = "deny"
todo = "deny"
unimplemented = "deny"
unwrap_used = "deny"
print_stdout = "deny"           # for library crates
```

Do not enable `clippy::restriction` wholesale. It contains mutually contradictory lints and
enabling it as a category is a sign nobody chose anything.

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

See [Review Standard](#review-standard) for how review actually works here: what it is for,
how to read a diff, the severity ladder, how to write a finding, and the merge bar.

---

## Review Standard

Review is where quality is actually decided. Everything else in this document is advice until a reviewer enforces it.

This section applies to humans and to AI agents equally. Agents review more code in my repos than people do, and the failure mode is identical: they check whether the code looks reasonable instead of checking whether it is complete.

### What Review Is For

In priority order:

1. **Correctness** — does it do what it claims, including on the paths nobody exercised?
2. **Completeness** — are the states enumerated, the resources bounded, the failures handled?
3. **Design** — is this the right shape, or is it a workaround for a shape that is wrong?
4. **Cost** — runtime, compile time, binary size, dependency weight, review budget for the next person.
5. **Clarity** — can the next person read it without reconstructing the author's mental state?
6. **Tests** — is the risky part actually covered, and would the test have caught the bug this change fixes?

Style is last and it does not block anything unless it violates a written rule in this document.

### How to Actually Review

**Read the diff twice.** First pass: what is this trying to do, and is that the right thing to do? Second pass: line by line, does it do it.

**Read the failure path first.** Find every `?`, every `match` on a `Result`, every `else`, every cancellation site, every early return. That is where the bugs are. The happy path is the part the author tested.

**Construct the states.** Do not accept the author's framing of what states exist. Enumerate them yourself. If the code has an enum, check the `match` is exhaustive and that each arm is right, not just present. If it does not have an enum but has three booleans, that is a finding.

**Ask what runs when nothing happens.** Idle CPU, idle allocations, idle syscalls, background tasks with no owner.

**Ask what happens on the tenth rapid repetition.** Most concurrency bugs are invisible on the first invocation and obvious on the tenth.

**Find the thing that owns each resource.** For every process spawned, file opened, task spawned, lock taken, terminal mode set: name the owner and the release site. If you cannot, that is the finding.

**Check the reuse question.** Is any of this already in `std` or in the dependency tree? This is the highest-value question in review and almost nobody asks it. Deleting 80 lines is worth more than perfecting them.

**Check the cost question.** New dependency, new derive, new generic, new proc-macro, new allocation in a loop — what did it cost, and did the author know?

### Severity Ladder

Use these consistently so "a comment" and "a blocker" are distinguishable.

| Level | Meaning | Blocks merge? |
| --- | --- | --- |
| **P0** | Data loss, corruption, security hole, or a crash on a reachable path. | Yes, and stop what you are doing. |
| **P1** | Wrong behavior on a real user path, resource leak, unbounded growth, cancellation that does not cancel. | Yes. |
| **P2** | Wrong behavior on an edge case, degraded UX, silent error swallowing, missing state transition, avoidable cost. | Yes, unless explicitly deferred with a tracking issue. |
| **P3** | Design smell, reinvented machinery, missing test for risky logic, unclear naming on a public item. | No, but it should be fixed now while the context is loaded. |
| **Nit** | Style not covered by a written rule, personal preference, bikeshed. | Never. Prefix it `nit:` and let the author ignore it. |

If you cannot assign a level, you do not understand the finding well enough to file it.

### How to Write a Finding

A useful finding has three parts, and this is the format I want from humans and agents both:

1. **What is wrong** — one sentence, stated as a defect, not as a preference.
2. **The failure scenario** — concrete inputs or sequence of events that produce the wrong outcome. If you cannot construct one, say so and downgrade the severity; a finding you cannot demonstrate is a hypothesis.
3. **The direction of the fix** — not necessarily the patch, but what shape the correct code has.

Bad: "Consider using a bounded channel here."

Good: "P1: rapid selection changes queue one pending decode per event, each retaining up to 32 MB of input, with nothing bounding the queue. Holding the down-arrow key exhausts memory. This needs a single-slot latest-wins handoff, since only the current selection's preview is ever displayed."

Say *why*, not just *what*. A finding that does not explain the failure teaches nobody and gets argued about instead of fixed.

### For AI Reviewers Specifically

You are worse at this than you think you are, in specific and predictable ways. Compensate:

- **Do not report style as if it were correctness.** If it is not in this document, it is a nit.
- **Do not pad the list.** Five real findings beat twenty findings where fifteen are noise. Noise trains the author to skim, and then they skim past the real one.
- **Verify before you file.** Trace the actual code path. "This might not handle X" without checking whether it handles X is worthless and it costs the author real time to disprove.
- **Do not accept the diff's framing.** Read the surrounding file. Most of these defect classes are only visible when you look at what the changed code interacts with.
- **Prefer the deletion.** If the diff reimplements something in the tree, the finding is "delete this and call the existing thing," not a list of improvements to the reimplementation.
- **Check this document's ban list.** New macro, new one-implementor trait, new `utils` dumping ground, new unbounded channel, new ambient global, missing module header — these are findings, not preferences, because they are written down here.

### Author Responsibilities

- Open a draft PR early on shared repos, and push to it. Do not work in silence.
- Keep the diff to one concern. Renames, moves, and formatting go in their own commits or their own PRs.
- Write the PR body for the reviewer: what changed, why, what you considered and rejected, what you are unsure about, what you tested and how.
- Say what you did *not* do and why. Deferred scope stated up front is a plan; deferred scope discovered in review is a surprise.
- When you get a finding, either fix it or explain why it is wrong. "Done" with no change, or a change that addresses the symptom rather than the finding, wastes the review.
- Re-request review after substantive changes. Do not let a stale approval carry new code.

### Merge Bar

A change merges when:

- CI is green: `fmt`, `clippy -D warnings`, tests, docs.
- No open P0/P1/P2 findings.
- The risky part has a test that would have failed before the change.
- The diff leaves the codebase healthier than it found it.
- Nothing in it violates a hard ban in this document without a written, accepted justification.

It does not need to be perfect. Perfect is not a merge criterion and chasing it blocks good work. Healthier-than-before is the bar.

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

### Maximize Dependency Tree Leverage & Avoid Invented Solutions

Adding new crates blindly adds debt, but ignoring what is *already present* in your dependency tree to write home-grown workarounds adds double the debt.

Before implementing a custom helper, runtime hack, or workaround:

1. **Audit `Cargo.lock` and existing dependencies:** Is there already a crate in the tree (or a feature flag in an existing dependency) that handles this out of the box?
2. **Respect internal crate semantics:** Avoid adding manual resets, force-clears, or custom mutexes on top of abstractions that already manage those lifecycle phases internally.
3. **Measure code churn and review budget:** Reinventing wheel logic inflates patch size and wastes maintainer review budget. If 5 lines of idiomatic library calls replace 80 lines of custom orchestration, use the library calls.

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

## Performance, Allocation, and Cost

### General Rule

Do not optimize blindly. Measure first.

But "measure first" is not a license to be ignorant of cost. There is a difference between
*tuning* — which needs a profiler — and *not shipping obvious waste*, which needs you to have
read your own code. Decoding an entire table to answer a set-membership question, re-parsing
the same input on every keystroke, or allocating inside a render loop are not performance
questions to be settled by benchmark. They are design errors that happen to also be slow.

Know at all times, roughly: what allocates, what copies, what syscalls, what blocks, what
runs per frame or per keystroke or per request, and what is accidentally quadratic.

Patterns that are avoided by default, without needing a measurement to justify it:

- unnecessary `clone()`
- repeated parsing of the same input
- repeated allocation in tight loops
- needless `String` ownership when `&str` or `Cow` is enough
- converting back and forth between types without reason
- decoding, parsing, or allocating far more than the caller needs, then discarding it
- work performed on a timer when nothing has changed
- collecting into a `Vec` only to iterate it once

### Allocation Rules

- Borrow when ownership is not needed.
- Use owned types when ownership clarifies lifetime and API behavior.
- Avoid sprinkling clones to silence the borrow checker.
- If the borrow checker is fighting you, the design may be wrong.

### Iterator Rules

- Use iterators when they improve clarity.
- Use loops when iterators get too dense.
- Do not turn simple control flow into unreadable combinator soup.

### Concurrency and Async

Moved to its own section because it needs more than a bullet list. See
[Async, Concurrency, and Cancellation](#async-concurrency-and-cancellation).

The one-line version: concurrency is not a substitute for better structure, async is not a
default, and if you spawn it you own its cancellation.

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

## Async, Concurrency, and Cancellation

Async Rust is the part of the language I trust least, and I have the scar tissue to justify that. It is not that concurrency is hard — concurrency is hard everywhere. It is that async Rust makes the *cost model invisible* while making the *failure modes exotic*, which is exactly backwards from how this document says code should work. A `.await` looks like a function call and is a state machine yield point. A dropped future looks like nothing and is a cancellation. A spawned task looks like a statement and is a resource with an independent lifetime.

So: async is not the default, it is a tool with a specific job, and using it obligates you to a longer checklist than sync code does.

### When Async Is Justified

Use async when you have **many concurrent I/O-bound operations whose completion order matters**. That is the problem it solves. Specifically:

- a server handling many connections
- a client fanning out many requests
- a UI event loop that must stay responsive while I/O completes
- anything where you would otherwise be writing a `select`/`poll` loop by hand

### When Async Is Not Justified

- You have one I/O operation. Use blocking I/O.
- You have a handful of parallel operations. Use threads. `std::thread::scope` is excellent and nobody remembers it exists.
- You have CPU work. Async does nothing for you. Use `rayon` or threads.
- You want a timeout. A thread and a channel with `recv_timeout` is fine.
- You want to "future-proof" the design. No.
- The library you want is async-only. Consider a different library, or `block_on` at exactly one place.

**Do not make a function async because it calls an async function three layers down that did not need to be async either.** Async is viral, and every function it infects loses the ability to be called from ordinary code, tested simply, or reasoned about locally. Push it to the edges. Core logic stays sync.

### Runtime Policy

- One runtime, chosen deliberately, named in a decision record if the project is nontrivial.
- Do not write abstraction layers over runtimes. You will not switch, and if you do, the abstraction will not survive contact with the switch.
- Do not enable runtime features you do not use. `tokio` with `features = ["full"]` is a compile-time and binary-size decision you made by not thinking about it.
- `block_on` appears at boundaries, in `main`, or in a test. It does not appear inside library code and it never appears inside async code.

### The Hard Rules

**Never block the executor.** Blocking I/O, CPU-heavy work, `std::sync::Mutex` under contention, filesystem operations, image decode, compression, cryptography, or any loop that runs long — none of it belongs on an executor thread. It stalls every other task on that thread, including the ones handling user input.

- Use `spawn_blocking`, a dedicated worker thread, or a separate process.
- "It's usually fast" is not an argument. Design for the 32 MB input, not the 3 KB one.
- Symptom to watch for in review: an `.await` on a decode/parse/compress call inside an event loop. That is a freeze, and the user will describe it as "it hangs sometimes."

**Never hold a lock or guard across `.await`.** Not a `MutexGuard`, not a `RefCell` borrow, not a database transaction guard, not a terminal raw-mode guard. If you need state across a yield point, restructure so you take the lock, extract what you need, drop it, then await.

**Cancellation is a first-class path and you must design it.** In async Rust, dropping a future cancels it at its last yield point. That means:

- Any work in flight when cancellation happens stops *wherever it was*, not at a clean boundary.
- State mutated before the yield stays mutated. State that was going to be cleaned up after the yield never gets cleaned up.
- `select!` cancels the losing branches. Every `select!` is a cancellation site — read every branch as "this may stop halfway."

For every task you spawn or future you `select!` on, answer: if this is cancelled right now, what is left behind? If the answer includes a running process, an open file, a half-written buffer, a lock, or a shared flag in the wrong state, fix it before merging.

**Cancelling a task does not cancel the work it started.** This is the one that bites hardest and it shows up in every codebase I have written.

- `JoinHandle::abort()` stops the *task at its next yield point*. Work inside `spawn_blocking` keeps running to completion. There is no way to interrupt it. If cancellation matters, the blocking work must poll a cancellation flag itself.
- `Command` with `kill_on_drop(true)` kills **the direct child only**. If that child is a shell, and the shell started a pipeline or a background job, those keep running. Spawn external processes into their **own process group** and signal the group on cancellation. Do not ship anything else.
- A task that has been aborted may still hold the last reference to a large buffer, a file handle, or a socket until it is actually polled and dropped.

**Bound everything.** Unbounded is a synonym for "unbounded memory growth under load."

- Channels are bounded by default. `unbounded_channel` requires a justification, and the justification must explain why the producer cannot outrun the consumer.
- Concurrent work is bounded: a semaphore, a worker pool, or a serialized single worker.
- Retained per-item state is bounded. If each in-flight item holds up to 32 MB and you can queue arbitrarily many, you wrote an OOM.
- Where only the newest result matters (a preview, a search, a render), use **latest-wins**: keep one pending slot, replace it, drop the old one. Do not queue work you already know is stale.

**Supersede stale work, do not just ignore its result.** If the selection changed, the query changed, or the frame moved on, the old work must be *stopped*, not merely discarded when it finishes. Discarding results still burns CPU, still holds memory, still delays the current work, and still leaves processes alive.

**Do not do work when nothing happened.** A loop that wakes on a fixed timer and redraws, re-polls, or re-renders regardless of whether anything changed is burning battery and terminal throughput for nothing. Event-driven means: redraw on input, on resize, on completion of pending work. Not on a 16 ms tick because that was the default and nobody looked.

**Backpressure is a design decision, not an emergent property.** Decide what happens when the consumer is slower than the producer — block, drop oldest, drop newest, or reject — and write it down at the channel's definition.

### Task Lifetime

- Every spawned task has a named owner and a defined end. "It exits when the program does" is only acceptable for a task explicitly documented as running for the process lifetime.
- Store the `JoinHandle`. A detached task you cannot cancel or await is a leak with extra steps.
- On shutdown: stop accepting new work, signal in-flight work, await with a bounded timeout, then force-stop. All four steps, in that order.

### Testing Async

- Test against the runtime you ship, not a mock executor.
- Test the cancellation path explicitly. Drop the future mid-flight and assert the invariants that should survive.
- Test the "rapid change" path: fire the triggering event many times in quick succession and assert bounded resource use. This is where latest-wins bugs, queue growth, and process leaks show up, and it is essentially never tested by accident.
- Test shutdown with work in flight.

### Sync Concurrency

When you use threads instead — which you should, more often than you do:

- Prefer ownership transfer and channels over shared mutable state.
- Keep lock scopes small and lexically obvious; prefer a block that ends with the guard than a guard living to end of function.
- `std::thread::scope` for structured parallelism with borrowed data. No `Arc` ceremony required.
- `rayon` for data parallelism. It is the right answer far more often than an async runtime.
- Document lock ordering anywhere two locks can be held at once. Or restructure so they can't.

---

## Defect Classes That Keep Shipping

This is not a theoretical list. Every entry here is a bug class that has passed human review, passed CI, passed multiple automated reviewers, and shipped in code I own. They pass review because each one looks like ordinary code at the diff level and only misbehaves in a state the reviewer did not think to construct.

**Read this list before reviewing anything that touches processes, concurrency, rendering, caching, or external input.** These are the questions that actually catch bugs; the generic "are there tests?" question does not.

### 1. Cancellation That Does Not Cancel

**Shape:** code cancels an operation by dropping a handle, aborting a task, or relying on a `Drop` impl, and the underlying work continues.

**Real instances:**
- A preview command runs through a shell with `kill_on_drop(true)`. The shell dies; the pipeline it spawned, and the `sleep 30 && ...` behind it, keep running. Every selection change leaks another process tree.
- Work moved to `spawn_blocking` for a decode. The wrapper task is aborted on selection change; the decode runs to completion anyway, on a pool thread, holding its input buffer.

**The rule:** cancellation must reach the thing doing the work. External processes go in their own process group and the group gets signalled. Blocking work either polls a cancellation flag or is bounded such that running to completion is acceptable — and you state which.

**Review question:** "When this is cancelled, what is still running?"

### 2. Unbounded Retention Under Rapid Input

**Shape:** each event allocates or retains something; events can arrive faster than they are consumed; nothing bounds the accumulation.

**Real instance:** holding the arrow key down changes the selection faster than previews decode. Each pending decode retains its input bytes, capped at 32 MB each. Nothing bounded the number of pending decodes. Memory exhaustion, from a keypress.

**The rule:** where only the newest matters, use a single-slot latest-wins handoff. Where all of them matter, use a bounded channel and define what happens when it is full. "Unbounded" in a type name is a design decision requiring justification.

**Review question:** "What happens if this event fires 200 times in two seconds?"

### 3. Work on the Event Loop

**Shape:** something expensive is awaited or called inside the loop that also handles input, so input stops being handled.

**Real instance:** image decode awaited inline in the input loop. Typing froze until decode finished. It was invisible in testing because the test images were small.

**The rule:** the loop that reads input does input. Anything with unbounded or input-dependent cost happens elsewhere and reports back through a channel.

**Review question:** "What is the worst-case time for one iteration of this loop?"

### 4. Idle Work

**Shape:** a timer, a poll interval, or a default tick causes work when nothing has changed.

**Real instance:** an input abstraction's default configuration emitted a render event every 16 ms. The application redrew the entire terminal 60 times per second while sitting idle, because nobody looked at the default.

**The rule:** event-driven means driven by events. Defaults from a library are decisions you have made; review them like your own code.

**Review question:** "With no user input and nothing pending, what is this process doing?"

### 5. Incomplete State-Transition Handling

**Shape:** a state machine handles the transitions someone thought of, and leaves visible or logical residue on the ones they did not.

**Real instance:** a preview pane rendering images to a terminal graphics protocol. Image→image was handled. Image→text was not, so the old image stayed painted underneath the new text. Then image→loading was found. Then image→hidden.

**The rule:** enumerate the transitions as an actual set. If the states are `{image, text, loading, hidden}`, there are sixteen ordered pairs; say what each does, even if the answer is "nothing." An enum plus an exhaustive `match` makes the compiler do this for you, which is the entire reason to use an enum.

**Review question:** "What are all the states, and which transitions are handled?"

### 6. Redundant, Racing, or Non-Atomic Frame Composition

**Shape:** a visible or externally observable update is composed of several steps that are not committed as one.

**Real instances:**
- A screen clear issued outside a synchronized-update block, so the cleared frame is briefly visible before the content arrives. The user sees a flash.
- A retry loop that re-checks state that cannot change during the loop, so it always runs twice, so it always clears and redraws twice, so it flickers.

**The rule:** externally visible updates are committed atomically where the platform provides a mechanism. Loops that retry must depend on something the loop body actually changes.

**Review question:** "Can a partial version of this update be observed?"

### 7. Error Handling That Discards the Successful Part

**Shape:** an operation partially succeeds; the code sees the non-zero status and throws away the useful output.

**Real instances:**
- A preview reads at most 32 MB from a command, closes the pipe, and the producer exits with `SIGPIPE`. The code saw a failure exit status and reported "exited with status 141" instead of showing the 32 MB it had successfully captured.
- A truncation marker appended on the success path but not the failure path, so truncated error output was silently presented as complete.

**The rule:** partial success is a state, and it is usually the state the user cares about. Handle "we got data *and* a failure signal" explicitly. Never present truncated output without saying it was truncated.

**Review question:** "If this fails halfway, what did we already have, and did we throw it away?"

### 8. Empty and Degenerate Inputs Treated as Errors

**Shape:** a guard rejects a valid degenerate case because the author only pictured the normal case.

**Real instance:** clipboard content rendered from HTML to plain text can legitimately be zero bytes (the HTML was entirely markup, or entirely hidden content). A zero-byte guard turned a successful copy into an error and left the UI hanging.

**The rule:** for every input, ask what empty, zero, one, maximum, and "valid but weird" mean. Zero is a value. An empty string is a string. An empty list is a list. Only reject it if the *domain* says it is invalid, not because it looks suspicious.

**Review question:** "Is empty an error here, or just empty?"

### 9. Indices That Mean Two Different Things

**Shape:** a value derived from a filtered/sorted/rendered view is used where a value from the original source was meant.

**Real instance:** an ordinal placeholder documented as "the input line number" was populated with the index into the currently visible, fuzzy-sorted list. It matched during testing because the test data was unfiltered.

**The rule:** if two ordinals exist, they get two named types, not two `usize`s. `SourceIndex(usize)` and `DisplayRow(usize)` cannot be confused; two `usize`s always will be, eventually, in the case you did not test.

**Review question:** "Which index space is this in, and who guarantees it?"

### 10. Cache Invalidation That Does Not Notify

**Shape:** an error or eviction updates one representation of state and not the other, so the rest of the system continues believing something stale.

**Real instance:** an image encode failed, so the entry was evicted from the cache — but the content-kind state still said "image," so the code that clears the graphics layer on image→text transitions never fired, and the failed image stayed on screen over the fallback panel.

**The rule:** state that must agree lives in one place. If it cannot, invalidation must propagate as an explicit signal, and the propagation must be tested.

**Review question:** "Who else believes something about this that just became false?"

### 11. Shell and Argument Injection Through Interpolation

**Shape:** user-controlled values are pasted into a command string, and quoting is handled by "being careful."

**The rule:** pass user values through the environment or as `argv` elements, never by string interpolation into a shell command. If a template language must support placeholders, the placeholders expand to references to already-exported values, and the expansion is context-aware (a placeholder inside single quotes, a heredoc, or an arithmetic context is not the same as one in a double-quoted string). Secret-shaped values — passwords, tokens, query text in a password prompt — are never exported at all.

**Review question:** "What happens if this value contains `$( )`, a newline, or a single quote?"

### 12. Silent Fallbacks

**Shape:** `unwrap_or_default()`, `.ok()`, `if let Ok(x)` with no `else`, a `match` arm that returns a neutral value — and the failure disappears.

**The rule:** every discarded error is a decision. Make it visible at the site: log it, degrade explicitly, or propagate it. If the correct behavior really is "ignore," write a comment saying why ignoring is correct.

**Review question:** "Where did that error go?"

### The Meta-Rule

Every one of these shipped because review checked whether the code was *reasonable* rather than whether it was *complete*. Reasonable code passes review. Complete code enumerates its states, bounds its resources, cancels what it started, and handles the boring cases as deliberately as the interesting ones.

When you fix one of these, the fix is not the patch. The fix is the narrowest lasting guard: a type that makes the confusion impossible, a bounded channel instead of an unbounded one, an exhaustive `match` instead of an `if`, or a regression test that reproduces the exact sequence. A patch without a guard means you will see this bug again under a different name.

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

## Reading Real Code: Worked Examples

Rules are abstract. This is what I mean, applied to code that already passed review.

### Example 1: Ceremony Hiding a Cost

Here is a storage layer over an embedded key-value store. It compiles, it is warning-free, it is "idiomatic," and it was written by someone who knew what they were doing.

```rust
pub(crate) fn list(&self) -> Result<Vec<HiddenEntry>> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
    let mut entries = Vec::new();

    for row in table.iter()? {
        let (id_guard, value_guard) = row?;
        entries.push(decode_entry(id_guard.value(), value_guard.value())?);
    }

    Ok(entries)
}

pub(crate) fn entry_keys(&self) -> Result<HashSet<EntryKey>> {
    self.list().map(|entries| {
        entries
            .into_iter()
            .map(|entry| entry.entry_key().clone())
            .collect()
    })
}
```

**What is wrong with it.**

`entry_keys` reads every row in the table, allocates a `Vec`, fully decodes every entry — every field, every string, every owned allocation inside `HiddenEntry` — then clones one field out of each and throws the rest away. To answer a set-membership question. On a table of a few hundred entries this is invisible; it is still work that exists for no reason, and it is a shape that gets copied to the next place, where the table is bigger.

The second problem is that `list()` looks like a cheap accessor. `entry_keys` reads like a projection. Neither name says "this decodes and allocates the entire table." The cost is real and the names hide it, which is the thing [The Explicitness Doctrine](#the-explicitness-doctrine) exists to prevent.

The guard ceremony (`id_guard.value()`, `value_guard.value()`, the `row?` destructure) is the library's API, not the author's fault, and it is genuinely ugly. That is Rust. The correct response is to keep it in one place at the boundary — not to let it dictate the shape of everything above it.

**What it should be.** Decode what you need, once, at the level that knows what is needed:

```rust
/// Decodes every stored entry. Allocates one `HiddenEntry` per row; prefer a
/// narrower query when you only need part of the record.
pub(crate) fn list(&self) -> Result<Vec<HiddenEntry>> { /* as before */ }

/// Reads only the key field from each row. Does not decode entry bodies.
pub(crate) fn entry_keys(&self) -> Result<HashSet<EntryKey>> {
    let read_txn = self.db.begin_read()?;
    let table = read_txn.open_table(HIDDEN_ENTRIES_TABLE)?;

    let mut keys = HashSet::new();
    for row in table.iter()? {
        let (_, value) = row?;
        keys.insert(decode_entry_key(value.value())?);
    }
    Ok(keys)
}
```

Same length. One scan, no wasted decode, no clone, and both doc comments state what the call costs. The ugly part is confined to the two functions that must talk to the store.

### Example 2: The Ceremony Is Not the Problem

```rust
pub(crate) fn remove(&self, id: HiddenEntryId) -> Result<Option<HiddenEntry>> {
    let write_txn = self.db.begin_write()?;
    let removed_entry = {
        let mut table = write_txn.open_table(HIDDEN_ENTRIES_TABLE)?;
        let entry = table
            .get(id.value())?
            .map(|value| decode_entry(id.value(), value.value()))
            .transpose()?;
        if entry.is_some() {
            table.remove(id.value())?;
        }
        entry
    };
    write_txn.commit()?;
    Ok(removed_entry)
}
```

This one is *fine*, and it is worth saying so, because "this looks noisy" is not a defect.

The inner block exists because the table borrow must end before `commit()`. That is not obfuscation, that is the ownership model doing its job and the author making it explicit with a scope. The `.map(...).transpose()?` is the standard way to say "decode it if it is there, and propagate a decode failure" — it is dense, but it is one idiom and it means exactly one thing.

The one real improvement is the double lookup: `get` then `remove` hits the tree twice. If the store exposes a removing-get, use it. If it doesn't, leave this alone.

**The lesson:** learn to tell the difference between code that is noisy because the language is noisy, and code that is noisy because it is doing unnecessary work. The first is a tax you pay and confine. The second is a defect. Reviewers who cannot tell them apart will either wave through the second or waste everyone's time on the first.

### Example 3: The Shape of a Correct Boundary

A pattern worth copying: the ugly, ceremony-heavy, library-shaped code lives in one module whose entire job is that boundary, and everything above it speaks in domain types.

```rust
//! redb-backed persistence for hidden entries.
//!
//! Owns every transaction, guard, and encoding detail for the hidden-entries
//! table. Everything above this module works with `HiddenEntry` and `EntryKey`
//! and never sees a redb type, a guard, or a raw byte slice.
//!
//! Invariant: every write is committed before the function returns; no partial
//! transaction escapes this module.
```

Above that module: no `Guard`, no `begin_read`, no byte slices, no `transpose`. Below it: whatever the library demands. When you swap the store, one module changes.

That is what "confine the ceremony" means, and it is most of what separates a codebase that stays readable from one that turns into transaction-guard soup at every layer.

---

## Code Review Checklist

This is the mechanical pass. It does not replace [Review Standard](#review-standard) or
[Defect Classes That Keep Shipping](#defect-classes-that-keep-shipping) — run those first,
because they are where the real bugs are. Use this list to catch what they miss.

Before merging, ask:

### Correctness

- Does the code do what it claims?
- Are edge cases handled? Empty, zero, one, maximum, valid-but-degenerate?
- Are invariants encoded, or merely hoped for and written in a comment?
- Has the failure path been read, not just the happy path?
- If it can be cancelled, what is left running, open, locked, or half-written?

### Completeness

- Are all the states enumerated, and are all the transitions handled?
- Is every resource's owner and release site identifiable?
- Is everything that can grow, bounded?
- What does this do when nothing is happening?
- What does this do when its triggering event fires 200 times in two seconds?

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

### Dependency & Reuse

- Does the change leverage existing capabilities in the dependency tree rather than inventing custom workarounds?
- Does it respect the internal execution model of underlying crates (e.g., avoiding redundant frame clears on double-buffered UI renderers, avoiding unnecessary custom wrappers)?
- Does it minimize code churn and cognitive review budget by leveraging existing abstractions?

### Explicitness

- Does every name tell the truth about allocation, blocking, mutation, and ownership?
- Does any function reach for ambient state instead of taking it as a parameter?
- Does the file have a header saying what it handles?
- Is there any new macro, one-implementor trait, or blanket impl, and is it justified?

### Hygiene

- Any new warnings?
- Any broad lint suppressions?
- Any dead code, debug output, or TODOs left behind?

---

## What Not To Do

### Hard Bans

These are findings, not opinions. A PR containing one of these does not merge without a written, accepted justification in the PR body.

| Banned | Why |
| --- | --- |
| `mod.rs` in new code | Every file in the project ends up named `mod.rs`; filenames stop carrying information |
| Macros to avoid typing, or to hide control flow | An undocumented private language with worse tooling, on your build critical path |
| A trait with exactly one implementor, written speculatively | Indirection with no polymorphism; a struct with extra steps |
| Ambient globals, mutable statics, thread-locals, or lazy singletons as inputs | Destroys local reasoning; makes signatures lie |
| `unwrap()` in maintained non-test code | Undocumented panic; nobody knows what invariant it assumed |
| `expect()` whose message does not state the invariant | Same, with a false sense of diligence |
| `std::process::exit` outside the binary's top layer | Termination hidden inside a helper |
| Unbounded channels or unbounded in-flight work without justification | Unbounded memory growth under load, discovered in production |
| Holding a lock, guard, or borrow across `.await` | Deadlocks and correctness bugs that only appear under concurrency |
| Blocking I/O or CPU-heavy work on an executor thread | Stalls every task on that thread, including input handling |
| Relying on `kill_on_drop` to cancel a shell or a process tree | Kills the direct child only; leaks the descendants |
| Public fields on invariant-carrying types | The invariant is now unenforceable |
| Boolean trap arguments (`f(x, true, false, true)`) | Unreadable at the call site; use an enum or a parameter struct |
| Blanket `allow(...)` to silence a lint category | Hides real findings alongside the one you meant |
| Fighting a dependency's execution model with manual clears, resets, or extra locks | The library already handles it; you introduced the bug |
| Reimplementing what is already in `std` or in `Cargo.lock` | Costs build time, review time, and carries bugs the original does not |
| `todo!()`, `unimplemented!()`, or an ignored config option on a reachable path | The software lies about what it does |
| Leaking `unsafe`, FFI, or `cfg` details into ordinary business logic | Boundary failure; poisons everything above it |
| Shadowing a binding to smuggle in a state transition | The reader sees one name and two meanings |
| Merging code whose failure and cancellation paths were never read | See [Defect Classes](#defect-classes-that-keep-shipping) |

### Strongly Discouraged

Not automatic blockers, but each one needs a reason:

- `utils.rs` / `helpers.rs` / `common.rs` as permanent storage
- `types.rs` holding every type in the crate, divorced from behavior
- giant iterator chains that need a decoder ring to debug
- `Deref` used to fake inheritance on a domain type
- `.into()` where the target type is not obvious at the call site
- deriving every trait reflexively rather than the ones the type means
- `tokio` with `features = ["full"]`
- making a function `async` because something three layers down is async
- speculative generality: generics, traits, and config for a second caller that does not exist
- premature workspace splits used to paper over bad module boundaries
- clever lifetimes standing in for a cleaner ownership design
- cargo-cult patterns copied from blog posts without evaluating the tradeoff
- `#[allow]` where `#[expect]` would tell you when it becomes obsolete
- panic-driven control flow
- comments that narrate the authoring process, argue with the reader, or explain what an AI was thinking

### Things That Are Not Findings

So reviewers stop wasting time:

- code that is verbose because the library's API is verbose, confined to a boundary module
- a small amount of duplication between two things that have different reasons to change
- an explicit loop where an iterator chain would have been denser
- a `match` with an arm that does nothing, when the arm documents an intentional no-op
- a long function that is a genuinely linear sequence with one abstraction level
- formatting that `rustfmt` produced
- anything covered by "nit:" that is not written down in this document

---

## Example Commands & Automation

### Daily Local Loop

```bash
cargo check          # the loop you should actually be running
cargo nextest run    # or cargo test if nextest is not installed
```

Keep this loop fast. If `cargo check` on a one-line edit takes more than a few seconds,
that is a problem to fix, not a fact of life. See
[Compile Time Is a Feature](#compile-time-is-a-feature).

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

If you're actively developing while online, you should usually already have a **draft PR** open on your branch (see `PROJECT_STANDARDS.md`). Use the checks below before marking it ready for review; if you skipped the draft step, run them before your first PR push.

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
```

### Build Time Health Loop

```bash
cargo build --timings                  # per-crate wall time and critical path
cargo llvm-lines | head -30            # biggest codegen contributors
cargo machete                          # unused dependencies
cargo tree --duplicate                 # same crate at two versions
```

Nightly, when you are actually hunting:

```bash
RUSTFLAGS="-Zmacro-stats" cargo +nightly build
cargo +nightly rustc -- -Z time-passes
```

### Example `.cargo/config.toml`

```toml
[target.x86_64-unknown-linux-gnu]
rustflags = ["-C", "link-arg=-fuse-ld=mold"]

[profile.dev]
debug = "line-tables-only"
split-debuginfo = "unpacked"

[profile.dev.build-override]
opt-level = 3
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

### The Short Version

If you remember nothing else:

- **Compiling is not correct.** Read the failure path, the cancellation path, and the empty-input path before you claim it works.
- **Explicit over implicit, always, unless it is tautological.** No hidden control flow, no hidden allocation, no hidden blocking, no hidden authority, no hidden termination.
- **Every file says what it handles.** `//!` at the top, stating the responsibility and the invariant.
- **No `mod.rs`.** No macros to avoid typing. No one-implementor traits. No ambient globals. No unbounded queues.
- **Read the dependency tree before you write machinery.** The thing you are about to build is probably already there, and deleting your version is the best patch you will write this week.
- **Confine the ceremony to boundary modules.** The library's ugliness stops at the edge; domain code speaks domain types.
- **Compile time and runtime cost are features.** Measure them, budget them, notice when they regress.
- **Async is a tool for many concurrent I/O operations, not a default.** If you use it, you own cancellation, bounding, and process-group cleanup.
- **Cancellation must reach the thing doing the work.** Aborting a task does not stop a `spawn_blocking` job or a shell's children.
- **Bound everything.** Latest-wins where only the newest matters.
- **Enumerate your states.** An enum and an exhaustive `match` make the compiler check the transitions you would have missed.
- **Name the rule, not the mechanics.** If you cannot name it, you do not understand it yet.
- **A little duplication beats the wrong abstraction.** Abstraction must own an invariant.
- **Zero warnings. Small diffs. One concern per change.**
- **Findings state the failure scenario.** A finding you cannot demonstrate is a hypothesis.
- **Break rules deliberately and write down why.** An undocumented exception is a mistake with confidence.

### The Long Version

- use stable, current Rust; keep the toolchain moving
- keep `main.rs` thin and `lib.rs` doing the work
- keep files, functions, and diffs small
- split by responsibility, never by syntax
- prefer explicit inputs over ambient magic
- encode invariants in types; use typestate when it removes real misuse
- keep names honest about ownership, allocation, mutation, blocking, and cost
- return errors, do not panic casually; make exits and failure modes intentional
- validate at the boundary, once, and convert to a validated type
- test at unit, integration, and doc levels; reach for property tests, snapshots, and fuzzing when the surface demands them
- write the regression test that would have caught the bug you just fixed
- document public behavior and non-obvious modules
- treat MSRV, public APIs, and feature flags as compatibility contracts
- record important technical decisions before you forget the alternatives
- keep warnings at zero and lint suppressions narrow and explained
- keep dependencies healthy, pruned, and justified
- choose boring, reversible designs
- instrument systems so they can be debugged in reality, proportional to what they are
- learn from incidents without blame theater
- isolate unsafe, FFI, and platform code behind boundary modules
- delete code aggressively; source control remembers

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

Additional sources behind the 2.0 revision:

- Holzmann, *The Power of Ten: Rules for Developing Safety-Critical Code*  
  https://spinroot.com/gerard/pdf/P10.pdf
- Parnas, *On the Criteria To Be Used in Decomposing Systems into Modules*  
  https://dl.acm.org/doi/10.1145/361598.361623
- How SQLite Is Tested  
  https://www.sqlite.org/testing.html
- corrode: Tips for Faster Rust Compile Times  
  https://corrode.dev/blog/tips-for-faster-rust-compile-times/
- Faultlore (Aria Beingessner) — Rust internals, unsafe, layout, and honest criticism  
  https://faultlore.com/blah/
- Logan Smith (`@_noisecode`), *How to write the perfect function*  
  https://www.youtube.com/watch?v=2OMRWPOSw9s
- Sean Parent, *C++ Seasoning* ("No Raw Loops")  
  https://www.youtube.com/watch?v=W2tWOdzgXHA
- rustc codegen options reference  
  https://doc.rust-lang.org/rustc/codegen-options/
- cargo-nextest  
  https://nexte.st/
- cargo-machete  
  https://github.com/bnjbvr/cargo-machete
- cargo-udeps  
  https://github.com/est31/cargo-udeps
- cargo-llvm-lines  
  https://github.com/dtolnay/cargo-llvm-lines
- mold linker  
  https://github.com/rui314/mold
- cargo-chef  
  https://github.com/LukeMathWalker/cargo-chef

---

End of Standard

