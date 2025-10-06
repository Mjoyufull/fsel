# Changelog
All notable changes to Gyr will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.0-eggrind] - 2025-01-06

### Added

* **Dmenu Mode**: Full dmenu replacement with `--dmenu` flag
  - Reads from stdin, outputs to stdout
  - Column filtering with `--with-nth` (comma-separated column numbers)
  - Custom delimiters with `--delimiter`
  - Content preview panel
  - Fuzzy matching support

* **Clipboard History Mode**: Browse clipboard history with `--cclip` flag
  - Integrates with cclip clipboard manager
  - Inline image previews for Kitty/Sixel terminals
  - Shows cclip rowid as line numbers
  - Automatically copies selection back to clipboard

* **Enhanced Configuration**:
  - Separate `[dmenu]` and `[cclip]` config sections
  - Mode-specific color schemes and layouts
  - Image preview settings (`hide_inline_image_message`)
  - Display options (`show_line_numbers`, `wrap_long_lines`)

### Changed

* Project tagline: now "Fast TUI app launcher and fuzzy finder"
* Updated Nix flake with new version and maintainer

### Technical

* Modular UI system supporting multiple interaction modes
* Advanced terminal graphics integration with Kitty/Sixel protocols
* Configuration inheritance system (cclip → dmenu → regular mode)
* Zero-copy string handling for large clipboard content
* Non-blocking image preview generation
* This release was ground up like an eggshell - hence "eggrind" 🥚

### Dependencies

* Optional: cclip (clipboard manager), chafa (image previews)

## [0.2.8-bolttree] - 2025-01-05

### Added

* Full mouse support (hover, click, scroll wheel)
* Direct launch mode with `-p/--program` flag
* Pre-filled search mode with `-ss` flag

### Improved

* Mouse cursor positioning and scroll behavior
* Selection persistence during rapid interactions
* Command-line argument validation

### Fixed

* Input field border gaps
* Selection disappearing during fast scrolling

## [0.2.7] - 2025-01-05

### Added

* XDG Desktop Entry Specification support:
  - Localization support for Name, Comment, GenericName fields
  - Keywords field parsing
  - Categories field parsing
  - MimeType field parsing
  - Icon field support
  - OnlyShowIn/NotShowIn desktop environment filtering
  - Hidden field support
  - StartupNotify and StartupWMClass fields
  - TryExec field validation
  - Type field validation
  - Desktop file ID tracking

* Multi-field fuzzy matching against name, generic name, keywords, description, and categories
* Weighted scoring system for better search results
* Extended verbose mode output with additional metadata

### Fixed

* XDG Base Directory Specification compliance:
  - Added missing XDG_DATA_HOME support
  - Fixed ~/.local/share/applications/ scanning
  - Proper directory scanning order
  - Correct XDG specification fallback logic

* Desktop entry parsing:
  - Proper semicolon-separated list handling
  - Better XDG field code removal from Exec commands
  - Comment and empty line handling
  - Section parsing boundary detection

### Changed

* Extended App struct with XDG Desktop Entry fields
* Application discovery now includes user-specific directories

### Dependencies

* Added `which` crate v4.4 for TryExec validation

## [0.2.6] - 2025-09-08

### Added

* Enhanced color support with multiple formats:
  - Hex colors: `#ff0000`, `#f00`
  - RGB colors: `rgb(255,0,0)`, `(255,0,0)`
  - 8-bit terminal colors: `196` (0-255)
  - Additional named colors: `gray`/`grey`, `darkgray`/`darkgrey`, `reset`
* Color examples file with popular themes (Gruvbox, Nord, Dracula, One Dark)
* Improved error messages for invalid color formats

## [0.2.5] - 2025-09-08

### Removed

* Cached entries system - removed due to reliability issues with detecting new applications
* `--clear-cache` CLI option - no longer needed
* Cache configuration options from config.toml (`enable_cache`, `cache_ttl_seconds`)
* Unused safe-regex dependency

### Changed

* Migrated from termion to crossterm for professional terminal handling
* Optimized application scanning for better performance without caching
* Improved terminal cleanup - no more blank lines or artifacts on exit
* Enhanced input handling with crossterm KeyEvent system

## [0.2.1] - 2025-09-07

### Added

* Multiple launch backends: `--systemd-run`, `--uwsm`, and `--no-exec` options
* ~~Cached entries system with 36% performance improvement~~ (later removed)
* Extensive UI customization options (15+ configuration settings)
* Nix flake for universal installation across distributions
* Enhanced configuration system with validation

### Changed

* Forked from original sourcehut repository (git.sr.ht/~nkeor/gyr) to GitHub
* Updated repository URLs and documentation for GitHub hosting
* Improved lock file management with automatic cleanup
* Enhanced error handling and graceful fallbacks
* Unpinned serde and serde_derive

## [v0.1.4] - 2023-08-20

### Changed

* Migrated from [tui](https://github.com/fdehau/tui-rs) to [ratatui](https://github.com/ratatui-org/ratatui)
* Pinned serde and serde_derive to v1.0.171, see https://github.com/serde-rs/serde/issues/2538

## [v0.1.3] - 2023-04-30

### Fixed

* Updated dependencies

## [v0.1.2] - 2022-09-13

### Added

* `-r`, `--replace` option to replace an existing Gyr instance.

### Changed

* Switched from dirty recursive directory walker to [walkdir](https://crates.io/crates/walkdir)

## [v0.1.1] - 2022-07-26

### Added

* VIM keybindings (`Ctrl+N`/`Ctrl+P`/`Ctrl+Y`)
* config: Disabling infinite scrolling via `hard_stop`

### Fixed

* ui: remove unused log
* Wait until loading finishes before showing the UI
* Switched to case insensitive sorting
* Read `$XDG_DATA_DIRS` instead of harcoded data paths

## [v0.1.0] - 2022-07-01

* Initial release
