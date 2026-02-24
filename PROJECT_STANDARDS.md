# Project Development & Contribution Standards

> A Manual for Maintaining Sensible Git Discipline Without Sacrificing Productive Chaos

**Document Version:** 1.4.0  
**Last Updated:** 2026-02-23  
**Audience:** Future me, contributors, and anyone brave enough to work on these projects

---

## Table of Contents

1. [Philosophy & Principles](#philosophy--principles)
2. [Related Documentation](#related-documentation)
3. [Branching Strategy](#branching-strategy)
4. [Workflow Overview](#workflow-overview)
5. [Feature Branch Development](#feature-branch-development)
6. [Commit Discipline](#commit-discipline)
7. [Pull Request Process](#pull-request-process)
8. [Code Review & Collaboration Standards](#code-review--collaboration-standards)
9. [Release Management](#release-management)
10. [Versioning Scheme](#versioning-scheme)
11. [Documentation Standards](#documentation-standards)
12. [What Not To Do](#what-not-to-do)
13. [Example Workflows](#example-workflows)
14. [Tooling & Automation](#tooling--automation)

---

## Philosophy & Principles

### Core Tenets

1. **main is sacred: releases but living docs.** It holds tagged, production-ready releases and up-to-date documentation. Code reaches main only via release or hotfix branches; docs go to main (via PR from contributors or anyone), then main is merged into dev so dev stays in sync.
2. **dev is the integration branch.** All code (feature) branches merge here first; release branches are created from dev.
3. **main and dev do not merge directly for code.** Code reaches main only via release or hotfix branches. Main is merged into dev after a hotfix (to sync the fix) or after docs land on main (to sync the docs).
4. **All code changes go through pull requests.** No exceptions. Even for solo work.
5. **Releases are the record; RELEASELOG.md is the in-repo log.** GitHub releases are the canonical record. We do not keep CHANGELOG.md; we keep RELEASELOG.md, updated on each release branch with the release title and body, with a `---` separator between each release.
6. **Commit messages matter at release time.** During development, commit as suits your workflow. Before merging, clean them up.
7. **Flow over formality.** Discipline exists to enable productivity, not strangle it.
8. **Reviews are conversations, not commands.** The goal is to improve, not to dictate.

### Design Intent

This workflow accommodates:
- Extended offline development periods
- Flow state coding sessions with burst commits
- Solo development with occasional collaboration
- Projects where releases happen in polished increments rather than continuous deployment
- Proper review and testing before integration or release

---

## Related Documentation

This document defines the technical Git workflow. See also:

- **[CONTRIBUTING.md](./CONTRIBUTING.md)** - Contributor guide for external contributors (setup, standards, templates)
- **[README.md](./README.md)** - Project overview and quick start
- **[USAGE.md](./USAGE.md)** - Detailed user documentation

---

## Branching Strategy

### Primary Branches

| Branch | Purpose | Push Policy |
|--------|----------|-------------|
| main | Releases and living docs. Every code commit on main is a tagged release; docs are updated on main. | Code reaches main only via release or hotfix branches (not from dev). Docs reach main via PRs (target main) from contributors or maintainers; maintainers may push trivial docs directly. |
| dev | Integration branch. All code (feature) work merges here; release branches are created from dev. | Receives merges **from feature branches** (via PRs) and **from main** after a hotfix or after docs land on main (to sync back). |

### Feature Branches

Code work occurs in feature branches created from dev. **Documentation-only changes** use a branch from main and a PR targeting main (see [Documentation-Only Changes](#documentation-only-changes)).

| Type | Naming | Purpose |
|------|---------|----------|
| Feature | feat/name | New features or functionality |
| Fix | fix/name | Bug fixes |
| Refactor | refactor/name | Code restructuring without changing behavior |
| Docs | docs/name | Documentation (branch from **main**, PR to **main**) |
| Chore | chore/name | Tooling, dependencies, build updates |

### Release Branches

| Type | Naming | Purpose |
|------|---------|---------|
| Release | release/version | Prepare releases with version bumps, docs updates, and final testing |

Release branches are created from dev when a maintainer decides to release. They are used to:
- Freeze a stable point in dev for release preparation
- Update version numbers and release-related docs (README, man pages, etc.) — **no source/code changes**
- Perform final testing before release
- Merge to main (which gets tagged); then merge the release branch back to dev (so dev has the version bump). main and dev do not merge into each other — the release branch is the path to main.

**Release branches are not for code changes.** Only version bumps and release-related doc updates (version refs in README, man page, etc.) belong on the release branch. If you find a bug during release prep: fix it in dev, then either (a) wait for dev to be up to date and merge dev into the release branch to bring in that fix, or (b) ship the release without the fix (the bug stays in that release). Keep the release branch free of new code changes.

**When to create a release branch:**
- A maintainer decides it's time for a release
- dev is in a stable state (no critical bugs, features are complete)
- All planned features for the release are merged into dev

### Hotfix Branches

| Type | Naming | Purpose |
|------|---------|----------|
| Hotfix | hotfix/version | Emergency patches for production issues only |

Hotfix branches are created from main, go through a PR to main, and are **exceptions, not the rule** — hotfixes are rare. Do not confuse hotfixes with normal bug fixes; normal fixes go through dev → release branch → main.

**Process:** The contributor opens a hotfix PR with the code fix only. The **maintainer** adds a commit on the hotfix branch (or before merge) to bump the version in Cargo.toml, README, and other refs, then merges the PR to main. There is no release branch: the maintainer creates the tag and GitHub release for that hotfix. Then **main is merged into dev** so the fix (and version bump) is present in development. This is one of the two cases where main is merged into dev (the other is after a docs-only change to main).

**Reverts** of a bad release are treated like hotfixes (branch from main, revert, version bump by maintainer, merge to main, tag and release, then merge main into dev).

**When merging main into dev (after a hotfix or docs-only change):** If there are conflicts, prioritize the hotfix side for code (the hotfix knows what was changed). Docs conflicts matter less since release branches will result in doc review anyway. For code conflicts, the person merging main into dev resolves them; context-dependent.

### Documentation-Only Changes

**Docs go through main.** Documentation-only changes (typo fixes, clarifications, formatting, correctness updates like fixing example syntax) are made against **main**, not dev. Contributors and anyone else open a PR **targeting main** for docs; after merge, main is merged into dev so dev has the latest docs. Everything that gets merged into dev from main is either docs (from docs PRs) or hotfix code — release branches only bring version bumps and release-related doc updates when they merge back to dev, so keeping docs on main keeps "living docs" in one place.

**Criteria for docs-only:**
- Changes only to `.md` files (README, USAGE, CONTRIBUTING, etc.) or other doc assets
- No source code or config file changes that affect behavior
- Typo fixes, grammar, formatting, clarifications, fixing outdated examples (e.g. updated Hyprland syntax)

**Process (contributor or maintainer):**
```bash
git checkout main
git pull origin main
git checkout -b docs/fix-usage-hyprland   # or docs/typo-readme, etc.
# Make documentation changes
git add -A && git commit -m "docs: fix Hyprland windowrule syntax in README"
git push origin docs/fix-usage-hyprland
# Open PR targeting main (not dev)
# After PR is merged to main:
git checkout dev
git merge main
git push origin dev
```

Maintainers may push trivial docs fixes directly to main and then merge main into dev; for anything that deserves review, use a PR to main.

---

## Workflow Overview

**Rule: main and dev do not exchange code directly.** Code reaches main only via release or hotfix branches. Main is merged into dev after a hotfix (to sync the fix) or after docs land on main (to sync the docs).

```
main (production releases only)
  |
  └── merge from release/v2.x.x (at release time) ← then tag (e.g. 2.5.0) and create GitHub release with body
       |
       release/v2.x.x (release preparation branch)
         |
         └── created from dev (freeze point)
              |
              dev (integration branch)
                |
                ├── PR merge from feat/feature-one
                ├── PR merge from fix/bug-fix
                └── PR merge from feat/feature-two
                     |
                     feat/feature-two (development work happens here)
```

Standard process:

1. Create feature branch from dev
2. Develop locally
3. Push feature branch to remote
4. Open pull request targeting dev
5. Get review and approval
6. Merge PR to dev
7. Maintainer creates release branch from dev when ready
8. Maintainer merges release branch to main
9. Tag and push
10. Maintainer merges release branch back to dev

---

## Feature Branch Development

### Creating a Feature Branch

```bash
git checkout dev
git pull origin dev
git checkout -b feat/detach-mode
```

### During Development

- Commit as you work, don't obsess over perfection.
- "wip" and "temp fix" are valid local commits.
- Code explains *what*, commits should explain *why*.
- Work offline freely — rebase and clean up later.

Example:

```bash
git commit -am "wip detach logic"
git commit -am "detach working with uwsm"
git commit -am "fix crash on exit"
```

### Preparing for Pull Request

Before opening a PR:

1. Rebase on latest dev:
   ```bash
   git fetch origin
   git rebase origin/dev
   ```

2. Clean commit history:
   ```bash
   git rebase -i origin/dev
   ```

3. Run all checks:
   ```bash
   cargo fmt
   cargo clippy
   cargo test
   cargo build --release
   ```

4. Push branch:
   ```bash
   git push origin feat/detach-mode
   ```

---

## Commit Discipline

Follow **Conventional Commits**:

```
type(optional-scope): short description

[optional body]

[optional footer]
```

Examples:

```bash
feat(detach): implement --detach flag with systemd-run support
fix(db): enforce foreign key constraints properly
refactor(cache): move batch operations to separate module
docs(usage): add examples for dmenu mode
chore: update flake.nix to use naersk
```

| Type | Meaning |
|------|---------|
| feat | New feature |
| fix | Bug fix |
| docs | Documentation only |
| refactor | Code restructuring |
| perf | Performance improvement |
| chore | Build, deps, tooling |
| test | Testing only |
| style | Whitespace, formatting |
| revert | Undo a commit |

---

## Pull Request Process

Code changes enter via pull requests to dev; documentation changes enter via pull requests to main (or maintainer push to main for trivial docs). No direct code pushes to main or dev.

### Opening a Pull Request

1. Push feature branch:
   ```bash
   git push origin feat/detach-mode
   ```

2. Open PR on GitHub:
   - **Code:** Base dev, compare your feature branch. Use the PR template below.
   - **Docs:** Base main, compare your docs branch (see [Documentation-Only Changes](#documentation-only-changes)).

### PR Template

**Title:**

```
feat: feature small detail
```

**Body:**

```markdown
## Summary
Brief description of what this PR does and why.

- [ ] I did basic linting
- [ ] I'm a clown who can't code 🤡

## Changes
- Added tag filtering UI
- Implemented tag persistence in database
- Updated documentation

## Testing
1. Build with cargo build --release
2. Run fsel --cclip and verify tags appear
3. Test tag persistence across sessions

## Breaking Changes
None

## Related Issues
Closes #42
```

### Draft Pull Requests

Use draft PRs for early feedback:
- Open as draft when code is incomplete but you want early review
- Mark ready for review when complete
- Useful for architectural discussions before full implementation

### Work-in-Progress PRs

Mark PRs as WIP by:
- Opening as draft on GitHub
- Adding `[WIP]` prefix to title if draft not available
- Adding `wip` label if available

---

## Code Review & Collaboration Standards

### Intent

Reviews exist to improve quality and maintain shared understanding — not to police style or waste time.
They're conversations between equals, not bureaucratic rituals.

### Roles

| Role | Responsibilities |
|------|------------------|
| **Maintainer** | Final review, merge approval, release tagging |
| **Collaborator** | Reviews code, tests features, requests changes |
| **Contributor** | Writes code, submits PRs, responds to feedback |

### Review Workflow

1. **PR Opened:** Target branch is always `dev`.
2. **Assign Reviewers:** Maintainers or collaborators review PRs.
3. **Review Comments:** Reviewers can mark feedback as *blocking* or *non-blocking*.
4. **Discussion:** Feedback is addressed; code may be amended and re-pushed.
5. **Approval:** One approval from a maintainer or designated collaborator required.
6. **Merge:** Use *Squash and Merge* unless multiple commits are meaningful.
7. **Tag and release (when merging to main):** Only maintainers create the version tag (version number only) and the GitHub release (title [version-codename], body from template).

### Review Timeline

| Stage | Expected Response |
|-------|-------------------|
| Initial response | A few hours to a few days |
| Full review | Within 1 week |
| Merge after approval | Within 1-2 days |

### Review Criteria

| Aspect | Expectation |
|--------|-------------|
| Correctness | The code does what it claims |
| Clarity | Another dev can understand it |
| Impact | Doesn't introduce regressions |
| Security | No obvious vulnerabilities |
| Style | Matches existing conventions |
| Documentation | Updated if needed |

### Feedback Etiquette

- Comment with **why**, not just "change this".
- Nitpicks = non-blocking.
- If it's broken, mark **Request Changes**.
- Prefer questions over commands:
  > "Could this be simplified?" not "Simplify this."

### Internal Merging

For PRs requiring significant refactoring or integration with ongoing work:
- Maintainers may merge changes internally as part of larger efforts
- Original contributor receives full attribution in commit messages and release notes
- This is not a rejection, but integration into active development
- Contributor will be notified and credited

Example response to contributor:
> "Thank you for this contribution. Your approach is better than the current implementation. I'm currently refactoring the project structure, so I'll be merging this internally as part of that effort. You'll be credited in the commit message and release notes when it ships."

### Stale Pull Requests

- PRs without activity for **30 days** will be marked stale
- Stale PRs will be closed after **14 additional days** of inactivity
- Exception: PRs marked as `work-in-progress` or `on-hold` by maintainers
- Closed PRs can be reopened if work resumes

### Sanity Checks

Integrate automated checks:
- `cargo fmt`, `cargo clippy`
- Commit message linter (optional)
- GitHub branch protection:
  - Require 1 review before merge (for code PRs; docs PRs to main may be configured per preference)
  - Require passing checks
  - Restrict direct pushes to main/dev (exceptions for maintainer docs to main if desired)

---

## Release Management

### When to Create a Release Branch

A maintainer creates a release branch when:
- They decide it's time for a release
- dev is in a stable state (no critical bugs, features are complete)
- All planned features for the release are merged into dev

**Important:** Release branches freeze a specific point in dev, allowing ongoing PRs to continue merging into dev without affecting the release preparation. Release branches are for version bumps and docs only — no code changes (see [Release Branches](#release-branches): if you find a bug, fix in dev and either merge dev into the release branch or ship without the fix).

**Tags are created during the release stage:** When you are ready to release, you create the release branch, do version bumps and final testing, then merge the release branch to main. Only after that merge do you create the tag and publish the release. The release (GitHub release) is the canonical record of what shipped; RELEASELOG.md in the repo is updated on each release branch with the title and body (see [Documentation Standards](#documentation-standards)). The tag is a simple version-number pointer to that commit.

### Preparation

1. **Merge main into dev** so dev has the latest docs (docs live on main and are synced to dev via main → dev).
2. Ensure all feature PRs for the release are merged into dev.
3. Confirm all tests pass on dev.
4. Create a release branch from dev (this freezes the release point):
   ```bash
   git checkout dev
   git pull origin dev
   git checkout -b release/v3.0.0-kiwicrab  # Replace with actual version
   ```
5. Update version references on the release branch:
   - `Cargo.toml` (root directory)
   - `flake.nix` (root directory)
   - `README.md` (installation instructions, if needed)
   - Man pages (`fsel.1` or similar)
   - Example configs if they contain version info
6. Commit version bump:
   ```bash
   git commit -am "chore: bump version to 3.0.0-kiwicrab"
   ```
7. Prepare release notes using the [Release body template](#release-body-template) below; update **RELEASELOG.md** on the release branch with the release title and body, adding a `---` separator above the previous release(s).
8. Verify [Semantic Versioning 2.0.0](https://semver.org/) compliance.
9. Run final tests on the release branch:
   ```bash
   cargo test
   cargo build --release
   ```

### Process

```bash
# 1. Merge release branch to main
git checkout main
git pull origin main
git merge release/v3.0.0-kiwicrab

# 2. Tag the release (tag = version number only, no codename)
git tag -a 3.0.0 -m "3.0.0"
git push origin main --tags

# 3. Merge release branch back to dev (so dev has the version bump)
git checkout dev
git merge release/v3.0.0-kiwicrab
git push origin dev

# 4. Delete the release branch only after it is merged to both main and dev
git branch -d release/v3.0.0-kiwicrab
git push origin --delete release/v3.0.0-kiwicrab
```

**Always merge the release branch to both main and dev before deleting it.** If you already deleted the release branch, merge main into dev once as a one-off recovery.

**Why this workflow:**
- main and dev stay independent; only release branches (and hotfixes) connect them
- dev continues accepting PRs during release preparation
- Release work is isolated on the release branch
- No conflicts from ongoing development
- Clear freeze point for the release
- dev gets version bumps by merging the release branch back, not by merging main

### GitHub Release

**Release title:** Use exactly `[version-codename]` in brackets, e.g. `[3.0.0-kiwicrab]`. No date or extra text in the title.

**Git tag:** Use the version number only (no codename), e.g. `3.0.0`, `2.5.0`, `2.4.0`. Create the tag on main after the release branch is merged; then create the GitHub release from that tag and paste the release body below (same content as in RELEASELOG.md for this release).

### Release body template

The release body (and the block you add to RELEASELOG.md) uses this structure (omit sections that don’t apply). Prefer bullets under each heading; use sub-bullets for detail. Optionally cite PRs (e.g. “from pr #23”).

```markdown
[3.0.0-kiwicrab] Latest

Breaking changes

- Database and cache format
  - Serialization changed from X to Y. Existing data is not migrated. On first run after upgrading, do Z.

Added

- Feature name
  - Bullet points describing what was added; optional "from pr #N".
- Another feature
  - Details.

Changed

- Area or component
  - What changed and why it matters.
- Dependencies / build
  - List dependency or tooling changes.

Fixed

- Brief description of fix: what was wrong and what users get now.
- Another fix.

Technical details

- Implementation notes for developers (optional): key algorithms, storage format, config keys, etc.

Documentation

- README: what was updated.
- Man page / USAGE / CONTRIBUTING: what was updated.

Notes

- SemVer: this is a MAJOR/MINOR/PATCH release because …
- Rationale: why this release matters in one or two sentences.

Contributors

- @handle1
- @handle2
- Co-authored-by: @bot (if applicable)

Compatibility

- Language/runtime: e.g. Rust 1.89+ (unchanged).
- Platforms: e.g. GNU/Linux and *BSD (unchanged).
- Config / database: compatible or breaking summary.
- Breaking: if applicable, what users must do (back up, re-pin, etc.).

```

**Section rules:**
- **Breaking changes** — If present, call out at top and again in Notes/Compatibility. Be explicit about migration or “no migration.”
- **Added / Changed / Fixed** — Use for user-visible and dependency/build changes. Link to PRs when helpful.
- **Technical details** — Optional; use for implementation notes that help contributors or integrators.
- **Documentation** — What doc files were updated and for what (version refs, new options, examples).
- **Notes** — SemVer rationale and short rationale for the release.
- **Contributors** — List everyone who contributed (and co-authors if you credit them).
- **Compatibility** — Runtime, platforms, config/DB, and any breaking migration steps.

---

## Versioning Scheme

Semantic Versioning 2.0.0 + optional codename.

**Display version (codename):** `major.minor.patch-codename`, e.g. `3.0.0-kiwicrab`.

**Git tag:** Version number only — `3.0.0`, `2.5.0`, `2.2.3`. No codename and no `v` prefix unless the project convention is to use `v` (e.g. `v3.0.0`).

**Release title (GitHub / RELEASELOG.md):** `[version-codename]` in brackets, e.g. `[3.0.0-kiwicrab]`.

### Semantic Versioning Rules

| Change | Increment | Example |
|--------|-----------|---------|
| Breaking | MAJOR | 1.5.3 → 2.0.0 |
| Feature | MINOR | 1.5.3 → 1.6.0 |
| Fix | PATCH | 1.5.3 → 1.5.4 |

### Codename Policy

**Codenames are updated when MAJOR version changes:**
- Codenames provide personality and memorable release identifiers
- New codename chosen at each MAJOR version bump (e.g., 2.x.x → 3.0.0)
- **Only maintainers** choose and update codenames
- Codenames persist across MINOR and PATCH versions within same MAJOR version

**Examples:**
- `2.0.0-seedclay`, `2.1.0-seedclay`, `2.5.0-seedclay` all use "seedclay"
- When `3.0.0` is released, new codename chosen: `3.0.0-kiwicrab`
- Then `3.0.0-kiwicrab`, `3.1.0-kiwicrab`, `3.2.0-kiwicrab` all use "kiwicrab"
- When `4.0.0` is released, new codename chosen again

### Pre-release Suffixes

For unstable or beta releases:
- `v2.0.0-alpha`
- `v2.0.0-beta`
- `v2.0.0-rc.1` (release candidate)

These can be combined with codenames:
- `v3.0.0-alpha-kiwicrab`

### LTS and next / parallel release lines

If you maintain permanent branches for LTS and "next" (or similar) cycles, treat them like separate mains: each has its own release flow, tags, and versioning. Mark everything relating to them with `lts`, `next`, or alike — branch names, versioning (e.g. 2.x LTS vs 3.x next), and docs. Same rules apply per line; they do not merge into each other except by explicit policy (e.g. cherry-picks, or not at all).

---

## Documentation Standards

### Minimum

1. `README.md` — overview, install, usage
2. `USAGE.md` — detailed guide (if needed)
3. `LICENSE` — BSD-2-Clause or similar
4. **RELEASELOG.md** — in-repo release log. We do not keep CHANGELOG.md. On each release branch, prepend the release title and body to RELEASELOG.md, with a `---` separator between each release (newest at top).

### Code Docs

- Every public API documented.
- Explain *why*, not *what*.
- Include examples for non-obvious APIs.

### Configs

Provide annotated example configs with defaults.

### Man Pages

Keep CLI man pages in sync with `--help`.
Generate from Markdown if possible using tools like `ronn` or `pandoc`.

---

## What Not To Do

**Absolutely Forbidden**

- Push code directly to main or dev (docs go to main via PR, or maintainer push for trivial docs)
- Merge code without a PR
- Release without testing
- Dump raw git logs as the release body
- Ignore version updates in all relevant files

**Strongly Discouraged**

- Inconsistent versioning
- Unreviewed breaking changes
- Merging with failing tests
- Leaving PRs without response for extended periods

---

## Example Workflows

### Standard Feature

```bash
git checkout dev
git pull origin dev
git checkout -b feat/detach-mode
# develop, commit freely
git push origin feat/detach-mode
# open PR to dev, review, merge
```

### Release

```bash
# 1. Create release branch from dev
git checkout dev
git pull origin dev
git checkout -b release/3.0.0-kiwicrab

# 2. Update version numbers in Cargo.toml, flake.nix, etc.
# ... edit files ...
git commit -am "chore: bump version to 3.0.0-kiwicrab"

# 3. Final testing
cargo build --release
cargo test

# 4. Merge to main, tag (version number only), then create GitHub release
git checkout main
git pull origin main
git merge release/3.0.0-kiwicrab
git tag -a 3.0.0 -m "3.0.0"
git push origin main --tags
# Create GitHub release: tag 3.0.0, title [3.0.0-kiwicrab], body = release notes from template

# 5. Merge back to dev
git checkout dev
git merge release/3.0.0-kiwicrab
git push origin dev

# 6. Clean up
git branch -d release/3.0.0-kiwicrab
git push origin --delete release/3.0.0-kiwicrab
```

### Hotfix

Hotfixes are exceptions: no release branch. Maintainer bumps version before merging.

```bash
# Contributor: branch from main, fix only
git checkout main
git pull origin main
git checkout -b hotfix/cache
# fix issue (no version bump in this commit)
git commit -am "fix: prevent cache corruption"
git push origin hotfix/cache
# Open PR to main

# Maintainer: add version bump (Cargo.toml, README, etc.) on hotfix branch, then merge PR
# ... bump to 3.0.1, commit, push ...
# Merge PR to main

# Maintainer: tag and release (no release branch)
git tag -a 3.0.1 -m "3.0.1"
git push origin main --tags
# Create GitHub release: title [3.0.1-kiwicrab], body = release notes

# Merge main into dev
git checkout dev
git merge main
git push origin dev
```

### Documentation-Only Change (PR to main)

```bash
# Branch from main, PR targets main (contributor or maintainer)
git checkout main
git pull origin main
git checkout -b docs/fix-hyprland-syntax
# Edit USAGE.md or README, etc.
git add -A && git commit -m "docs: fix Hyprland windowrule syntax in README"
git push origin docs/fix-hyprland-syntax
# Open PR targeting main (not dev)
# After PR is merged to main:
git checkout dev
git merge main
git push origin dev
```

For trivial typo fixes, maintainers may push directly to main and then merge main into dev the same way.

---

## Tooling & Automation

### Git Aliases

```ini
[alias]
st = status -sb
lg = log --oneline --graph --decorate --all
cleanup = rebase -i origin/dev
oops = commit --amend --no-edit
tags = tag -l --sort=-version:refname --format='%(refname:short) %(creatordate:short)'
sync = !git fetch origin && git rebase origin/dev
```

### Hooks

Example `.git/hooks/pre-commit`:

```bash
#!/bin/bash
cargo fmt -- --check || exit 1
cargo clippy -- -D warnings || exit 1
```

### Continuous Integration

Consider adding GitHub Actions for:
- Automated formatting checks (`cargo fmt --check`)
- Linting (`cargo clippy`)
- Testing (`cargo test`)
- Release builds (`cargo build --release`)

Example `.github/workflows/ci.yml` structure:
```yaml
name: CI
on: [push, pull_request]
jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Format check
        run: cargo fmt -- --check
      - name: Clippy
        run: cargo clippy -- -D warnings
      - name: Tests
        run: cargo test
```

---

## Summary

This workflow forces sanity:

- All work through branches & PRs (code to dev, docs to main)
- dev is your proving ground for code
- main is sacred: releases and living docs
- Reviews are collaborative, not confrontational
- RELEASELOG.md and GitHub releases are the record; tags point at releases

Future you: if it's 2am and you're wondering how to do this properly — do it this way. You'll thank yourself later.

---

End of Standard
