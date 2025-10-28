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

- **Rust 1.89+ stable** (NOT nightly)
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  rustc --version  # Verify stable, not nightly
  ```
- **Cargo** (comes with Rust)
- **Git**

### Optional Dependencies

For full functionality during development:

- **cclip** - For testing clipboard history mode
- **chafa** - For testing image previews
- **kitty** or Sixel-capable terminal (Foot, WezTerm) - For testing inline images
- **uwsm** - For testing Universal Wayland Session Manager integration
- **systemd** - For testing systemd-run integration (usually pre-installed)

### Clone and Build

```sh
# Clone the repository
git clone https://github.com/Mjoyufull/fsel.git
cd fsel

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

**Simple documentation fixes** (typos, grammar, formatting) can be pushed directly to `main` without going through the PR process:

**Criteria for direct push:**
- Changes only to `.md` files (README, USAGE, CONTRIBUTING, etc.)
- No code changes whatsoever
- Typo fixes, grammar corrections, formatting improvements

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

**For substantial documentation changes** (rewrites, new sections, structural changes), please use the normal PR process for review.

---

## Branching Strategy

**IMPORTANT**: Never push directly to `main` or `dev`. All changes go through pull requests.

### Primary Branches

| Branch | Purpose | Push Policy |
|--------|---------|-------------|
| **main** | Stable, production-ready code. Every commit is a tagged release. | Never push directly. Merge only from `dev` after testing and tagging. |
| **dev** | Integration branch. All features merge here before `main`. | Never push directly. Only receives merges from feature branches via pull requests. |

### Feature Branches

All work occurs in feature branches created from `dev`:

| Type | Naming | Purpose |
|------|--------|---------|
| Feature | `feat/name` | New features or functionality |
| Fix | `fix/name` | Bug fixes |
| Refactor | `refactor/name` | Code restructuring without changing behavior |
| Docs | `docs/name` | Documentation changes |
| Chore | `chore/name` | Tooling, dependencies, build updates |

### Standard Workflow

```sh
# 1. Create feature branch from dev
git checkout dev
git pull origin dev
git checkout -b feat/your-feature-name

# 2. Develop locally (commit freely)
git commit -am "wip: working on feature"

# 3. Prepare for PR (clean up commits)
git fetch origin
git rebase origin/dev
git rebase -i origin/dev  # Interactive rebase to clean history

# 4. Push feature branch
git push origin feat/your-feature-name

# 5. Open pull request targeting dev
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

1. **Rebase on latest dev**:
   ```sh
   git fetch origin
   git rebase origin/dev
   ```

2. **Run all checks**:
   ```sh
   cargo fmt
   cargo clippy -- -D warnings
   cargo test
   cargo build --release
   ```

3. **Clean commit history**:
   ```sh
   git rebase -i origin/dev
   ```

4. **Push branch**:
   ```sh
   git push origin feat/your-feature-name
   ```

### Opening a PR

1. Go to GitHub and open a pull request
2. **Base**: `dev` (NOT `main`)
3. **Compare**: your feature branch
4. Use the PR template below

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

**Note:** Draft PRs still target the `dev` branch and follow all other PR guidelines.

### PR Guidelines

- Target the `dev` branch, not `main`
- Use a clear, descriptive title following conventional commits format
- Keep PRs focused on a single feature or fix
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
- fsel version: [e.g., 2.2.0-seedclay]
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

### Preparation (Maintainers Only)

1. Ensure all feature PRs are merged into `dev`
2. Confirm all tests pass:
   ```sh
   cargo test
   cargo build --release
   ```
3. Maintainers update version references in:
   - `Cargo.toml` (root directory)
   - `flake.nix` (root directory)
   - `README.md` (installation instructions)
   - Man pages (`fsel.1` or similar)
4. Prepare release notes following [Keep a Changelog](https://keepachangelog.com/)
5. Verify [Semantic Versioning 2.0.0](https://semver.org/) compliance

### Codename Policy

**Codenames change only on MAJOR version bumps:**
- Codename for 2.x.x series: `seedclay`
- Previous codename for 1.x.x series: `riceknife`
- When 3.0.0 is released, a new codename will be chosen
- **Only maintainers** choose and assign codenames

This is a new policy starting from version 2.0.0. All 2.x.x releases use `seedclay`.

### Process

```sh
git checkout main
git pull origin main
git merge dev
git tag -a v2.2.0-seedclay -m "v2.2.0-seedclay: tag system and codebase refactor"
git push origin main --tags
```

### GitHub Release

Create a release using [Keep a Changelog](https://keepachangelog.com/) format:

```markdown
## [2.2.0-seedclay] - 2025-10-27

### Added
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

- Push directly to `main` or `dev`
- Merge without PR
- Release without testing
- Ignore version updates in relevant files
- Skip running `cargo fmt` and `cargo clippy` before pushing

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
A: Yes, but it's a very quick process. Create a `docs/fix-typo` branch, make the change, push, and open a PR. Documentation improvements are always welcome.

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

- [PROJECT_STANDARDS.md](./PROJECT_STANDARDS.md) - Git workflow and standards
- [USAGE.md](./USAGE.md) - User documentation
- [README.md](./README.md) - Project overview

---

**Questions?** If anything in this guide is unclear or you have suggestions for improving it, please open an [issue](https://github.com/Mjoyufull/fsel/issues) or [discussion](https://github.com/Mjoyufull/fsel/discussions).

Thank you again for contributing to fsel!