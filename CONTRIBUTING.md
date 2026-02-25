# Contributing to fsel

Thank you for your interest in contributing to fsel. This project welcomes contributions from the community, whether you're fixing bugs, improving documentation, or proposing new features.

---

## Table of Contents

- [Getting Started](#getting-started)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [How to Contribute](#how-to-contribute)
- [Branching Strategy](#branching-strategy)
- [Commit Standards](#commit-standards)
- [Pull Request Process](#pull-request-process)
- [Code Review](#code-review)
- [Testing](#testing)
- [Coding Standards](#coding-standards)
- [Reporting Bugs](#reporting-bugs)
- [Suggesting Features](#suggesting-features)
- [Release Process](#release-process)
- [What Not To Do](#what-not-to-do)
- [Getting Help](#getting-help)

---

## Getting Started

Before contributing, please:

1. Read the [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for our Git workflow and conventions
2. Check existing [issues](https://github.com/Mjoyufull/fsel/issues) and [pull requests](https://github.com/Mjoyufull/fsel/pulls) to avoid duplicating work
3. Understand that **all code changes go through pull requests** — no exceptions
4. **Fork the repository** if you don't have write access (most contributors)

### Key Resources

- **Issue Tracker**: [GitHub Issues](https://github.com/Mjoyufull/fsel/issues)
- **Discussions**: [GitHub Discussions](https://github.com/Mjoyufull/fsel/discussions)
- **Project Standards**: [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md)
- **Usage Documentation**: [USAGE.md](./USAGE.md)
- **Project README**: [README.md](./README.md)

---

## Development Setup

### Prerequisites

fsel is written in Rust. You will need:

- **Rust 1.90+ stable** (NOT nightly)
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustc --version  # Verify stable, not nightly
  ```
- **Cargo** (comes with Rust)
- **Git**

### Optional Dependencies

For full functionality during development:

- **cclip** - For testing clipboard history mode
- **Kitty, Foot, WezTerm, or other Sixel/Kitty/Halfblocks-capable terminal** - For testing inline image previews (3.1.0+ uses built-in [ratatui-image](https://github.com/benjajaja/ratatui-image); pre-3.1.0 used chafa)
- **uwsm** - For testing Universal Wayland Session Manager integration
- **systemd** - For testing systemd-run integration (usually pre-installed)

### Fork and Clone

**For external contributors (most people):**

1. **Fork the repository** on GitHub:
   - Go to https://github.com/Mjoyufull/fsel
   - Click the "Fork" button in the top-right corner
   - This creates your own copy of the repository

2. **Clone your fork**:
   ```sh
   # Replace YOUR_USERNAME with your GitHub username
   git clone https://github.com/YOUR_USERNAME/fsel.git
   cd fsel
   
   # Add the upstream repository as a remote
   # This lets you sync with the main repository before creating PRs
   git remote add upstream https://github.com/Mjoyufull/fsel.git
   ```
   
   **Why add upstream?** The upstream remote lets you:
   - Fetch the latest changes from the main repository
   - Rebase your feature branch on top of the latest `dev` branch
   - Keep your fork synchronized with the main project

3. **Keep your fork up to date** (do this periodically, especially before starting new work):
   ```sh
   # Fetch latest changes from the main repository
   git fetch upstream
   
   # For code work: update your fork's dev branch
   git checkout dev
   git merge upstream/dev
   git push origin dev
   
   # For docs work: update your fork's main branch
   git checkout main
   git merge upstream/main
   git push origin main
   ```
   
   **When to sync**: Before starting a new feature branch (use dev for code, main for docs), or if you notice the main repository has new commits you want to include.

**For maintainers with write access:**

```sh
# Clone the repository directly
git clone https://github.com/Mjoyufull/fsel.git
cd fsel
```

### Build

```sh
# Build in debug mode (faster compilation)
cargo build

# Build in release mode (optimized)
cargo build --release

# Run directly
cargo run

# Run with arguments
cargo run -- --help
```

### Development Build

For faster iteration during development:

```sh
# Build and run in debug mode
cargo run

# Watch for changes and rebuild automatically (requires cargo-watch)
cargo install cargo-watch
cargo watch -x run
```

---

## Project Structure

```
src/
├── cli.rs              # Command-line argument parsing
├── common/             # Shared types and utilities
│   ├── item.rs         # Common item structures
│   └── mod.rs
├── core/               # Core functionality
│   ├── cache.rs        # Desktop file caching system
│   ├── database.rs     # History and pinned apps database
│   └── mod.rs
├── desktop/            # Desktop entry parsing
│   ├── app.rs          # Desktop application representation
│   └── mod.rs
├── main.rs             # Entry point
├── modes/              # Application modes
│   ├── app_launcher/   # Main application launcher mode
│   │   ├── launch.rs   # Application launching logic
│   │   ├── mod.rs
│   │   ├── run.rs      # Mode execution
│   │   └── search.rs   # Application search and filtering
│   ├── cclip/          # Clipboard history mode
│   │   ├── mod.rs
│   │   ├── preview.rs  # Image preview handling
│   │   ├── run.rs      # Mode execution
│   │   ├── scan.rs     # Clipboard scanning
│   │   └── select.rs   # Selection handling
│   ├── dmenu/          # Dmenu mode
│   │   ├── mod.rs
│   │   ├── parse.rs    # Input parsing and column handling
│   │   └── run.rs      # Mode execution
│   └── mod.rs
├── process.rs          # Process management and discovery
├── strings.rs          # String utilities and parsing
└── ui/                 # User interface components
    ├── app_ui.rs       # Application launcher UI
    ├── dmenu_ui.rs     # Dmenu mode UI
    ├── graphics.rs     # Graphics and image display (Kitty/Sixel)
    ├── input.rs        # Keyboard and mouse input handling
    ├── keybinds.rs     # Configurable keybindings
    └── mod.rs
```

### Module Responsibilities

- **cli.rs**: Defines all command-line flags and arguments
- **common/**: Shared types used across multiple modules
- **core/**: Database operations, caching, and persistent data
- **desktop/**: XDG Desktop Entry parsing and application discovery
- **modes/**: Each mode is self-contained with its own logic
- **process.rs**: Process spawning, detachment, and PID management
- **strings.rs**: String manipulation and parsing utilities
- **ui/**: All terminal UI rendering and interaction

---

## How to Contribute

There are many ways to contribute to fsel:

### Code Contributions

- Fix bugs listed in [issues](https://github.com/Mjoyufull/fsel/issues)
- Implement new features
- Improve performance
- Refactor code for clarity or maintainability

### Non-Code Contributions

- Improve documentation (see [Documentation Changes](#documentation-changes) below)
- Create example configurations
- Answer questions in [discussions](https://github.com/Mjoyufull/fsel/discussions)
- Test new releases and report issues
- Package fsel for other distributions
- Write tutorials or blog posts

### Documentation Changes

**Docs go to main.** Documentation-only changes (typos, grammar, formatting, correctness updates like fixing example syntax) use a branch from **main** and a PR **targeting main** — not dev. After your PR is merged to main, a maintainer merges main into dev so dev stays in sync.

**Criteria for docs-only:**
- Changes only to `.md` files (README, USAGE, CONTRIBUTING, etc.) or other doc assets
- No source code or config file changes that affect behavior
- Typo fixes, grammar, formatting, clarifications, fixing outdated examples (e.g. updated Hyprland syntax)

**Process (contributor):**
```bash
# Branch from main (sync main first)
git fetch upstream
git checkout main
git merge upstream/main
git checkout -b docs/fix-typo-readme   # or docs/fix-hyprland-syntax, etc.
# Make documentation changes
git add -A && git commit -m "docs: fix typo in README"
git push origin docs/fix-typo-readme
# Open PR targeting main (not dev)
```

Maintainers may push trivial docs fixes directly to main and then merge main into dev; for anything that deserves review, use a PR to main. See [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for full details.

---

## Branching Strategy

**IMPORTANT**: Never push code directly to `main` or `dev`. Code goes through PRs to dev; docs go through PRs to main (or maintainer push to main for trivial docs).

### Primary Branches

| Branch | Purpose | Push Policy |
|--------|---------|-------------|
| **main** | Releases and living docs. Every code commit on main is a tagged release; docs are updated on main. | Code reaches main only via release or hotfix branches. Docs reach main via PRs targeting main (or maintainer push for trivial docs). |
| **dev** | Integration branch. All code work merges here; release branches are created from dev. | Receives merges from feature branches (via PRs) and from main after a hotfix or after docs land on main (to sync). |

### Feature Branches

**Code work** uses feature branches created from `dev`. **Documentation-only changes** use a branch from **main** and a PR targeting **main** (see [Documentation Changes](#documentation-changes)).

| Type | Naming | Purpose |
|------|--------|---------|
| Feature | `feat/name` | New features or functionality (branch from dev, PR to dev) |
| Fix | `fix/name` | Bug fixes (branch from dev, PR to dev) |
| Refactor | `refactor/name` | Code restructuring (branch from dev, PR to dev) |
| Docs | `docs/name` | Documentation (branch from **main**, PR to **main**) |
| Chore | `chore/name` | Tooling, dependencies, build updates (branch from dev, PR to dev) |

### Release Branches

| Type | Naming | Purpose |
|------|--------|---------|
| Release | `release/version` | Prepare releases with version bumps and final testing |

Release branches are created from `dev` when a maintainer decides to release. They freeze a stable point in `dev` for release preparation, allowing ongoing PRs to continue merging into `dev` without affecting the release. See the [Release Process](#release-process) section for details.

**Note:** Code reaches `main` only via release or hotfix branches. Docs are updated on `main` via PRs (or maintainer push). See [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for the full workflow.

### Standard Workflow

**For external contributors — code (using forks):**

```sh
# 1. Update your fork's dev branch
git fetch upstream
git checkout dev
git merge upstream/dev
git push origin dev

# 2. Create feature branch from dev
git checkout dev
git checkout -b feat/your-feature-name

# 3. Develop locally (commit freely)
git commit -am "wip: working on feature"

# 4. Prepare for PR (sync with latest dev and clean up commits)
git fetch upstream
git rebase upstream/dev
git rebase -i upstream/dev  # Optional: clean up commit history

# 5. Push feature branch to your fork
git push origin feat/your-feature-name

# 6. Open pull request targeting Mjoyufull/fsel:dev
# IMPORTANT: Enable "Allow edits by maintainers" checkbox
```

**For external contributors — documentation:**

```sh
# 1. Update your fork's main branch
git fetch upstream
git checkout main
git merge upstream/main
git push origin main

# 2. Create docs branch from main
git checkout -b docs/your-doc-fix

# 3. Edit docs, commit, push
git add -A && git commit -m "docs: describe your change"
git push origin docs/your-doc-fix

# 4. Open pull request targeting Mjoyufull/fsel:main (not dev)
```

**For maintainers (direct access):**

```sh
# Code: branch from dev, PR to dev (same as above but use origin instead of upstream)
# Docs: branch from main, PR to main; or push trivial docs directly to main then merge main into dev
```

---

## Commit Standards

Follow **Conventional Commits** format:

```
type(optional-scope): short description

[optional body]

[optional footer]
```

### Commit Types

| Type | Meaning |
|------|---------|
| `feat` | New feature |
| `fix` | Bug fix |
| `docs` | Documentation only |
| `refactor` | Code restructuring |
| `perf` | Performance improvement |
| `chore` | Build, deps, tooling |
| `test` | Testing only |
| `style` | Whitespace, formatting |
| `revert` | Undo a commit |

### Examples

```sh
feat(detach): implement --detach flag with systemd-run support
fix(db): enforce foreign key constraints properly
refactor(cache): move batch operations to separate module
docs(usage): add examples for dmenu mode
chore: update flake.nix to use naersk
```

### During Development

- Commit as you work — don't obsess over perfection
- "wip" and "temp fix" are valid local commits
- Clean up commit history before opening PR using `git rebase -i`

---

## Pull Request Process

### Before Submitting

**For code PRs (targeting dev):**

1. **Rebase on latest dev**:
   ```sh
   # For external contributors using forks:
   git fetch upstream
   git rebase upstream/dev
   
   # For maintainers with direct access:
   git fetch origin
   git rebase origin/dev
   ```
   
   **Note**: Rebase doesn't delete your changes! It replays your commits on top of the latest branch and helps avoid merge conflicts.

2. **Run all checks**:
   ```sh
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

3. **Clean commit history** (optional): `git rebase -i origin/dev`

4. **Push branch**: `git push origin feat/your-feature-name`

**For documentation PRs (targeting main):** Rebase on latest main (`git fetch upstream && git rebase upstream/main` or `origin/main`), then push your docs branch. No need to run cargo checks for docs-only changes.

### Opening a PR

1. **Code changes**: Open a PR **targeting dev**
   - **Base repository**: `Mjoyufull/fsel`, **Base**: `dev`, **Compare**: your feature branch (e.g. `feat/your-feature-name`)
   
   **Documentation changes**: Open a PR **targeting main**
   - **Base repository**: `Mjoyufull/fsel`, **Base**: `main`, **Compare**: your docs branch (e.g. `docs/fix-typo`)

2. **IMPORTANT**: Enable the **"Allow edits by maintainers"** checkbox
   - This allows maintainers to make small fixes, rebase, or help resolve conflicts
   - This follows the collaborative philosophy in [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md)
   - Maintainers will respect your work and credit you appropriately

3. Use the PR template below

### PR Template

**Title**: `feat: add your feature` (follow conventional commits)

**Body**:
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

GitHub allows you to open PRs as "drafts" - these are PRs that aren't ready for review yet but you want to show your progress.

**When to use draft PRs:**
- You want early feedback on approach before completing the work
- You're working on a large feature and want visibility into your progress
- You want architectural review before full implementation
- You're stuck and need help to continue

**How to create a draft PR:**
1. When opening a PR on GitHub, click the dropdown on "Create pull request"
2. Select "Create draft pull request" instead
3. The PR will be marked as draft and reviewers won't be notified
4. When ready, click "Ready for review" to convert it to a normal PR

**Note:** Draft PRs for code still target `dev`; draft PRs for docs target `main`. Follow the same branch-target rules as above.

### PR Guidelines

- **Code PRs**: Target the `dev` branch. **Docs PRs**: Target the `main` branch.
- Use a clear, descriptive title following conventional commits format
- Keep PRs focused on a single feature, fix, or doc change
- Break large changes into smaller, reviewable PRs
- Respond to review feedback promptly
- Be open to suggestions and constructive criticism

---

## Code Review

### What to Expect

- **Initial response**: A few hours to a few days
- **Full review**: Within 1 week
- **Merge after approval**: Within 1-2 days
- Reviewers may request changes or ask questions
- Multiple rounds of review may be necessary

### Internal Merging

Sometimes PRs are accepted but merged internally as part of larger refactoring efforts:

- You will be credited in commit messages and release notes
- The functionality you contributed will be included
- This is not a rejection, but integration into ongoing development

Example maintainer response:
> "Thank you for this contribution. Your approach is better than the current implementation. I'm currently refactoring the project structure, so I'll be merging this internally as part of that effort. You'll be credited in the commit message and release notes when it ships."

### Stale PRs

- PRs without activity for **30 days** will be marked stale
- Stale PRs will be closed after **14 additional days** of inactivity
- Exception: PRs marked as work-in-progress or on-hold by maintainers
- Closed PRs can be reopened if work resumes

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

- Comment with **why**, not just "change this"
- Nitpicks = non-blocking
- If it's broken, mark **Request Changes**
- Prefer questions over commands:
  > "Could this be simplified?" not "Simplify this."

---

## Testing

### Running Tests

```sh
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests with output
cargo test -- --nocapture

# Run tests with backtrace
RUST_BACKTRACE=1 cargo test
```

### Writing Tests

Add unit tests in the same file as the code:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_desktop_entry() {
        let entry = parse_entry("test.desktop");
        assert!(entry.is_ok());
    }
}
```

Add integration tests in `tests/` directory for end-to-end testing.

### Manual Testing

Before submitting a pull request, manually test:

1. Basic application launching
2. Dmenu mode with piped input
3. Clipboard mode if cclip is installed
4. Mouse and keyboard navigation
5. Various command-line flags
6. Configuration file loading

---

## Coding Standards

### Rust Style

- Follow the official [Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/)
- Use `rustfmt` for formatting:
  ```sh
  cargo fmt
  ```
- Use `clippy` for linting:
  ```sh
  cargo clippy -- -D warnings
  ```
- All code must compile without warnings on Rust stable

### Code Quality

- Write clear, self-documenting code
- Add doc comments for public APIs:
  ```rust
  /// Launches an application with the specified detachment mode.
  ///
  /// # Arguments
  ///
  /// * `app` - The desktop application to launch
  /// * `detach` - Whether to detach the process
  ///
  /// # Returns
  ///
  /// Returns `Ok(())` on success, or an error if launching fails
  pub fn launch_app(app: &App, detach: bool) -> Result<()> {
      // implementation
  }
  ```
- Keep functions focused and single-purpose
- Avoid deeply nested code
- Use meaningful variable and function names

### Error Handling

- Use `Result` and `?` operator for error propagation
- Provide context with error messages:
  ```rust
  .with_context(|| format!("Failed to read config file: {}", path.display()))?
  ```
- Handle errors gracefully and provide user-friendly messages

### Performance Considerations

- Avoid unnecessary allocations
- Use references where possible
- Profile before optimizing
- Document performance-critical sections

---

## Reporting Bugs

If you find a bug, please [open an issue](https://github.com/Mjoyufull/fsel/issues/new) with the following information:

### Bug Report Template

```markdown
**Description**
A clear and concise description of the bug.

**To Reproduce**
Steps to reproduce the behavior:
1. Run command '...'
2. Type '...'
3. Press '...'
4. See error

**Expected Behavior**
What you expected to happen.

**Actual Behavior**
What actually happened.

**Environment**
- fsel version: [e.g., 3.1.0-kiwicrab]
- OS: [e.g., Arch Linux, kernel 6.6.1]
- Terminal: [e.g., kitty 0.30.0]
- Rust version: [output of `rustc --version`]
- Desktop Environment: [e.g., Sway, Hyprland, GNOME]

**Configuration**
If relevant, include your config file or specific settings:
```toml
# Your config.toml contents
```

**Logs/Output**
If applicable, include error messages or logs:
```
Error output here
```

**Additional Context**
Any other information that might be relevant.
```

### Good First Issues

Look for issues labeled:
- `good first issue` - Simple bugs suitable for newcomers
- `bug` - Confirmed bugs that need fixing
- `help wanted` - Issues where maintainers need assistance

---

## Suggesting Features

Feature suggestions are welcome. Before suggesting a feature:

1. Check if it has already been suggested in [issues](https://github.com/Mjoyufull/fsel/issues)
2. Consider if it fits fsel's scope as a TUI application launcher
3. Think about how it would be implemented

### Feature Request Template

```markdown
**Feature Description**
A clear description of the feature you'd like to see.

**Use Case**
Explain the problem this feature would solve or the workflow it would improve.

**Proposed Solution**
If you have ideas on how to implement this, describe them here.

**Alternatives Considered**
Other ways you've considered solving this problem.

**Additional Context**
Any other context, mockups, or examples.
```

---

## Release Process

**Note**: Only maintainers handle releases and all version updates. Contributors do not need to update version numbers.

### When to Create a Release Branch

A maintainer creates a release branch when:
- They decide it's time for a release
- `dev` is in a stable state (no critical bugs, features are complete)
- All planned features for the release are merged into `dev`

**Important:** Release branches freeze a specific point in `dev`, allowing ongoing PRs to continue merging into `dev` without affecting the release preparation. Code reaches `main` only via release or hotfix branches; see [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) for the full workflow.

### Preparation (Maintainers Only)

1. **Merge main into dev** so dev has the latest docs (docs live on main and are synced to dev via main → dev).
2. Ensure all feature PRs for the release are merged into `dev`.
3. Confirm all tests pass on `dev`:
   ```sh
   cargo test
   cargo build --release
   ```
4. Create a release branch from `dev` (this freezes the release point):
   ```sh
   git checkout dev
   git pull origin dev
   git checkout -b release/3.1.0-kiwicrab  # Replace with actual version
   ```
5. Update version references on the release branch:
   - `Cargo.toml` (root directory)
   - `flake.nix` (root directory)
   - `README.md` (installation instructions, if needed)
   - Man pages (`fsel.1` or similar)
6. Commit version bump:
   ```sh
   git commit -am "chore: bump version to 3.1.0-kiwicrab"
   ```
7. Prepare release notes using the template in [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md); update **RELEASELOG.md** on the release branch with the release title and body, adding a `---` separator above the previous release(s).
8. Verify [Semantic Versioning 2.0.0](https://semver.org/) compliance.
9. Run final tests on the release branch:
   ```sh
   cargo test
   cargo build --release
   ```

### Codename Policy

**Codenames change only on MAJOR version bumps:**
- Codename for 3.x.x series: `kiwicrab`
- Previous codename for 2.x.x series: `seedclay`
- Previous codename for 1.x.x series: `riceknife`
- **Only maintainers** choose and assign codenames

This policy started from version 2.0.0. All 3.x.x releases use `kiwicrab`.

### Process

```sh
# 1. Merge release branch to main
git checkout main
git pull origin main
git merge release/3.1.0-kiwicrab

# 2. Tag the release (version number only, no "v" prefix per PROJECT_STANDARDS)
git tag -a 3.1.0 -m "3.1.0"
git push origin main --tags

# 3. Merge release branch back to dev (so dev has the version bump)
git checkout dev
git merge release/3.1.0-kiwicrab
git push origin dev

# 4. Delete the release branch
git branch -d release/3.1.0-kiwicrab
git push origin --delete release/3.1.0-kiwicrab
```

**Why this workflow:**
- `dev` continues accepting PRs during release preparation
- Release work is isolated on the release branch
- No conflicts from ongoing development
- Clear freeze point for the release
- `dev` stays in sync with version numbers

### GitHub Release

Create a release using the release body template in [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) (same structure as the block added to RELEASELOG.md). Example format:

```markdown
## [3.1.0-kiwicrab] - YYYY-MM-DD

### Added
- Native inline and fullscreen image previews in cclip mode via [ratatui-image](https://github.com/benjajaja/ratatui-image) (Kitty, Sixel, Halfblocks; no chafa required)
- Tag system for clipboard items with color/emoji metadata
- Sixel image support for Foot terminal

### Changed
- Major codebase refactor into modular structure
- Improved image clearing logic for Foot terminal

### Fixed
- Text disappearing after image display in Foot
- Tag metadata not persisting across sessions

### Notes
MINOR version bump per Semantic Versioning 2.0.0 - backward compatible.
```

---

## What Not To Do

### Absolutely Forbidden

- Push code directly to `main` or `dev` (docs go to main via PR, or maintainer push for trivial docs)
- Merge code without a PR
- Release without testing
- Ignore version updates in relevant files
- Skip running `cargo fmt` and `cargo clippy` before pushing code PRs

### Strongly Discouraged

- Inconsistent versioning
- Unreviewed breaking changes
- Merging with failing tests
- Ignoring clippy warnings
- Leaving PRs without response for weeks

---

## Getting Help

### Communication Channels

- **GitHub Issues**: For bugs and feature requests
- **GitHub Discussions**: For questions and general discussion
- **Pull Request Comments**: For questions about specific changes

### Questions About Contributing

If you're unsure about:
- How to implement a feature
- Whether a change would be accepted
- How to test something
- How to structure your code
- Anything else related to contributing

Please open a [discussion](https://github.com/Mjoyufull/fsel/discussions) or comment on a related issue. The maintainers are happy to help guide you.

### Common Questions

**Q: I found a typo in the documentation. Do I still need to open a PR?**  
A: Yes. Branch from **main** (e.g. `docs/fix-typo`), make the change, push, and open a PR **targeting main** (not dev). Documentation improvements are always welcome.

**Q: My PR hasn't been reviewed yet. Should I ping someone?**  
A: Wait 2-4 days for initial response. If no response after 5 days, feel free to leave a polite comment on the PR.

**Q: Can I work on multiple features at once?**  
A: Yes, but create separate branches and PRs for each feature. This makes review easier and allows features to be merged independently.

**Q: I want to refactor a large part of the codebase. Should I do it?**  
A: Open an issue first to discuss the refactoring plan. Large refactors need coordination to avoid conflicts with ongoing work.

**Q: The maintainer wants to merge my PR internally. Did I do something wrong?**  
A: No! This means your contribution is good, but it needs integration with ongoing refactoring work or feature changes. You'll be credited in the release notes.

---

## Recognition

### Contributors

All contributors are recognized in:
- Release notes when their changes are included
- Git commit history with proper attribution
- GitHub contributor statistics
- Special thanks in major release announcements

### Types of Recognition

- **Code Contributors**: Listed in release notes for features and fixes
- **Documentation Contributors**: Credited in commit messages and release notes
- **Bug Reporters**: Thanked in issue closure and release notes
- **Feature Requesters**: Credited when features are implemented
- **Reviewers**: Acknowledged for helpful feedback

### Thank You

Every contribution, no matter how small, helps make fsel better. Whether you're:
- Fixing a typo in documentation
- Reporting a bug
- Implementing a major feature
- Answering questions in discussions
- Testing release candidates

Your time and effort are genuinely appreciated. Thank you for contributing to fsel.

---

## License

By contributing to fsel, you agree that your contributions will be licensed under the BSD-2-Clause License, the same license as the project.

See the [LICENSE](./LICENSE) file for full details.

---

## Additional Resources

### Learning Resources

- [Rust Book](https://doc.rust-lang.org/book/) - Official Rust programming guide
- [Rust by Example](https://doc.rust-lang.org/rust-by-example/) - Learn Rust through examples
- [Clippy Lints](https://rust-lang.github.io/rust-clippy/master/) - Understanding clippy warnings
- [Conventional Commits](https://www.conventionalcommits.org/) - Commit message format
- [Keep a Changelog](https://keepachangelog.com/) - Changelog format

### Project-Specific Documentation

- [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) - Git workflow, release process, and RELEASELOG.md
- [USAGE.md](./USAGE.md) - User documentation
- [README.md](./README.md) - Project overview

---

**Questions?** If anything in this guide is unclear or you have suggestions for improving it, please open an [issue](https://github.com/Mjoyufull/fsel/issues) or [discussion](https://github.com/Mjoyufull/fsel/discussions).

Thank you again for contributing to fsel!