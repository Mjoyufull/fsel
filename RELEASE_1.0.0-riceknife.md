# fsel 1.0.0-riceknife - 2025-10-08

This release brings dmenu to feature parity with the real thing (and then some), plus a bunch of quality-of-life improvements for ricers who want their launcher exactly how they want it.

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
- Dynamic panel positioning (top/middle/bottom)

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

Update WM configs to use `fsel`. Old config files work as-is.

**Full Changelog**: https://github.com/Mjoyufull/fsel/compare/0.3.0-eggrind...1.0.0-riceknife
