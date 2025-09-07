# Changelog
All notable changes to Gyr will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

* Forked from original sourcehut repository (git.sr.ht/~nkeor/gyr) to GitHub
* Updated repository URLs and documentation for GitHub hosting
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
