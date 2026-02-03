# Project Development & Contribution Standards

> A Manual for Maintaining Sensible Git Discipline Without Sacrificing Productive Chaos

**Document Version:** 1.2.0  
**Last Updated:** 2025-10-27  
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

1. **main is sacred.** Never push directly. It contains only tagged, production-ready releases.
2. **dev is the integration branch.** All feature branches merge here first.
3. **All code changes go through pull requests.** No exceptions. Even for solo work.
4. **Releases are the changelog.** GitHub releases serve as the historical record. No separate CHANGELOG.md file is maintained.
5. **Commit messages matter at release time.** During development, commit as suits your workflow. Before merging, clean them up.
6. **Flow over formality.** Discipline exists to enable productivity, not strangle it.
7. **Reviews are conversations, not commands.** The goal is to improve, not to dictate.

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
| main | Stable, production-ready code. Every commit is a tagged release. | Never push directly. Merge only from release branches after testing and tagging. |
| dev | Integration branch. All features merge here before release branches. | Never push directly. Only receives merges from feature branches via pull requests. |

### Feature Branches

All work occurs in feature branches created from dev.

| Type | Naming | Purpose |
|------|---------|----------|
| Feature | feat/name | New features or functionality |
| Fix | fix/name | Bug fixes |
| Refactor | refactor/name | Code restructuring without changing behavior |
| Docs | docs/name | Documentation changes |
| Chore | chore/name | Tooling, dependencies, build updates |

### Release Branches

| Type | Naming | Purpose |
|------|---------|---------|
| Release | release/version | Prepare releases with version bumps and final testing |

Release branches are created from dev when a maintainer decides to release. They are used to:
- Freeze a stable point in dev for release preparation
- Update version numbers in all relevant files
- Perform final testing before release
- Merge to main (which gets tagged) and then back to dev

**When to create a release branch:**
- A maintainer decides it's time for a release
- dev is in a stable state (no critical bugs, features are complete)
- All planned features for the release are merged into dev

### Hotfix Branches

| Type | Naming | Purpose |
|------|---------|----------|
| Hotfix | hotfix/version | Emergency patches for production issues |

Hotfix branches are created from main, merged back into main after patching, then merged into dev.

### Documentation-Only Changes

**Exception to PR workflow:** Documentation-only changes (typo fixes, clarifications, formatting) that don't affect code can be pushed directly to main without going through dev.

**Criteria for direct-to-main docs:**
- Changes only to `.md` files (README, USAGE, CONTRIBUTING, etc.)
- No code changes whatsoever
- No config file changes
- Typo fixes, grammar corrections, formatting improvements
- Clarifications that don't change meaning

**Process:**
```bash
git checkout main
git pull origin main
# Make documentation changes
git commit -m "docs: fix typo in README"
git push origin main
# Sync to dev
git checkout dev
git merge main
git push origin dev
```

**Note:** For substantial documentation rewrites or new documentation, still use the PR process for review.

---

## Workflow Overview

```
main (production releases only)
  |
  └── merge from release/v2.x.x (at release time) ← tag applied here
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

All changes enter the project via pull requests — no direct pushes to main or dev.

### Opening a Pull Request

1. Push feature branch:
   ```bash
   git push origin feat/detach-mode
   ```

2. Open PR on GitHub:
   - Base: dev
   - Compare: feat/detach-mode
   - Use the PR template below

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
7. **Tag Release (when merging to main):** Only maintainers tag versions.

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
  - Require 1 review before merge
  - Require passing checks
  - Disallow direct pushes to main/dev

---

## Release Management

### When to Create a Release Branch

A maintainer creates a release branch when:
- They decide it's time for a release
- dev is in a stable state (no critical bugs, features are complete)
- All planned features for the release are merged into dev

**Important:** Release branches freeze a specific point in dev, allowing ongoing PRs to continue merging into dev without affecting the release preparation.

### Preparation

1. Ensure all feature PRs for the release are merged into dev.
2. Confirm all tests pass on dev.
3. Create a release branch from dev (this freezes the release point):
   ```bash
   git checkout dev
   git pull origin dev
   git checkout -b release/v3.0.0-kiwicrab  # Replace with actual version
   ```
4. Update version references on the release branch:
   - `Cargo.toml` (root directory)
   - `flake.nix` (root directory)
   - `README.md` (installation instructions, if needed)
   - Man pages (`fsel.1` or similar)
   - Example configs if they contain version info
5. Commit version bump:
   ```bash
   git commit -am "chore: bump version to 3.0.0-kiwicrab"
   ```
6. Prepare release notes following [Keep a Changelog](https://keepachangelog.com/) format.
7. Verify [Semantic Versioning 2.0.0](https://semver.org/) compliance.
8. Run final tests on the release branch:
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

# 2. Tag the release
git tag -a v3.0.0-kiwicrab -m "v3.0.0-kiwicrab: release notes here"
git push origin main --tags

# 3. Merge release branch back to dev (so dev has the version bump)
git checkout dev
git merge release/v3.0.0-kiwicrab
git push origin dev

# 4. Delete the release branch
git branch -d release/v3.0.0-kiwicrab
git push origin --delete release/v3.0.0-kiwicrab
```

**Why this workflow:**
- dev continues accepting PRs during release preparation
- Release work is isolated on the release branch
- No conflicts from ongoing development
- Clear freeze point for the release
- dev stays in sync with version numbers

### GitHub Release

Create a release using [Keep a Changelog](https://keepachangelog.com/) format.

**Release Notes Structure:**
- Use Keep a Changelog categories: Added, Changed, Deprecated, Removed, Fixed, Security
- Include technical details for developers
- Include compatibility notes
- Include installation instructions
- Reference [Semantic Versioning](https://semver.org/) compliance

Example:

```markdown
## [3.0.0-kiwicrab] - 2026-02-02

### Added
- Tag color name display feature (--cclip-show-tag-color-names)
- Tag management CLI flags (--tag clear, --tag list)

### Changed
- Major codebase refactor into modular structure
- Improved Sixel image clearing logic

### Fixed
- Tag color names now display correctly in UI
- Sixel clearing no longer wipes text

### Notes
MINOR version bump per Semantic Versioning 2.0.0 - backward compatible.
```

---

## Versioning Scheme

Semantic Versioning 2.0.0 + optional codename.

Format:

```
major.minor.patch-codename
```

Example:

```
v3.0.0-kiwicrab
```

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

---

## Documentation Standards

### Minimum

1. `README.md` — overview, install, usage
2. `USAGE.md` — detailed guide (if needed)
3. `LICENSE` — BSD-2-Clause or similar

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

- Push directly to main or dev
- Merge without PR
- Release without testing
- Dump raw git logs as changelog
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
git checkout -b release/v3.0.0-kiwicrab

# 2. Update version numbers in Cargo.toml, flake.nix, etc.
# ... edit files ...
git commit -am "chore: bump version to 3.0.0-kiwicrab"

# 3. Final testing
cargo build --release
cargo test

# 4. Merge to main and tag
git checkout main
git pull origin main
git merge release/v3.0.0-kiwicrab
git tag -a v3.0.0-kiwicrab -m "v3.0.0-kiwicrab: major release"
git push origin main --tags

# 5. Merge back to dev
git checkout dev
git merge release/v3.0.0-kiwicrab
git push origin dev

# 6. Clean up
git branch -d release/v3.0.0-kiwicrab
git push origin --delete release/v3.0.0-kiwicrab
```

### Hotfix

```bash
git checkout main
git pull origin main
git checkout -b hotfix/cache
# fix issue
git commit -am "fix: prevent cache corruption"
git push origin hotfix/cache
# PR to main, approve, merge
git tag -a v3.0.1-kiwicrab -m "v3.0.1-kiwicrab: hotfix"
git push origin main --tags
git checkout dev
git merge main
git push origin dev
```

### Documentation-Only Fix

```bash
git checkout main
git pull origin main
# Fix typo in README
git commit -am "docs: fix typo in installation section"
git push origin main
# Sync to dev
git checkout dev
git merge main
git push origin dev
```

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

- All work through branches & PRs
- dev is your proving ground
- main is immutable history
- Reviews are collaborative, not confrontational
- Tags *are* the changelog

Future you: if it's 2am and you're wondering how to do this properly — do it this way. You'll thank yourself later.

---

End of Standard