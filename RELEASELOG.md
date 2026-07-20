[3.6.0-kiwicrab]

Added

- Persistent app-launcher entry hiding (from PR #90, closes #81)
  - `Alt+Delete` hides the exact selected desktop entry or executable without deleting or modifying its source file.
  - `Alt+U` restores the most recently hidden entry while the launcher is open.
  - New `--list-hidden`, `--unhide <ID>`, and `--unhide-all` commands manage persistent hides outside the TUI.
  - Hidden entries stay excluded from the interactive launcher, direct launch, and `--stdout`, including after cache or history cleanup.
- Optional deterministic duplicate suppression (from PR #90)
  - New `[app_launcher].auto_hide_duplicates`, `--auto-hide-duplicates[=no]`, and `FSEL_APP_LAUNCHER_AUTO_HIDE_DUPLICATES` controls.
  - Automatic hiding defaults to `false`; manual hiding remains available regardless of that setting.
  - Duplicate selection follows XDG application-directory precedence, supports source-specific entries such as Bedrock Linux strata, and exposes the next eligible source when the current winner is manually hidden.

Changed

- App-launcher visibility and diagnostics (from PR #90)
  - Source-specific entry identities now distinguish equal names and desktop IDs from different paths, including non-UTF-8 paths.
  - `--vvv` reports manual, automatic, and unavailable hidden-entry counts.
  - Hidden-entry listing includes stable numeric IDs, timestamps, source paths, and unavailable-source markers.

Fixed

- App-launcher startup cursor race (from PR #84, addresses #83)
  - Async keyboard input now starts after terminal setup and clearing, preventing cursor-position responses from being consumed by the wrong reader.
  - App mode no longer intermittently fails with `The cursor position could not be read within a normal duration`.
- Configured keybinds across interactive modes (from PR #85, closes #82)
  - Cclip and dmenu navigation, selection, exit, backspace, and mode-specific actions now use configured bindings instead of hard-coded keys.
  - Standalone `~/.config/fsel/keybinds.toml` is loaded when `config.toml` has no embedded `[keybinds]` table; embedded bindings keep precedence.
  - The documented `tab` key and shifted letter events now parse and dispatch correctly.

Technical details

- Added typed `hidden_entries` persistence in redb with source-specific, lossless entry keys and automatically created tables; no manual migration is required.
- Centralized path-key encoding for cache and hidden-entry identities, including Unix non-UTF-8 paths.
- Duplicate suppression groups by desktop-file ID and normalized visible name, then applies deterministic XDG root and relative-path ordering.
- Added regression coverage for persistence, visibility filtering, direct and stdout launch paths, CLI validation, custom modifier bindings, Bedrock-style sources, subdirectory desktop IDs, and non-UTF-8 paths.

Documentation

- README and USAGE: documented manual hiding, restore commands, duplicate suppression, defaults, precedence, and troubleshooting.
- `config.toml` and `keybinds.toml`: documented the new launcher option and hide/restore bindings.
- Man page and CLI help: documented hidden-entry administration and duplicate-suppression flags.
- Version references and release metadata updated for 3.6.0-kiwicrab on the release branch.

Notes

- SemVer: MINOR (3.5.2 -> 3.6.0). This release adds persistent manual entry hiding and optional automatic duplicate suppression while retaining backward-compatible defaults.
- Rationale: 3.6.0 gives users explicit, reversible control over noisy launcher entries, fixes configured keybind behavior across every interactive mode, and removes an intermittent app-launcher startup failure.

Contributors

- @Mjoyufull
- Code review: @cubic-dev-ai, @chatgpt-codex-connector

Compatibility

- Language/runtime: Rust 1.94+ stable; edition remains 2024.
- Platforms: GNU/Linux and *BSD for fsel overall.
- Config: compatible; `auto_hide_duplicates` is optional and defaults to `false`, and new keybind actions have defaults.
- Database: compatible; hidden-entry tables are created automatically and require no destructive migration.
- Breaking: none.

---

[3.5.2-kiwicrab]

Fixed

- Database: auto-create pinned table when open fails (from pr #80)
  - The logic to create the pinned table after a failed open is now robustly handled.
  - Extracted pinned table creation into a dedicated `ensure_pinned_table()` helper to reduce code duplication and improve maintainability.
- Documentation: Fix typo in directory name in README (from Commit 6b6ae52 )

Technical details

- Added `ensure_pinned_table()` helper function for database recovery path behavior when the pinned table is created mid-operation.

Documentation

- README: Fixed typo in directory name.
- Version references updated for 3.5.2-kiwicrab.

Notes

- SemVer: PATCH (3.5.1 -> 3.5.2). This release fixes an issue with the database pinned table opening and includes a minor README typo fix.
- Rationale: 3.5.2 improves database robustness by automatically creating the pinned table if it is missing during runtime operations.

Contributors

- @coko7 (og, the guy created Rust so glad to have his hands on the project)
- @Marbowls (co-maintainer)

Compatibility

- Language/runtime: Rust 1.94+ stable; edition remains 2024.
- Platforms: GNU/Linux and *BSD for fsel overall.
- Config / database: compatible; no schema migration required.
- Breaking: none.

## Download fsel 3.5.2-kiwicrab

|  File  | Platform | Checksum |
|--------|----------|----------|
| [fsel-aarch64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.2/fsel-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.2/fsel-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [fsel-x86_64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.2/fsel-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.2/fsel-x86_64-unknown-linux-gnu.tar.xz.sha256) |

---

[3.5.1-kiwicrab]

Changed

- cclip clipboard copy path (from pr #76)
  - Removed the strange X11 fallback path from cclip mode.
  - cclip mode now requires a Wayland session for copying, matching upstream cclip's Wayland-only clipboard model.
  - The Wayland path still prefers `cclip get <rowid> | wl-copy --type <mime>` when `wl-copy` is available, and falls back to `cclip copy <rowid>`.

Fixed

- cclip copy hang (from pr #76, closes #75)
  - Copying a clipboard history item no longer freezes the TUI by waiting forever on clipboard provider processes that intentionally stay alive while owning the selection.
  - Clipboard providers now get a short startup window so fast failures are still detected without blocking normal successful copies.
  - Empty `cclip get` output is treated as a copy failure instead of silently succeeding.

- Disabled cclip image preview test failure (from pr #76, closes #73)
  - Explicitly disabled cclip image preview no longer triggers terminal graphics detection.
  - Fixes failing `check()` behavior seen in AUR/package builds when preview support is turned off.

Technical details

- Added bounded clipboard-provider startup handling for `wl-copy` and `cclip copy`.
- Removed the old `xclip` / `xsel` branch from cclip selection copy code.
- Added tests for provider startup behavior and fast-failure handling.

Documentation

- Version references and release metadata updated for 3.5.1-kiwicrab.

Notes

- SemVer: PATCH (3.5.0 -> 3.5.1). This release fixes regressions in cclip copy and disabled-preview behavior, and removes an unsupported X11 clipboard path from cclip mode.
- Rationale: 3.5.1 tightens cclip mode around the behavior upstream cclip actually supports: Wayland clipboard history with reliable copy handling and no startup/test regressions.

Contributors

- @Mjoyufull
- Code review: @cubic-dev-ai, @devin-ai-integration

Compatibility

- Language/runtime: Rust 1.94+ stable; edition remains 2024.
- Platforms: GNU/Linux and *BSD for fsel overall.
- cclip mode: Wayland required for copying clipboard history items.
- Config / database: compatible; no schema migration required.
- Breaking: no supported config, database, or documented CLI surface is broken. Users relying on the strange X11 copy fallback in cclip mode should use a Wayland session for cclip mode.



## Download fsel 3.5.1-kiwicrab

|  File  | Platform | Checksum |
|--------|----------|----------|
| [fsel-aarch64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.1/fsel-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.1/fsel-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [fsel-x86_64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.1/fsel-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.1/fsel-x86_64-unknown-linux-gnu.tar.xz.sha256) |

---

[3.5.0-kiwicrab]

Added

- App launcher JSON stdout mode (from pr #69)
  - New `--stdout` flag prints launcher entries as JSON to stdout for scripting/piping.
  - Supports filtering with existing search args (e.g. `-ss`), so output can be pre-filtered.
  - App-launcher flow now supports non-TUI output mode cleanly.
- CLI/docs coverage for stdout mode (from pr #69)
  - Help text and usage docs now include `--stdout` behavior and examples.

Changed

- Cclip startup and image-preview lifecycle (from pr #72)
  - Removed duplicate initial filter pass when no startup search query is provided.
  - Removed successful-startup `cclip list rowid` preflight; database checks are now diagnostic on failure path.
  - Cclip now draws first, then lazily performs richer image protocol detection only when needed for selected image items.
  - Async input lifecycle around stdio image detection is now explicit to avoid terminal read races.
- Wayland clipboard copy path (from pr #72)
  - Wayland copy prefers `cclip get <rowid> | wl-copy --type <mime>` when `wl-copy` is available.
  - Falls back to `cclip copy <rowid>` when needed.
- Release pipeline notes-link normalization
  - Release workflow now rewrites generated asset links in release notes to use the real GitHub release tag path (e.g. `/releases/download/3.5.0/...`) instead of synthetic dist tags.

Fixed

- Direct launch locking/session correctness (from pr #71)
  - Direct app-launch path now respects `--replace` by reusing launcher session/db ownership instead of opening a competing database path.
  - Prevents lock-acquisition conflicts during quick successive launches.
- Cclip image preview stability (from pr #72)
  - Prevented blocking startup behavior caused by terminal image capability probes.
  - Fixed stale/stuck preview state when picker/image-manager state is replaced at runtime by resetting display state on manager swap.
- Cclip startup overhead and flow
  - Eliminated redundant startup work in cclip path (preflight list + duplicate initial filter scenarios).

Technical details

- `--stdout` mode is integrated into launcher run flow with JSON serialization (`serde_json`) and non-interactive output path.
- Cclip image runtime now separates:
  - fast initial env-based adapter hinting for first draw
  - lazy stdio picker refinement for image selections
- Global display state handling for image runtime now guards manager swap transitions.
- Release workflow now normalizes release-body asset URLs before `gh release create`, preserving dist compatibility while keeping user-facing links correct.

Documentation

- `USAGE.md` and CLI help text updated for `--stdout` and cclip behavior changes.
- Release workflow docs/behavior aligned with correct release-download URL paths.
- Version references and release metadata updated for `3.5.0-kiwicrab` on the release branch.

Notes

- SemVer: MINOR (`3.4.1` -> `3.5.0`) because this release adds new user-facing functionality (`--stdout`) while also shipping important cclip/runtime fixes.
- Rationale: 3.5.0 improves both scriptability and daily UX: launcher output is automatable, cclip startup is more responsive, image preview handling is safer under runtime transitions, and generated release notes now point to correct downloadable assets.

Contributors

- @Mjoyufull
- @myume
- Code review: @cubic-dev-ai, @devin-ai-integration

Compatibility

- Language/runtime: Rust 1.94+ stable; edition remains 2024.
- Platforms: GNU/Linux and *BSD (unchanged).
- Config / database: compatible; no schema migration required.
- Breaking: none.

## Download fsel 3.5.0-kiwicrab

|  File  | Platform | Checksum |
|--------|----------|----------|
| [fsel-aarch64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.0/fsel-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.0/fsel-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [fsel-x86_64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.5.0/fsel-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.5.0/fsel-x86_64-unknown-linux-gnu.tar.xz.sha256) |

---

[3.4.1-kiwicrab]

Fixed

- Long text input now scrolls horizontally in dmenu and cclip modes instead of wrapping out of view in the single-line input panel (from pr #67)
  - Query text now follows the same horizontal-scroll behavior as the app launcher, keeping the cursor and end of the input visible while typing.
  - This release also carries small cleanup follow-ups in related input, event, and launcher session code paths.
- Empty stale launcher lockfiles no longer trap startup in a retry loop after a crash leaves behind a 0-byte lockfile (from pr #63)
  - fsel now re-validates the lockfile immediately before deleting it, avoiding a race where another process could have written a valid lockfile between read and cleanup.
  - Missing empty lockfiles are treated as a non-fatal retry case instead of causing startup to fail.
- NixOS `lib.getExe` resolution now works reliably for the fsel package (from pr #66)
  - Added `mainProgram = "fsel"` to `flake.nix` so wrappers and configs that rely on `lib.getExe` resolve the correct executable.

Technical details

- dmenu and cclip input rendering now compute a horizontal `scroll_x` from the rendered line width and use `Paragraph::scroll((0, scroll_x))` instead of wrapping.
- Empty-lock cleanup continues through `remove_lockfile_if_unchanged(...)`, which re-reads the file contents before removal and safely handles missing files.
- Nix packaging metadata now declares the package's primary executable explicitly via `mainProgram`.

Documentation

- Release metadata and version references updated for `3.4.1-kiwicrab`.

Notes

- SemVer: PATCH (3.4.0 -> 3.4.1). This release is backward-compatible and focused on bug fixes plus a small packaging metadata correction.
- Rationale: 3.4.1 tightens a few rough edges in daily use. Long queries stay visible in dmenu and cclip, stale empty lockfiles no longer wedge launcher startup, and Nix wrappers depending on `lib.getExe` resolve fsel correctly.

Contributors

- @Mjoyufull
- @Qewa2
- Co-authored-by: @devin-ai-integration
- Code review: @cubic-dev-ai, @devin-ai-integration

Compatibility

- Language/runtime: Rust 1.94+ stable; crate edition remains 2024.
- Platforms: GNU/Linux and *BSD (unchanged); NixOS packaging behavior is improved.
- Config / database: compatible; no required config migration and no database/schema changes.
- Breaking: none.


## Download fsel 3.4.1-kiwicrab

|  File  | Platform | Checksum |
|--------|----------|----------|
| [fsel-aarch64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.4.1/fsel-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.4.1/fsel-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [fsel-x86_64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.4.1/fsel-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.4.1/fsel-x86_64-unknown-linux-gnu.tar.xz.sha256) |

---

[3.4.0-kiwicrab]

Breaking changes

- Build toolchain
  - Minimum supported Rust toolchain is now 1.94 stable, and the crate now builds as a Rust 2024 edition project.
  - Source builders, packagers, and downstream contributors should update their Rust toolchains before upgrading.

Added

- Standards-driven refactor and module split (from pr #52)
  - Fsel now has a lib-backed entrypoint, dedicated app/platform/path modules, typed config schema modules, and smaller mode-local boundaries across app launcher, dmenu, and cclip.
  - Added `CODE_STANDARDS.md`, checked-in rustfmt/lint policy, locked CI checks, rustdoc verification, CLI behavior tests, and a legacy-config fixture.
- App launcher desktop-action filtering (from pr #59, fixes #54)
  - New `--filter-actions[=no]` flag hides desktop action entries such as "New Window" and "Open Private Window".
  - New `[app_launcher].filter_actions` config key and `FSEL_APP_LAUNCHER_FILTER_ACTIONS` environment override.
  - Desktop discovery, launcher search, help text, and docs all respect the filter; the default remains off.

Changed

- Internal architecture and runtime boundaries (from pr #52)
  - Split monolithic CLI, config, desktop discovery, ranking, state, dmenu, and cclip code into smaller focused modules.
  - Centralized terminal lifecycle, runtime path construction, process handling, lock/session ownership, and platform-specific process helpers.
  - Replaced boolean-heavy desktop discovery plumbing with typed options and clearer module boundaries.
- Contributor and project docs
  - `CONTRIBUTING.md` now points code contributors at `CODE_STANDARDS.md` and reflects the refactored project layout.
  - `PROJECT_STANDARDS.md` was updated during this cycle to clarify docs responsibilities and release workflow.

Fixed

- Direct launch exact-match behavior (from pr #60, closes #58)
  - `fsel -p` now respects `--match-mode exact` and refuses near matches instead of launching the closest fuzzy result.
  - Exact mode accepts case-insensitive exact app-name or executable-name hits; fuzzy mode keeps the existing best-effort behavior.
  - Error text, CLI help, README, usage guide, config comments, and man page now describe direct-launch match-mode behavior consistently.
- Refactor correctness fixes shipped with pr #52
  - `MatchMode::Exact` is now actually enforced in launcher matching instead of being ignored internally.
  - Dmenu and cclip now honor explicit per-mode `title_panel_position` config overrides.
  - Config enum values are case-insensitive, and invalid values now fall back to defaults instead of hard-failing config load.
  - Desktop localization now prefers more specific locales correctly, and desktop cache keys now support non-UTF-8 paths.
  - `process_exists` now treats `EPERM` correctly, and launcher replace logic gives SIGTERM a real grace period before escalation.

Technical details

- Introduced `src/lib.rs` and a thin binary entrypoint; application startup now routes through `fsel::run()` / `fsel::cleanup_after_error()`.
- `src/cli.rs`, `src/config.rs`, `src/core/state.rs`, `src/desktop/app.rs`, `src/modes/cclip/run.rs`, `src/modes/dmenu/run.rs`, `src/ui/dmenu_ui.rs`, and `src/process.rs` were decomposed into focused submodules.
- Config loading now uses typed schema/default/error/env modules rather than one large stringly file, with explicit validation and env override handling.
- Launcher session ownership now uses structured lock contents and content-verified removal; desktop cache keys use raw-byte encoding with legacy fallback.
- The app launcher direct-launch path now branches on `MatchMode` and uses dedicated exact vs fuzzy selection logic with regression tests.

Documentation

- README / USAGE.md / config.toml / fsel.1
  - Documented `--filter-actions`, `[app_launcher].filter_actions`, `FSEL_APP_LAUNCHER_FILTER_ACTIONS`, and the `-p` exact-mode behavior.
  - Updated version references for `3.4.0-kiwicrab` and refreshed toolchain guidance.
- CONTRIBUTING.md / PROJECT_STANDARDS.md / CODE_STANDARDS.md
  - Added the code standards handbook, updated contributor guidance, and tightened release/doc workflow guidance.

Notes

- SemVer: MINOR (3.3.1 -> 3.4.0). This release adds a new app-launcher feature and ships a large internal refactor while keeping the normal CLI, config, and database surface compatible for users.
- Rationale: 3.4.0 is the "make the codebase sane again" release. It lands the long-planned CODE_STANDARDS-driven refactor, adds optional filtering for noisy desktop action entries, and makes direct `-p` exact matching behave the way users expect.

Contributors

- @Mjoyufull
- @ArtikusHG (pr #59) thank you soo much for the first contribution
- Code review: @cubic-dev-ai, @devin-ai-integration

Compatibility

- Language/runtime: Rust 1.94+ stable; crate edition is now 2024.
- Platforms: GNU/Linux and *BSD (unchanged).
- Config / database: compatible; no database migration and no required config migration. Legacy app-launcher aliases remain supported.
- Breaking: source builders need a current toolchain; `filter_actions` is new and off by default; `-p` now strictly obeys `--match-mode exact`.


## Download fsel 3.4.0-kiwicrab

|  File  | Platform | Checksum |
|--------|----------|----------|
| [fsel-aarch64-unknown-linux-gnu.tar.axz](https://github.com/Mjoyufull/fsel/releases/download/3.4.0/fsel-aarch64-unknown-linux-gnu.tar.xz) | ARM64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.4.0/fsel-aarch64-unknown-linux-gnu.tar.xz.sha256) |
| [fsel-x86_64-unknown-linux-gnu.tar.xz](https://github.com/Mjoyufull/fsel/releases/download/3.4.0/fsel-x86_64-unknown-linux-gnu.tar.xz) | x64 Linux | [checksum](https://github.com/Mjoyufull/fsel/releases/download/3.4.0/fsel-x86_64-unknown-linux-gnu.tar.xz.sha256) |

---

[3.3.1-kiwicrab]

Fixed

- Zero title panel height across modes (from pr #47)
  - `title_panel_height_percent = 0` now fully hides the title/content panel in app launcher, dmenu, and cclip instead of reserving empty space.
  - Layout and scrolling math now saturate cleanly when panels consume all available height, avoiding underflow, off-by-one visibility issues, and forced preview rows.
  - Content and image preview rendering are gated behind visible-panel checks, so hidden panels no longer leave gaps or stale preview behavior.


- Usage.md updated to reflect `fsel -c`   @Marbowls 

- CLI help coverage and tag validation (from pr #49)
  - `-h` and `--help` now document the full supported CLI surface, including `-c`, with clearer behavior descriptions and examples.
  - Unknown-option quick help now matches the updated wording and supported option set.
  - `--tag wipe` now errors unless `--cclip` is also set, preventing invalid standalone usage.

Technical details
- UI layout helpers
  - Shared effective and saturating height helpers now handle zero-height title panels consistently across app launcher, dmenu, and cclip.
  - Regression tests cover zero-height behavior, rounding, and saturated item-panel calculations.

- CLI and docs sync
  - Runtime help, quick error help, and README usage text now describe the same option surface and behavior.
  - `--tag wipe` validation now matches the documented requirement that it is a cclip-only flow.

Documentation

- README / USAGE / fsel.1
  - Updated CLI help text, option descriptions, and version references for `3.3.1-kiwicrab`.

- config.toml
  - Documented that `title_panel_height_percent = 0` hides the title/content panel and frees that space for the list.

Notes

- SemVer: PATCH (3.3.0 -> 3.3.1). This release is backward-compatible and contains bug fixes plus help and documentation alignment.
- Rationale: this release tightens two rough edges in daily use. Zero-height panel configs now behave consistently, and the CLI help and validation path now matches the actual supported interface.

Contributors

- @Mjoyufull
- @Marbowls 
- Code review: @cubic-dev-ai

Compatibility

- Language/runtime: Rust 1.90+ (unchanged).
- Platforms: GNU/Linux and *BSD (unchanged).
- Config / database: compatible; no migration and no database/schema changes.
- Behavior: `title_panel_height_percent = 0` is now fully supported across app launcher, dmenu, and cclip; `--tag wipe` remains a cclip-only command and requires `--cclip`.

---

[3.3.0-kiwicrab]

Breaking changes

- App launcher launch methods
  - Removed Sway-specific launch integration and `$SWAYSOCK` autodetection from code (from pr #43).
  - Removed `-s` / `--nosway` from the launcher path.
  - To keep the old Sway behavior, use `--launch-prefix="swaymsg exec --"` or `[app_launcher].launch_prefix = ["swaymsg", "exec", "--"]`.
  - No migration is performed; existing configs, scripts, or aliases that relied on the old Sway path must be updated manually.

Added

- Generic launch prefix support (from pr #43)
  - New `--launch-prefix "<CMD>"` CLI option for launching apps through a custom argv prefix.
  - New `[app_launcher].launch_prefix = ["..."]` config support for persistent custom launch wrappers.
  - New `FSEL_APP_LAUNCHER_LAUNCH_PREFIX` environment override.
- Installation guidance updates (from pr #41 and follow-up docs)
  - README now includes Void Linux install guidance.
  - README now includes the `fsel-bin` AUR package alongside `fsel-git`.

Changed

- App launcher execution path (from pr #43)
  - `--uwsm` and `--systemd-run` now resolve to launch prefixes internally and share the same argv-based launch path as custom prefixes.
  - Launch-method validation and error text now consistently describe `--launch-prefix`, `--systemd-run`, and `--uwsm`.
  - Detach behavior and documentation now describe the generic launch-prefix flow instead of a WM-specific path.

Fixed

- Terminal launcher parsing (from pr #43)
  - `terminal_launcher` now respects quoted arguments via `shell_words::split`, so values like `kitty --class "fsel term" -e` parse correctly.
- Launch-method override precedence (from pr #43)
  - CLI parsing preserves the intended "last one wins" behavior while still rejecting conflicting active launch methods.

Technical details

- Launching is now unified around a single prefix-plus-command argv path in `src/cli.rs` and `src/modes/app_launcher/launch.rs`.
- Config support for `launch_prefix` uses structured string arrays, avoiding shell-splitting ambiguity in `config.toml`.
- `--uwsm` maps to `["uwsm", "app", "--"]` and `--systemd-run` maps to `["systemd-run", "--user", "--scope"]` through the same internal path.

Documentation

- README / USAGE / config.toml / fsel.1: documented `--launch-prefix`, `[app_launcher].launch_prefix`, launcher precedence, and migration away from Sway-specific launch handling (from pr #43).
- README: added Void Linux installation instructions (from pr #41).
- README: added `fsel-bin` AUR install guidance (follow-up docs commit).
- Version refs updated for `3.3.0-kiwicrab` in release artifacts and packaging metadata.

Notes

- SemVer: release branch targets MINOR (`3.2.0` -> `3.3.0`) for the new generic launch-prefix feature, but this release also removes the old Sway-specific launcher path, so users should read Breaking changes carefully.
- Rationale: the generic prefix mechanism makes launcher integration WM-agnostic, simplifies the execution model, and keeps `uwsm`, `systemd-run`, and custom wrappers on one maintained path.

Contributors

- @Mjoyufull
- @eiseq (pr #41)
- Code review: @cubic-dev-ai

Compatibility

- Language/runtime: Rust 1.90+ (unchanged).
- Platforms: GNU/Linux and *BSD (unchanged).
- Config / database: compatible; new `launch_prefix` config is optional and no database migration is required.
- Breaking: if you relied on automatic Sway launch handling or `-s` / `--nosway`, switch to an explicit launch prefix.

---

[3.2.0-kiwicrab]

Added

- App launcher ranking and pinned ordering (from pr #36)
  - `ranking_mode` in `[app_launcher]`: `frecency` (default), `recency`, or `frequency`; controls how apps are sorted and how tie-break boosts are applied.
  - `pinned_order`: `ranking` (default), `alphabetical`, `oldest_pinned`, or `newest_pinned`; deterministic ordering of pinned apps via stored `pin_timestamps` with automatic backfill for existing pins.
  - Environment overrides: `FSEL_RANKING_MODE`, `FSEL_PINNED_ORDER`, `FSEL_APP_LAUNCHER_RANKING_MODE`, `FSEL_APP_LAUNCHER_PINNED_ORDER`.
  - Debug logs show active `ranking_mode` and `pinned_order` and the ranking label in the score breakdown.
  - Addresses feature request in issue #25.

Changed

- Dependencies (from pr #38)
  - Updated Cargo.lock: tokio 1.50.0, rustix 1.1.4, image 0.25.10, redb 3.1.1, which 8.0.2, and related patches across the stack.
- GitHub: funding options updated.

Fixed

- cclip image preview performance (from pr #35, addresses issue #33)
  - Reduced redraw pressure during image preview so large Foot/Sixel windows no longer flood the event queue and stall navigation.
  - cclip uses demand-driven redraws (synthetic 60 FPS render stream disabled for cclip); explicit redraw when background image loading completes.
  - Terminal sync/clearing improved for Kitty/Sixel/Foot to avoid flicker and stale buffers.
- Nix build (from pr #32)
  - Updated flake.lock and tooling for Rust 1.90 (quantette 0.5.1 requirement); Nix builds succeed again.

Technical details

- Ranking: `pin_timestamps` stored in redb for `oldest_pinned` / `newest_pinned`; missing timestamps are backfilled for existing pinned apps (no migration step). Pin state is not persisted when pinned-app loading fails, avoiding pin wipe on transient DB errors.
- Cclip: optional input `render_rate`  default 16 ms elsewhere, `None` for cclip for demand-driven redraws; resize and image-load completion trigger explicit redraws.

Documentation

- README / USAGE / config.toml / fsel.1: `ranking_mode` and `pinned_order` options, env overrides, and examples (from pr #36).
- Hyprland: windowrule examples updated to new block syntax (from pr #34).
- README: Niri integration added — window rule for `title="launcher"` and Mod+D bind example (from pr #37).
- Nix/Rust: README, CONTRIBUTING, and project standards reference Rust 1.90+ (from pr #32).

Notes

- SemVer: MINOR (3.1.0 → 3.2.0). New features (ranking modes, pinned order) and fixes; config and database remain backward-compatible.
- Rationale: Configurable ranking and pinned order improve predictability and control; cclip fix restores responsive navigation with image previews; dependency refresh and Nix fix keep builds and runtime current.

Contributors

- @Mjoyufull
- @asklipiosd (pr #34)
- @movedtocodeberg (pr #37)
- @akotro (pr #32)
- Code review: @cubic-dev-ai

Compatibility

- Language/runtime: Rust 1.90+ (Nix/CI and quantette require 1.90; see pr #32).
- Platforms: GNU/Linux and *BSD (unchanged).
- Config / database: Compatible; new `[app_launcher]` keys and `pin_timestamps` are optional with defaults and backfill.

---

[3.1.0-kiwicrab]

[3.1.0-kiwicrab] Latest

Breaking changes

- **Image previews no longer use chafa**
  - Clipboard image previews are now rendered inside the TUI via [ratatui-image](https://github.com/benjajaja/ratatui-image). The `chafa` binary is no longer used or required.
  - If you relied on chafa for cclip image previews, uninstall it; 3.1.0 uses built-in Kitty/Sixel/Halfblocks support only.
- **Dependencies**
  - Removed `base64` crate (was only used for manual Kitty protocol encoding). No migration; existing config and data unchanged.

Added

- **Native TUI image previews in cclip mode** (from pr #24)
  - Inline image preview in the content panel when an image clipboard entry is selected.
  - Fullscreen image preview (Alt+i); exit with Esc or q.
  - Automatic terminal protocol detection (Kitty, Sixel, Halfblocks) via ratatui-image; no external image viewer.
  - New `ImageManager` in `ui/graphics.rs`: centralized load/render, LRU cache (50 entries), async load with 5s timeout, decode offloaded with `spawn_blocking`.
  - Display states: Empty, Image, Loading, Failed; preview header and status reflect current state.
- **Clipboard copy**
  - Wayland: single `cclip copy <rowid>` (no more `cclip get | wl-copy` pipeline).
  - Mouse-click copy uses the same path as Enter/Ctrl+Y: `CclipItem::from_line` and `copy_to_clipboard()`; parse errors shown in the UI.
- **Cclip UX and robustness**
  - `reload_and_restore` helper for delete/tag/untag; selection and scroll restored by rowid; scroll_offset clamped so the selected item stays visible.
  - `failed_rowids` set to avoid endless retry on repeated load failures.
  - Fullscreen preview loop: bounded tolerance for consecutive input errors so the UI cannot hang on persistent `input.next()` failures.
  - Navigation uses shared `max_visible` to avoid u16 underflow on very small terminals.

Changed

- **Graphics and detection**
  - `GraphicsAdapter::detect()` uses Picker-based protocol when available (Kitty, Sixel, Halfblocks, iTerm2); env fallback only when Picker is not used.
  - Dmenu mode: removed `Picker::from_query_stdio()` on startup; uses `GraphicsAdapter::detect(None)` env fallback only (no image rendering in dmenu).
- **Cclip parsing and UI**
  - Image detection uses `is_cclip_image_item` (mime type) instead of `get_cclip_rowid` (which applied to all entries), so text entries are no longer treated as images.
  - `get_cclip_rowid`, `get_image_info`, and related parsing in `dmenu_ui.rs` use `splitn` consistently; duplicate `get_cclip_rowid` removed.
  - `DisplayState`: simplified to `Image(String)` (unused `Rect` removed); later `DisplayState` made `std::sync::Mutex` with poison recovery (`unwrap_or_else(|e| e.into_inner())`) for consistency.
- **Dependencies / build**
  - Added: `ratatui-image` 10.0 (crossterm, tokio), `image` 0.25 (png, jpeg, gif, bmp, webp; no `image-defaults`).
  - Removed: `base64`.
  - Ratatui 0.30 constraints API: `.as_ref()` removal for compatibility.

Fixed

- Fullscreen image preview loop could hang when `input.next()` repeatedly returned errors; now exits after bounded consecutive failures.
- Stale image state when `load_cclip_image` failed; state is cleared and UI shows failure instead of previous image.
- Navigation in cclip could panic in debug on very small terminals due to unchecked u16 subtraction; layout now uses shared `max_visible` and safe bounds.
- Scroll position after reload (delete/tag/untag) could leave the selected item off-screen; `scroll_offset` is clamped so the selected item remains visible.
- Parse errors on clipboard copy (Enter and mouse) were dropped; they are now shown via the UI temp message.
- Mutex poisoning in `DISPLAY_STATE` could drop updates; all lock sites now use poison recovery so state stays consistent after a panic.
- In modes/cclip/run.rs fixed maintain selection index after item deletion (from pr #27)
Technical details

- **ImageManager** (`src/ui/graphics.rs`): `load_cclip_image(rowid)` runs `cclip get <rowid>` via `tokio::process::Command` with 5s timeout; stdout is decoded in `tokio::task::spawn_blocking` and stored as `StatefulProtocol` in an LRU cache; `render()` draws via `StatefulImage` and propagates `last_encoding_result()`.
- **DisplayState**: global `DISPLAY_STATE` (std sync Mutex) holds `Empty | Image(rowid) | Loading(rowid) | Failed(msg)`; updated from ImageManager and cclip run loop; used by `info_with_image_support` for status text.
- **Cclip run loop**: ImageManager created with Picker (or halfblocks fallback); graphics adapter cached once per run; background task loads images and updates DISPLAY_STATE/failed_rowids; fullscreen modal reuses same ImageManager and clears or restores state on exit.

Documentation

- **PROJECT_STANDARDS.md** (v1.3.0, from pr #26): Release and hotfix rules; main updated only via release or hotfix branches; release branches for version bumps and docs only; hotfix process and main --> dev sync; tag = version number only; GitHub release title `[version-codename]`; release body template and section rules.
- **README, USAGE, CONTRIBUTING**: Chafa removed from requirements; note that pre-3.1.0 still uses chafa, 3.1.0+ uses built-in ratatui-image.
- **config.toml**: Comment for image preview updated (Kitty/Sixel/Halfblocks, no chafa).
- **fsel.1**: Version 3.1.0-kiwicrab, date 2026-02-23; app launcher lock path corrected to `~/.local/share/fsel/fsel-fsel.lock`; `--cclip` and Alt-i description updated; `[cclip]` example includes `hide_inline_image_message`.

Notes

- SemVer: MINOR (3.0.0 --> 3.1.0). New feature (native image previews), no breaking config or database format; only removal of chafa requirement and of the base64 dependency.
- This release ships the ratatui-image–based cclip preview (pr #24) and the updated project standards (pr #26), plus docs and manpage fixes for 3.1.0, and the fix for item deletion (pr #27)

Contributors

- @Mjoyufull
- Code review and suggestions: @coderabbitai (which lowkey sucked and didn't understand the codebase so i switched to ) -->, @cubic-dev-ai

Compatibility

- **Language/runtime:** Rust 1.89+ (unchanged).
- **Platforms:** GNU/Linux and *BSD (unchanged).
- **Config / database:** Compatible; no migration. `[cclip]` options unchanged.
- **dev,note:** PRE 3.1.0 STILL USES CHAFA I CANNOT GO BACK IN TIME

---

[3.0.0-kiwicrab]

## Breaking changes

- **Database and cache format**
  - Serialization changed from bincode to postcard. Existing history, cache, and pinned apps are not migrated. On first run after upgrading, history and cache will be reset/rebuilt. Re-pin apps and reconfigure as needed.

## Added

**from pr #23**

- **TTY mode**
  - `-t, --tty` flag and `terminal_launcher = "tty"` config option for launching terminal apps in the current terminal.
  - In TTY mode fsel replaces itself with the target application using `exec`, so the launched app takes over the session (e.g. htop, vim).
  - History and frecency are recorded before process replacement; uwsm/systemd prefixes are bypassed in TTY mode.

- **Clipboard entry deletion (cclip)**
  - `Alt+Delete` keybind to delete the selected clipboard item (calls `cclip delete <ID>`).
  - Selection and scroll position preserved after deletion (next item becomes selected at the same index).
  - Deletion failures reported in the UI; active tag filter maintained when reloading after delete.

- **Environment overrides**
  - `FSEL_*` environment variables override config for layout, dmenu, cclip, and app_launcher options (e.g. `FSEL_TERMINAL_LAUNCHER`, `FSEL_CCLIP_IMAGE_PREVIEW`).

- Version bump to 3.0.0-kiwicrab (new codename: kiwicrab).

## Changed

- **App launcher launch flow**
  - Target executable is validated and resolved (PATH search, executable bit) before opening the write transaction and updating history/frecency.
  - Failed execs are no longer recorded as successful launches; resolved path used for exec.

- **Dependencies and serialization**
  - Replaced `bincode` with `postcard` 1.1 for all database and cache serialization.
  - Replaced `chrono` with `time` 0.3 for debug logging.
  - Replaced `config` crate with direct TOML parsing and env handling.
  - Replaced `regex` with `strip-ansi-escapes` for ANSI stripping.
  - Replaced `dirs` with `directories` crate equivalents.
  - Ratatui updated to 0.30 with `layout-cache` feature for faster layout calculations and UI rendering.
  - Tokio 1.49 with slimmer feature set; nucleo-matcher 0.3.1, unicode-width 0.2.2, directories 6.0.

- **CLI**
  - Duplicate `prefix_depth` field removed; `-t` shorthand for `--tty` parsing fixed.
  - When TTY mode is detected, `terminal_launcher` config is normalized to empty string to prevent misuse.

- **Build and binary**
  - Dependency count reduced (e.g. from ~317 to ~229); release binary size reduced (e.g. ~4.5MB to ~2.7MB).
  - Resolved `atomic-polyfill` audit by disabling unused `heapless` features in postcard.

## Fixed

- cclip deletion error handling: failures from `cclip delete` are reported to the user; tag filter persists when reloading history after deletion.
- Selection jump after cclip delete: selection index and scroll offset preserved so the list does not jump to the top.

## Breaking changes

- **Database and cache format**
  - Serialization changed from bincode to postcard. Existing history, cache, and pinned apps are not migrated. On first run after upgrading, history and cache will be reset/rebuilt. Re-pin apps and reconfigure as needed.

## Technical details

- **TTY mode:** Uses `CommandExt::exec` to replace the fsel process with the target; launch logic records history/frecency before exec and skips environment-specific prefixes in TTY mode.
- **cclip delete:** New `cclip_delete` keybind (default Alt+Delete), configurable in keybinds.toml; `delete_item` helper invokes `cclip delete <ID>`; UI preserves selection and scroll after reload.
- **Config:** Direct TOML parsing with `toml` crate; env overrides applied per-option for general, layout, dmenu, cclip, and app_launcher sections.
- **Persistence:** redb still used for database; postcard for serialization of history, frecency, pinned apps, cache entries, and tag metadata.

## Documentation

- README: version references 3.0.0-kiwicrab; install and usage updated.
- USAGE.md: TTY mode, cclip deletion (Alt+Delete), env overrides; Field Reference includes `prefix_depth` and `terminal_launcher = "tty"`; debug example date updated.
- Man page (fsel.1): version 3.0.0-kiwicrab; `-t, --tty`; Alt+Delete for cclip delete; config example comment for `terminal_launcher = "tty"`.
- config.toml: comment for `terminal_launcher` "tty" option.
- CONTRIBUTING.md / PROJECT_STANDARDS.md: release examples and codename policy updated for 3.x (kiwicrab).

## Notes

This is a **MAJOR** release under [Semantic Versioning 2.0.0](https://semver.org/):

- Breaking change: database/cache format (bincode → postcard). Existing data is reset on first run.
- New features (TTY mode, cclip deletion, env overrides) are additive.
- Config file structure and CLI flags remain compatible except for the persistence format change.

**Rationale:** TTY mode enables seamless terminal app launching from TTY login; cclip deletion improves clipboard workflow; dependency modernization reduces binary size, dependency count, and addresses maintenance/security (e.g. unmaintained bincode, audit findings) preparing for sum big type shi.

## Contributors

- @Mjoyufull
- Co-authored-by: @coderabbitai (review and follow-up suggestions)

## Compatibility

- **Rust:** 1.89+ stable (unchanged).
- **Platforms:** GNU/Linux and *BSD (unchanged).
- **Config:** config.toml structure and options remain compatible; no new required keys.
- **Breaking:** History, cache, and pinned apps use the new postcard format. First run after upgrade will reset/rebuild them. Back up or re-pin if needed.

---

[2.5.0-seedclay]

## Added
from pr #22
- Advanced search ranking system
  - 12-tier prioritization system with sophisticated scoring algorithm
  - Prioritizes pinned apps, exact matches (app name and executable), prefix matches, and word-start matches
  - Metadata matching on keywords and categories with configurable scoring
  - Configurable prefix depth for fine-tuning when prefix matches take priority over fuzzy matches
  - Frecency boost (additive) combined with fuzzy matcher scores for intelligent ranking
  - Ensures the most relevant apps surface first, making it easy to find what you're looking for

from pr #21
- Improved error handling for stale lock file cleanup (cclip mode)
  - Better handling of corrupted or invalid PID values in lock files
  - Graceful error recovery when lock file cleanup fails (non-blocking warnings)
  - Prevents startup failures due to stale lock files from crashed processes

- Version bump to 2.5.0-seedclay


## Changed

- Search ranking algorithm
  - Replaced simple scoring with sophisticated 12-tier bucket system
  - Pinned apps now have distinct priority tiers (120M-20M score range)
  - Normal apps have separate tiers (90M-0 score range)
  - Exact matches (app name and executable) prioritized over prefix matches
  - Prefix matches prioritized over word-start matches within configurable depth
  - Word-start matches prioritized over fuzzy matches
  - Metadata matches (keywords, categories) receive dedicated scoring tiers
  - Frecency boost applied additively after bucket scoring for fine-grained ranking

- Search behavior
  - Within prefix depth, prefix and word-start matches take priority over fuzzy matches
  - Executable name matches receive 2x weight in fuzzy scoring
  - Fuzzy matcher scores multiplied by 100 and added to bucket scores
  - Frecency scores multiplied by 10 and added to final score for time-based prioritization


## Fixed

- cclip mode lock file handling
  - Stale lock files from crashed processes no longer block startup
  - Corrupted lock files (invalid PID format) are automatically cleaned up
  - Lock file cleanup errors are logged as warnings instead of fatal errors
  - Better error messages when lock file operations fail


## Technical Details

- Search ranking implementation
  - 12 distinct scoring tiers based on match type and pin status:
    - Pinned App Name Exact: 120,000,000
    - Pinned Exec Name Exact: 115,000,000
    - Pinned App Name Prefix: 110,000,000
    - Pinned Exec Name Prefix: 105,000,000
    - Pinned App Name Word-Start: 100,000,000
    - Pinned Exec Name Word-Start: 95,000,000
    - Normal App Name Exact: 90,000,000
    - Normal Exec Name Exact: 85,000,000
    - Normal App Name Prefix: 80,000,000
    - Normal Exec Name Prefix: 75,000,000
    - Normal App Name Word-Start: 70,000,000
    - Normal Exec Name Word-Start: 65,000,000
    - Pinned Metadata Match: 40,000,000
    - Normal Metadata Match: 30,000,000
    - Pinned Fuzzy Match: 20,000,000
    - Normal Fuzzy Match: 0 (base)
  - Fuzzy matcher scores (from nucleo-matcher) multiplied by 100 and added to bucket score
  - Frecency scores (from zoxide-style algorithm) multiplied by 10 and added to final score
  - Prefix depth configuration (default: 3) determines when prefix/word-start matches take priority
  - Word-start detection uses regex to find word boundaries in app names and executables

- Lock file cleanup
  - Process existence check before attempting cleanup
  - Graceful error handling with warning messages instead of fatal errors
  - Non-interactive commands (tag clear, tag list, tag wipe) skip lock file checks entirely
  - Lock file removal errors are logged but don't prevent startup


## Documentation

- README
  - Added prominent feature description for advanced search ranking system
  - Updated version references to 2.5.0-seedclay

- Man page (fsel.1)
  - Version updated to 2.5.0-seedclay
  - Date updated to 2026-01-06


## Notes

- This is a MINOR release under SemVer:
  - New features added in a backward-compatible manner (advanced search ranking, improved error handling)
  - No breaking changes to config format or database schema
  - Existing search behavior enhanced but remains compatible

- Rationale:
  - Advanced search ranking ensures users find the apps they're looking for faster
  - 12-tier system provides clear prioritization hierarchy
  - Improved lock file handling prevents frustrating startup failures
  - Frecency integration maintains context-aware ranking while respecting explicit match quality


## Contributors

- @Mjoyufull


## Compatibility

- Rust 1.89+ (unchanged)
- GNU/Linux and *BSD (unchanged)
- Config and database formats remain compatible; no destructive schema changes
- Existing pinned apps and frecency data preserved

---

[2.4.0-seedclay]

## Added
from pr #19 
- Async core & TEA-inspired UI
  - Migrate main loop to tokio + ratatui using a state/message/update pattern inspired by The Elm Architecture.
  - Enables non-blocking event handling and clearer state flow for future features.

- Frecency ranking (Zoxide-style)
  - Time-bucketed scoring combining recency and frequency for more contextually relevant results.
  - Recently used apps (within 1 hour) get 4x boost; within 1 day get 2x; older entries are deprioritized.
  - Automatic aging when total scores exceed threshold to prevent unbounded growth.

- Parallel app scanning with jwalk + rayon
  - Directory walking now uses jwalk for parallel filesystem traversal.
  - Desktop file parsing parallelized with rayon for faster startup on systems with many apps.
  - File list caching: subsequent launches skip directory walking entirely when no changes detected.

- config-rs based configuration
  - Replaced manual TOML parsing with config-rs for more robust config handling.
  - Supports environment variable overrides with FSEL_ prefix.

- Fullscreen image viewer input lock (cclip mode)
  - Alt+i now enters a focused viewing mode where all input is ignored except Esc/q/Ctrl+C.
  - Prevents accidental navigation or selection while examining clipboard images.

- Version bump to 2.4.0-seedclay


## Changed

- Startup & loading
  - Preserve synchronous, instant-visible startup while using async event loop internally.
  - All apps loaded upfront before first render for instant display.
  - App detection now caches the file list with directory mtime tracking; cache invalidates automatically when apps are added/removed.

- UI parity & input behavior
  - Restored legacy scroll physics and mouse-selection behavior to match v2.2.0 exactly.
  - Input handling realigned so existing keybinds and selection UX remain consistent.

- Ranking
  - Frecency scoring replaces simple launch-count history for smarter app ordering.
  - Pinned apps still take priority over frecency.

- Dependencies & build
  - Add tokio, ratatui, config-rs, jwalk, rayon (and small supporting crates). Expect a modestly larger binary.


## Fixed

- Parity regressions introduced during refactor
  - Reinstated legacy scroll momentum and mouse selection semantics.
  - Fixed rendering/input desyncs introduced during early async experiments.

- Startup race conditions and graceful shutdown
  - Deterministic drawing order and proper process cleanup on exit.

- Config parsing robustness
  - Legacy config keys handled with backward-compatible defaults and clearer error messages.


## Technical Details

- Runtime & UI
  - Tokio is the async runtime; the app uses `block_on()` to bridge async event handling with synchronous operations.
  - TEA-inspired pattern: State (model), Message (events), update() (state transitions), render() (view).

- App discovery
  - jwalk provides parallel directory traversal (replaces walkdir for performance).
  - rayon parallelizes desktop file parsing across CPU cores.
  - File list cached in redb with directory mtimes; invalidates when directories change.

- Frecency algorithm
  - Zoxide-style time-bucketed scoring:
    - Within 1 hour: score × 4
    - Within 1 day: score × 2
    - Within 1 week: score × 0.5
    - Older: score × 0.25
  - Automatic aging when total scores exceed 10,000 to prevent unbounded growth.

- Configuration
  - config-rs loads defaults then merges user config → environment values.
  - Existing config keys remain supported; no breaking changes to config format.

- Persistence
  - Frecency data stored in redb database alongside history and pinned apps.
  - Existing history preserved; timestamps migrated seamlessly.

- Safety
  - `#![deny(unsafe_code)]` at crate root; limited `#[allow(unsafe_code)]` only for necessary libc calls (process management).


## Documentation

- README / USAGE
  - Updated with frecency ranking notes and current feature set.

- Man page (fsel.1)
  - Version updated to 2.4.0-seedclay.
  - Added frecency description, --tag wipe option, improved examples.


## Notes

- This is a MINOR release under SemVer:
  - New features added in a backward-compatible manner (async internals, frecency, parallel scanning).
  - No breaking user-facing config keys removed; legacy parity prioritized.

- Rationale:
  - Async event loop enables responsive UI and lays groundwork for future background features.
  - Frecency ranking surfaces relevant apps without manual pinning.
  - Parallel scanning with jwalk/rayon significantly improves cold-start performance on large app collections.


## Contributors

- @Mjoyufull
- Acknowledgements to @Marbowls for prior groundwork and testing.


## Compatibility

- Rust 1.89+ (unchanged)
- GNU/Linux and *BSD (unchanged)
- Config and database formats remain compatible; no destructive schema changes.

---

[2.3.0-seedclay]

## Added
- Clean fullscreen image preview in cclip (Alt+I)
  - Kitty: clear screen, center image, position cursor at (0,0) for a clean background.
  - Foot/Sixel: true fullscreen (full terminal size) with explicit pre-clear and top-left origin.
- Config error UX: duplicate key/table detection with actionable guidance
  - Clear message when the same key/table appears more than once.
  - Explains correct placement (root vs [dmenu] vs [cclip]) and avoiding repeated sections.

## Changed
- Foot/Sixel clearing and redraw strategy
  - On text↔image or “image changed” transitions, perform pre-draw terminal.clear(), then Clear all panels inside the draw to re-sync ratatui’s buffer and prevent missing glyphs.
  - Keeps clearing targeted to panels; avoids full-screen flicker in normal navigation.
- Tag-mode image handling in cclip
  - Entering tag edit/create clears the current image and suspends inline preview for the duration of tag mode; automatically resumes on exit.
  - Kitty: use graphics protocol hide. Foot/Sixel: terminal.clear() + panel Clear to sync buffer.
- Rendering pipeline robustness
  - Layout and wrapping calculations moved inside terminal.draw() so compute and render use identical dimensions.
  - Removed post-draw clearing; now use Clear inside the draw pass for correct diffing.
- Process management API (Option A, non-breaking)
  - Added kill_process_sigterm_result(pid) -> io::Result<()>, kept kill_process_sigterm(pid) wrapper.
  - Lockfile semantics: only remove lock on Ok or ESRCH; preserve on EPERM/other errors.
- Version bumped to 2.3.0-seedclay (MINOR, backward-compatible).

## Fixed
- Missing/disappearing characters in Foot due to out-of-sync screen clears.
- Fullscreen preview not truly fullscreen/centered across terminals.
- Lingering text artifacts at panel edges (panel Clear in-draw).
- Clippy warning (collapsed consecutive replace calls in content sanitizer).

## Technical Details
- Terminal/graphics
  - Kitty: use Clear widgets on panels for text, Kitty graphics protocol for image hide.
  - Foot/Sixel: pre-draw terminal.clear() on image transitions; inside draw, Clear content/items/input panels so ratatui repaints all cells atomically.
  - Synchronized updates (DECSET 2026) used around draw where appropriate to avoid mid-frame tearing.
- UI pipeline
  - Wrapping and width measurement aligned with actual render area (computed inside draw).
  - Inline image preview is suspended while in tag modes to prevent redraws during workflows.
- Config parsing
  - read_with_enhanced_errors detects duplicate keys/tables and emits a professional, actionable message.

## Documentation
- README: install/version references updated to 2.3.0-seedclay.
- Man page (fsel.1): version updated and key sections clarified.
- Usage notes: reflect clean fullscreen behavior and improved config error message.

## Notes
- This is a MINOR release under SemVer:
  - Backward-compatible features and UX improvements.
  - No breaking API changes.
  - Patch reset to 0.

## Contributors
- Special thanks to @Marbowls for the original pr #17 and substantial groundwork:
  - Introduced the fallible kill helper and improved lockfile semantics.
  - Helped drive the Foot/Sixel clearing discussion and stability work.
- Follow-up integration, terminal-specific refinements, config error UX, and release prep by me.

## Compatibility
- Rust 1.89+ (unchanged).
- GNU/Linux and *BSD (unchanged).
- Config and database formats remain compatible; no schema changes.

---

[2.2.3-seedclay]

## [2.2.3-seedclay] - 2025-10-29
pushing
#16 by @walldmtd 

### Fixed
- Completed selection reset refactor - removed leftover old logic in `app_ui.rs` that was causing inconsistent selection behavior when filtering (PR #16, continuation of #11)
- Selection now properly resets to first item whenever filter query changes, matching behavior across entire codebase

### Notes
PATCH version bump per Semantic Versioning 2.0.0 - backward compatible bug fix only.

### Credits
- @walldmtd for catching the leftover logic and providing the fix

---

[2.2.2-seedclay]

**Hotfix Release** - UX improvement for tag operations.

### Fixed
- **Cursor Position**: Cursor now stays on current item after tag operations
  - Previously jumped to top of list after creating/editing/removing tags
  - Now preserves both selection and scroll offset
  - Applies to Ctrl+T (tag creation) and Alt+T (tag removal)

### Changed
- **Documentation**: Clarified `--tag clear` behavior
  - Now explicitly states it only clears fsel's tag metadata (colors, emojis)
  - Added notes about clearing cclip tags separately with `cclip tag -d <ID>`
  - Updated in README.md, USAGE.md, fsel.1, and CLI help

### Technical Details
- Scroll offset now preserved when reloading items after tag operations
- Selection restoration uses `old_scroll_offset.min(pos)` to ensure visibility
- Both tag creation and tag removal handlers updated

### Compatibility
- Rust 1.89+ stable required (unchanged)
- GNU/Linux and *BSD support (unchanged)
- Config files fully compatible with 2.2.1
- No database schema changes
- All CLI flags backward compatible

---

[2.2.1-seedclay]

**Hotfix Release** - Critical bug fixes for cclip mode.

### Fixed
- **Lock File Issue**: Non-interactive commands (`--tag clear`, `--tag list`) no longer blocked by lock file check
  - Users can now run tag management commands while cclip TUI is open
  - Lock file only created for interactive TUI mode
- **UTF-8 Crash**: Fixed panic when scrolling through clipboard items with multi-byte UTF-8 characters at 5000-byte boundary
  - Content truncation now respects UTF-8 char boundaries
  - No more crashes on Unicode characters like `─`, `│`, emoji, etc.
- **Invalid Command**: Removed non-existent `cclip clear-tags` command call
  - `--tag clear` now only clears fsel's tag metadata
  - Provides helpful instructions for manual cclip tag clearing

### Technical Details
- Lock file check now skips for `cclip_clear_tags` and `cclip_tag_list` flags
- Content truncation uses `is_char_boundary()` to find valid UTF-8 boundary
- Tag clear command updated with user-friendly guidance

### Compatibility
- Rust 1.89+ stable required (unchanged)
- GNU/Linux and *BSD support (unchanged)
- Config files fully compatible with 2.2.0
- No database schema changes
- All CLI flags backward compatible

---

[2.2.0-seedclay]

### Added
- **Complete Tag System for Clipboard Items**: Full-featured tagging system for organizing clipboard history
  - **Interactive Tag Creation** (Ctrl+T in cclip mode):
    - Multi-step wizard for creating tags with name, emoji, and color
    - Browse and select from existing tags
    - Edit existing tag metadata (emoji/color)
    - Tag metadata stored in fsel's database
  - **Tag Removal** (Alt+T in cclip mode):
    - Remove individual tags from items
    - Remove all tags at once (blank input)
    - Interactive selection from item's tags
  - **Tag Filtering**:
    - `--tag <name>` - Filter clipboard items by tag
    - `--tag list` - List all available tags
    - `--tag list <name>` - List items with specific tag (use `-vv` for details)
  - **Tag Management**:
    - `--tag clear` - Clear all tags and metadata from database
    - `--cclip-show-tag-color-names` - Show color names in tag display (e.g., `[tag(blue)]`)
  - **Tag Display**:
    - Tags shown as `[tagname]` prefix in clipboard list
    - Optional emoji prefixes (e.g., `[📌 important]`)
    - Color-coded tags with customizable colors
    - Tag metadata (emoji, color) persists across sessions
  - **Tag Metadata Storage**:
    - Stored in fsel's redb database (separate from cclip)
    - Supports hex colors, RGB, named colors, and 8-bit colors
    - Emoji support for visual tag identification
- **Config Option**: `show_tag_color_names` in `[cclip]` section for persistent color name display
- **Keybinds**: Configurable tag keybind (default: Ctrl+T) in keybinds.toml

### Changed
- **Major Codebase Refactor**: Reorganized flat module structure into hierarchical organization
  - `src/common/` - Shared item structures and utilities
  - `src/core/` - Cache and database operations
  - `src/desktop/` - Desktop file handling
  - `src/modes/` - Mode-specific implementations (app_launcher, dmenu, cclip)
  - `src/ui/` - UI components, graphics, input handling, keybinds
- **Selection Reset Behavior**: Selection and scroll offset now reset on filter changes (contributed by @walldmtd in #11)
  - Matches rofi behavior for more intuitive filtering
  - Selection no longer stays on stale positions when results change
  - Improves UX when typing or using backspace in dmenu/filter mode
- **Graphics Module**: Moved from `src/graphics.rs` to `src/ui/graphics.rs`
- **Keybinds**: Updated module path references from `crate::keybinds` to `crate::ui::keybinds`
- **Sixel Image Clearing**: Improved logic to prevent text corruption in Sixel terminals

### Fixed
- Tag color names now display correctly in clipboard history UI when enabled
- `to_list_item()` function now preserves formatted tag strings from `display_text`
- Sixel terminal image clearing no longer wipes ratatui-drawn text
- Graphics state management race conditions further refined

### Technical Details
- **Module Organization**: Better separation of concerns with clear module boundaries
- **Code Maintainability**: Improved code organization for easier navigation and maintenance
- **Display State**: Made `DisplayState` publicly exported from graphics module
- **Tag System Architecture**:
  - `TagMode` enum in `src/ui/dmenu_ui.rs` for tag UI state management
  - `TagMetadata` struct in `src/modes/cclip/mod.rs` for tag data
  - `TagMetadataFormatter` for consistent tag display formatting
  - `CclipItem` struct extended with `tags: Vec<String>` field
  - Tag metadata stored in redb `TAG_METADATA_TABLE`
  - Integration with cclip's tag commands (tag, untag, clear-tags, list-tags)
- **Tag Formatting**: Fixed tag display pipeline to respect `include_color_names` parameter
- **Tag Keybinds**: Added `matches_tag()` method to Keybinds struct for configurable tag shortcuts

### Documentation
- Updated README.md with:
  - Tag system feature description
  - Tag management CLI examples
  - Tag keybind documentation (Ctrl+T, Alt+T)
- Updated USAGE.md with:
  - Complete tag management section
  - Interactive tag creation workflow
  - Tag filtering examples
  - Tag removal examples
- Updated man page (fsel.1) with:
  - All tag-related CLI flags
  - Tag keybind reference
  - Tag usage examples
- Updated config.toml with:
  - `show_tag_color_names` option in `[cclip]` section
  - Tag-related comments and examples
- Updated keybinds.toml with:
  - Tag keybind configuration (Ctrl+T)
  - Untag keybind documentation (Alt+T, hardcoded)
  - Tag workflow instructions

### Notes
This is a MINOR version bump (2.1.1 → 2.2.0) per Semantic Versioning 2.0.0:
- New features added in backward compatible manner
- No breaking changes to existing functionality
- Internal refactoring does not affect public API

The codebase refactor improves maintainability without changing user-facing behavior. All existing features continue to work as before.

### Contributors

Special thanks to:
- **@walldmtd** - Selection/scroll reset on filter change (#11)

### Compatibility
- Rust 1.89+ stable required (unchanged)
- GNU/Linux and *BSD support (unchanged)
- Config files fully compatible
- No database schema changes
- All CLI flags backward compatible

---

[2.1.1-seedclay]

### Fixed
- Restore full detach semantics so `--uwsm/--systemd-run` launches no longer leak terminal control when invoked from otter-launcher, matching the 2.0.x behavior (src/helpers.rs).
- Include the `--` separator in `uwsm app -- …` invocations to ensure commands reach the intended target (src/helpers.rs).

### Changed
- Bump release metadata to `2.1.1-seedclay` across Cargo.toml, Cargo.lock, flake.nix, and installation instructions in README.md.

### Removed
- `unbind_proc = true` from otter-launcher examples in USAGE.md; added guidance to leave it disabled for TUIs to avoid raw-input corruption.

---

[2.1.0-seedclay]

### Added
- **Detachment UX** ([#7](https://github.com/Mjoyufull/fsel/issues/7)): Added `-d/--detach` flag plus compatible runtime options (`--systemd-run`, `--uwsm`) so GUI apps like Discord or Steam can launch without tying up the terminal.
- **Help UX**: Two-tier help system. `fsel -h` shows a concise grouped overview; `fsel -H`/`--help` renders a detailed tree reference.
- **Process discovery**: `/proc` scanning (`find_processes_holding_file()`) to locate any PIDs locking `hist_db.redb` during replace operations.

### Changed
- **Replace flow** ([#6](https://github.com/Mjoyufull/fsel/issues/6)): Strengthened `-r/--replace` to terminate all DB holders, verify cleanup, and prevent duplicate sessions across modes before reopening the database.
- **Detach handling** ([#7](https://github.com/Mjoyufull/fsel/issues/7)): Unified detach logic so GUI launches no longer die when the spawning terminal closes; works consistently with or without `--systemd-run` / `--uwsm`.
- **Application discovery** ([#5](https://github.com/Mjoyufull/fsel/issues/5)): Raised recursive depth for XDG searches and respected `--filter-desktop` so `.desktop` files in subdirectories (e.g., Wine apps) appear correctly.
- **Help formatting**: Reordered help text to mirror the short-view grouping and removed ANSI colors for broader compatibility.
- **Version bump**: Updated crate metadata and docs to `2.1.0-seedclay`.

### Fixed
- **Persistent DB lock**: Eliminated “Database already open. Cannot acquire lock.” errors by ensuring previous `fsel` instances exit cleanly before a replace run.
- **GUI detach failures** ([#7](https://github.com/Mjoyufull/fsel/issues/7)): Resolved Discord/Steam processes being killed with terminal exit by performing safe `setsid()` + stdio redirection.
- **Wine app visibility** ([#5](https://github.com/Mjoyufull/fsel/issues/5)): Corrected discovery so `.desktop` entries in nested directories (like `/usr/share/applications/wine/`) respect filter flags and appear in the UI.

[2.1.0-seedclay]: https://github.com/Mjoyufull/fsel/releases/tag/v2.1.0-seedclay

---

[2.0.1-seedclay]

### Fixed

- **Configuration crashes** when uncommenting color/UI options in default config.toml
- **TOML structure** in default configuration - moved all sections to bottom of file to prevent parsing conflicts
- **Error messages** now clearly identify problematic fields and suggest correct placement

### Changed

- **Enhanced error reporting** with clean, compact messages that specify which section contains invalid fields
- **Documentation accuracy** - updated all config references to match actual code implementation

### Removed

- **Tagging references** from user documentation (feature not yet implemented)

This release resolves the primary issue #4  where users experienced crashes when uncommenting configuration options due to improper TOML section placement in the default config file. The configuration structure now follows proper TOML format with all sections at the bottom, preventing field misattribution that caused application crashes.

---

[2.0.0-seedclay]

## Changed

### CLI Flag Names (Breaking Change)
- **Changed flag format from underscores to hyphens** for consistency:
  - `--clear_history` → `--clear-history`
  - `--clear_cache` → `--clear-cache`
  - `--refresh_cache` → `--refresh-cache`
- Old underscore versions will no longer work
- Update your scripts and aliases accordingly

### Database Migration: sled → redb (Breaking Change)
- **Migrated from `sled` to `redb` for all database operations**
- Database file changed from `hist_db` to `hist_db.redb`
- Improved database reliability and performance with ACID-compliant transactions
- Better error handling and transaction safety
- Cleaner API with explicit read/write transactions
- **Data Migration**: Launch history and pinned apps from version 1.X.X and below will not be automatically migrated
  - The new version creates a new database file (`hist_db.redb`)
  - If you have an old database file from 1.X.X (`~/.local/share/fsel/hist_db`), it will remain unused
  - You can safely delete the old database file to free up disk space: `rm ~/.local/share/fsel/hist_db`
  - Launch history (usage counts) and pinned/favorited apps will need to be rebuilt

### Dependency Updates
- Updated `ratatui` from 0.22.0 → 0.29 (major version bump)
  - API changes: `f.size()` → `f.area()`, `terminal.size()` → `terminal.get_frame().area()`
  - Improved rendering performance and stability
- Updated `crossterm` from 0.27 → 0.29
- Replaced `fuzzy-matcher` with `nucleo-matcher` 0.3 (faster SIMD-accelerated fuzzy matching algorithm)
- Updated `toml` from 0.7 → 0.9
- Updated `which` from 4.4 → 8.0
- Updated `shell-words` from 1.0 → 1.1
- Updated `scopeguard` from 1.1 → 1.2
- Updated `walkdir` from 2.3 → 2.5

### Performance Improvements
- Optimized `parse_semicolon_list()` function with early empty-value filtering
- Improved string parsing efficiency in desktop file processing
- Better memory usage with redb's transaction-based architecture
- SIMD-accelerated fuzzy matching with nucleo-matcher

### Code Quality
- Enhanced transaction safety with explicit commit/rollback patterns
- Improved error propagation throughout database operations
- Better separation of concerns between cache and database layers
- Added batch operations for cache updates (batch_get, batch_set)

## Technical Details

### Database Schema Changes
- Migrated from sled's tree-based storage to redb's table-based storage
- New table definitions:
  - `DESKTOP_CACHE_TABLE`: Desktop file cache with mtime tracking
  - `NAME_INDEX_TABLE`: Fast app name lookup index
  - `FILE_LIST_TABLE`: Cached directory listings
  - `HISTORY_TABLE`: Launch history with usage counts
  - `PINNED_TABLE`: Pinned/favorited applications
- All tables use string keys with binary value storage for flexibility

### API Changes
- `DesktopCache::new()` now returns `Result<Self>` instead of `Self`
- Database operations now use explicit read/write transactions
- `HistoryCache::load()` now returns `Result<Self>` instead of `Self`
- Cache operations properly handle table creation and initialization

## Fixed
- Improved database consistency with ACID transactions
- Better handling of concurrent access patterns
- More reliable cache invalidation and refresh logic

## Notes
- This is a major version bump due to breaking changes (CLI flags and database migration)
- All functionality remains the same from a user perspective
- Configuration files and keybinds are fully compatible
- Desktop file cache rebuilds automatically on first run
- The old database file will not cause conflicts and can be deleted at your convenience

## Compatibility
- Rust 1.70+ stable required (unchanged)
- GNU/Linux and *BSD support (unchanged)
- Config file format unchanged
- **CLI flags changed**: Use hyphens instead of underscores (see CLI Flag Names section)

---

[1.1.0-riceknife]

### Added
- Desktop file caching system for significantly faster startup times
- File list caching to eliminate directory walking on subsequent launches
- Name index for instant app lookup by name
- `--clear_cache` flag to clear all cached data
- `--refresh_cache` flag to force refresh of desktop file list
- Batch history loading for improved database performance

### Changed
- Startup performance improved by 5-10x (from ~100ms to ~10-20ms after first run)
- Cache automatically refreshes every 5 minutes to pick up new applications
- History and pinned app data now loaded in a single database scan
- Locale detection now cached for the lifetime of the process

### Fixed
- Dmenu mode column output with `--accept-nth` flag now works correctly

---

[1.0.1-riceknife]

### Added
- Cargo install

### Fixed
- Fixed fullscreen image preview keybind in cclip mode - changed from Ctrl+I to Alt+I due to terminal control code conflict where Ctrl+I is interpreted as Tab
- Fixed keybind priority order to prevent custom keybinds from being overridden by hardcoded navigation keys

### Changed
- Switched from rust-overlay to naersk in flake.nix to eliminate manual cargoHash updates on version changes
- Temporarily disabled tag functionality until cclip has a new release for it soon im sure (code preserved with `#[allow(dead_code)]` for future re-enablement)

---

[1.0.0-riceknife]

This release brings dmenu to feature parity with the real thing (and then some), plus a bunch of quality-of-life improvements for ricests who want their launcher exactly how they like it it.

## Added

### Dmenu Extensions

Dmenu mode is now fully featured. You can pipe anything through it and get exactly the behavior you need:

- `--dmenu0`: null-separated input/output
- `--password[=char]`: mask input (default: *)
- `--index`: output index instead of text
- `--accept-nth`, `--match-nth`: column filtering for output/matching
- `--select`, `--select-index`: pre-select entries
- `--auto-select`: auto-select single matches
- `--prompt-only`: input-only mode (no list)
- `--only-match`: force list selection (no custom input)
- `--exit-if-empty`: exit on empty stdin
- `--hide-before-typing`: hide list until first keystroke
- Symlink detection: works when invoked as `dmenu`

### App Launcher

The launcher got smarter about what it shows and how you interact with it:

- `--list-executables-in-path`: include $PATH executables (finally launch your scripts)
- `--filter-desktop[=no]`: toggle OnlyShowIn/NotShowIn filtering (default: yes)
- `--match-mode <mode>`: fuzzy (default) or exact substring matching
- `--hide-before-typing`: hide list until first keystroke
- `confirm_first_launch`: optional confirmation for apps with no history
- Pin/favorite system: `Ctrl+Space` to toggle, persistent across sessions
- Configurable pin icon and color

### Customization

Remap everything. Seriously, everything:

- `keybinds.toml`: remap all keys (up, down, left, right, select, exit, pin, backspace, image_preview)
- Modifier support: Ctrl, Shift, Alt
- `title_panel_position`: top/middle/bottom placement
- `disable_mouse`: global mouse toggle
- Per-mode mouse disable (dmenu/cclip)
- Better error messages for invalid CLI flags

### Cclip Mode

- Fullscreen image viewer (press `i`)
- Dynamic title panel positioning (top/middle/bottom)

### Technical

Cleaned up the codebase for maintainability:

- `helpers.rs`: extracted common functions (launch_app, pin management, db handling)
- `keybinds.rs`: modular keybind system
- Input handler: configurable mouse disable
- Tokio: added `io-util` feature

## Changed

- Project renamed: `gyr` → `fsel`
- Binary: `gyr` → `fsel`
- Config: `~/.config/gyr/` → `~/.config/fsel/`
- Data: `~/.local/share/gyr/` → `~/.local/share/fsel/`
- Graphics: improved state management, eliminated race conditions
- Input: mouse events respect `disable_mouse` config

## Fixed

- Image stacking in cclip mode
- Race conditions in graphics display
- Flicker during image transitions
- Cursor position restoration after image display
- Mouse event handling when disabled

## Migration

```bash
mv ~/.config/gyr ~/.config/fsel
mv ~/.local/share/gyr ~/.local/share/fsel
```

Update WM configs to use `fsel`. Old config files shoulds work as-is.

---

[gyr 0.3.0-eggrind]

# gyr 0.3.0-eggrind

Fast TUI app launcher and fuzzy finder for GNU/Linux and *BSD

 "eggrind" 🥚
### Dmenu Mode
Full dmenu replacement with `--dmenu` flag. Reads stdin, outputs to stdout with fuzzy matching and content preview.

### Clipboard History Mode  
Browse clipboard history with `--cclip` flag. Integrates with cclip, shows image previews in Kitty/Sixel terminals.

```bash
gyr --cclip
```

### Enhanced Configuration
Separate `[dmenu]` and `[cclip]` config sections with mode-specific colors and layouts. Configuration inheritance system.

## Technical
- Modular UI system supporting multiple interaction modes
- Advanced terminal graphics integration with Kitty/Sixel protocols  
- Configuration inheritance system (cclip → dmenu → regular mode)
- Zero-copy string handling for large clipboard content
- Non-blocking image preview generation

### Dependencies
Optional: cclip (clipboard manager), chafa (image previews)

## Links
- [**Changelog**](https://github.com/Mjoyufull/gyr/blob/main/CHANGELOG.md)

---

[0.2.8-bolttree]

## Added

* Full mouse support (hover, click, scroll wheel)
* Direct launch mode with `-p/--program` flag  
* Pre-filled search mode with `-ss` flag

## Improved

* Mouse cursor positioning and scroll behavior
* Selection persistence during rapid interactions
* Command-line argument validation

## Fixed

* Input field border gaps
* Selection disappearing during fast scrolling

---

[0.2.7] tree-leech

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

---

0.2.6

[0.2.6] - 2025-09-08
Added

    Enhanced color support with multiple formats:
        Hex colors: #ff0000, #f00
        RGB colors: rgb(255,0,0), (255,0,0)
        8-bit terminal colors: 196 (0-255)
        Additional named colors: gray/grey, darkgray/darkgrey, reset
    Color examples file with popular themes (Gruvbox, Nord, Dracula, One Dark)
    Improved error messages for invalid color formats

---

0.2.5

[0.2.5] - 2025-09-08
Removed

    Cached entries system - removed due to reliability issues with detecting new applications
    --clear-cache CLI option - no longer needed
    Cache configuration options from config.toml (enable_cache, cache_ttl_seconds)
    Unused safe-regex dependency

Changed

    Migrated from termion to crossterm for professional terminal handling
    Optimized application scanning for better performance without caching
    Improved terminal cleanup - no more blank lines or artifacts on exit
    Enhanced input handling with crossterm KeyEvent system

---

0.2.1

## [0.2.1] - 2025-09-07

### Added

* Multiple launch backends: `--systemd-run`, `--uwsm`, and `--no-exec` options
* Cached entries system with 36% performance improvement
* Extensive UI customization options (15+ configuration settings)
* Nix flake for universal installation across distributions
* Enhanced configuration system with validation

### Changed

* Forked from original sourcehut repository (git.sr.ht/~nkeor/gyr) to GitHub
* Updated repository URLs and documentation for GitHub hosting
* Improved lock file management with automatic cleanup
* Enhanced error handling and graceful fallbacks
* Unpinned serde and serde_derive
