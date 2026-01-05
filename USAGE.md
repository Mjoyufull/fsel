# fsel Usage Guide

Quick reference for common use cases.

## App Launcher

### Basic Usage
```sh
# Launch fsel
fsel

# Pin your favorite apps (Ctrl+Space in TUI)
# Pinned apps always appear first with 📌 icon

# Frequently used apps automatically rise to the top (Frecency ranking)

# Pre-fill search (works with app launcher, dmenu, and cclip modes)
# Note: -ss must be the LAST option
fsel -ss firefox

# Direct launch (no UI)
fsel -p firefox

# Show CLI tools from $PATH
fsel --list-executables-in-path

# Hide list until typing
fsel --hide-before-typing

# Exact matching only
fsel --match-mode=exact

# Cache management
fsel --clear-cache      # Clear all caches (full rebuild)
fsel --refresh-cache    # Refresh file list (pick up new apps)
fsel --clear-history    # Clear launch history

# Replace existing instances (fsel and cclip modes only)
fsel -r                 # Replace running fsel instance (ensures previous session exits)
fsel --cclip -r         # Replace running cclip instance
# Not supported in --dmenu mode
```

### Launch Methods
```sh
# Default (direct execution)
fsel

# Through Sway
fsel  # Auto-detected if $SWAYSOCK is set
fsel -s  # Disable Sway integration (short form)
fsel --nosway  # Disable Sway integration (long form)

# Through systemd
fsel --systemd-run
fsel --systemd-run --detach   # Fully detached using systemd scope

# Through uwsm
fsel --uwsm
fsel --uwsm --detach          # Fully detached via uwsm

# Detach from terminal (prevents apps from being killed when terminal closes)
# Useful for apps like Discord or Steam; works standalone or with --systemd-run/--uwsm
fsel --detach

# Print command instead of running
fsel --no-exec
```

## Dmenu Mode

### Basic Dmenu
```sh
# Simple selection
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# From file
cat options.txt | fsel --dmenu

# From command output
git branch | fsel --dmenu

# Null-separated input
find . -print0 | fsel --dmenu0
```

### Column Operations
```sh
# Display only column 2
ps aux | fsel --dmenu --with-nth=2

# Display column 2, output column 1
ps aux | fsel --dmenu --with-nth=2 --accept-nth=1

# Match against column 3, display column 1
printf "A\tB\tC\nD\tE\tF" | fsel --dmenu --with-nth=1 --match-nth=3

# Custom delimiter
echo "A:B:C" | fsel --dmenu --delimiter=":"
```

### Special Modes
```sh
# Password input
echo -e "pass1\npass2" | fsel --dmenu --password

# Custom password character
echo -e "pass1\npass2" | fsel --dmenu --password=•

# Output index instead of text
echo -e "A\nB\nC" | fsel --dmenu --index

# Prompt-only (no list)
fsel --dmenu --prompt-only

# Force selection from list
echo -e "A\nB\nC" | fsel --dmenu --only-match
```

### Pre-selection
```sh
# Pre-fill search query
echo -e "firefox\nchrome\nfirefox-dev" | fsel --dmenu -ss fire

# Pre-select by string
git branch | fsel --dmenu --select main

# Pre-select by index
echo -e "A\nB\nC" | fsel --dmenu --select-index=1

# Auto-select when one match
echo -e "Option 1\nOption 2" | fsel --dmenu --auto-select
```

### Matching
```sh
# Exact matching
echo -e "test\ntesting\ntest123" | fsel --dmenu --match-mode=exact

# Exit if empty input
cat empty.txt | fsel --dmenu --exit-if-empty
```

## Clipboard Mode

### Basic Usage
```sh
# Browse clipboard history
fsel --cclip

# Pre-fill search to find specific content
fsel --cclip -ss image

# With image previews (requires Kitty/Sixel terminal + chafa)
fsel --cclip  # Images show automatically if supported
```

### Tag Management
```sh
# Filter clipboard items by tag
fsel --cclip --tag prompt
fsel --cclip --tag code

# List all available tags
fsel --cclip --tag list

# List items with specific tag
fsel --cclip --tag list prompt

# List items with tag (verbose shows details)
fsel --cclip --tag list prompt -vv

# Clear tag metadata from fsel database
# Note: This only clears fsel's tag metadata (colors, emojis)
# To clear tags from cclip entries, use: cclip tag -d <ID>
fsel --cclip --tag clear

# Wipe ALL tags from cclip entries (requires cclip 3.2+)
# This removes the tag text (e.g., "[tag]") from the actual clipboard items
fsel --cclip --tag wipe

# Show tag color names in item display
fsel --cclip --cclip-show-tag-color-names
```

### Keybindings in cclip mode
- `Enter` - Copy selection to clipboard
- `Alt+i` - Display image fullscreen (bypass TUI)
- `Esc` - Exit without copying
- Arrow keys - Navigate
- Type to filter

**Note:** Tag creation and management requires cclip with tag support. Tags appear as `[tagname]` prefixes in the clipboard item list.

## Scripting Examples

### Process Killer
```sh
ps aux | fsel --dmenu --with-nth=2,11 --accept-nth=2 | xargs kill
```

### Git Branch Switcher
```sh
git branch | fsel --dmenu --select main | xargs git checkout
```

### SSH Connection Picker
```sh
grep "^Host " ~/.ssh/config | fsel --dmenu --with-nth=2 | xargs ssh
```

### Window Switcher (Sway)
```sh
swaymsg -t get_tree | \
  jq -r '..|select(.type=="con" and .name!=null)|.name' | \
  fsel --dmenu | \
  xargs -I {} swaymsg '[title="{}"] focus'
```

## Tips & Tricks

### Terminal Recommendations

**Best:** Kitty - Full inline image support, best performance
```sh
# Install Kitty
sudo pacman -S kitty  # Arch
sudo apt install kitty  # Debian/Ubuntu
```

**Also Great:** Foot, WezTerm, any Sixel-capable terminal
- Sixel now fully supported for inline previews

### Drop-in dmenu Replacement
```sh
# Create symlink
ln -s $(which fsel) ~/.local/bin/dmenu

# Now scripts using dmenu will use fsel
rofi-script.sh  # Works automatically
```

### Otter-Launcher Integration

Combine fsel with [otter-launcher](https://github.com/kuokuo123/otter-launcher) for a powerful dual-mode setup:

**Setup:**
1. Typing just an app name → Opens fsel with pre-filled search
2. Typing `app <name>` → Instantly launches app without TUI

```toml
# ~/.config/otter-launcher/config.toml
[general]
default_module = "search"
empty_module = "search"
exec_cmd = "sh -c"

# Mode 1: Search mode (default)
[[modules]]
description = "search apps with fsel"
prefix = "search"
cmd = "fsel -vv -d -r -ss \"{}\""
with_argument = true

# Mode 2: Instant launch
[[modules]]
description = "launch apps instantly"
prefix = "app"
cmd = "fsel -vv -d -r -p \"{}\""
with_argument = true
```

**Usage:**
```sh
# In otter-launcher:
firefox          # Opens fsel with "firefox" pre-searched
app firefox      # Instantly launches Firefox (no TUI)
app code         # Instantly launches VS Code
```

**Optional: Add launch method flags if needed:**
```toml
# With uwsm (requires uwsm installed)
cmd = "fsel --uwsm -vv -d -r -p \"{}\""

# With systemd-run (requires systemd)
cmd = "fsel --systemd-run -vv -d -r -p \"{}\""

# With Sway (auto-detected if $SWAYSOCK is set)
cmd = "fsel -vv -r -d -p \"{}\""

```

**Warning:** Keep `unbind_proc` disabled for Fsel modules whilst using -d, and you need to do -d for apps to launch without unbind_proc and you need unbind_proc to launch apps without -d,. If it is set to `true`, otter-launcher returns to its own prompt while `fsel` is still running and raw terminal input will leak (escape sequences like `[B`). Use a dedicated terminal wrapper if you need asynchronous launch behavior.
```

### Performance with Large Lists
```sh
# Disable desktop filtering for speed
fsel --filter-desktop=no

# Use exact matching for faster results
fsel --match-mode=exact

# Limit executables from PATH
# (edit config to disable list_executables_in_path)
```

### Debug/Test Mode

Enable detailed debug logging with `-T` or `--test`:

```sh
# Enable debug mode
fsel -T

# Debug logs are written to ~/.config/fsel/logs/
# Filename format: fsel-debug-YYYYMMDD-HHMMSS-pidXXXXX.log
```

**What gets logged:**
- Startup configuration (prefix_depth, match_mode, etc.)
- Total apps loaded and frecency entries
- Query changes (each character typed, backspace)
- Search snapshots with full scoring breakdown:
  - Tier classification (Pinned App Name Exact, Normal Fuzzy Match, etc.)
  - Bucket score, matcher score, frecency boost
  - Top 50 matches with complete breakdown
  - Filter timing
- Selection changes (which app is selected, scroll position)
- Launch events (app name, command, scoring details)
- Session summary (total duration)

**Use cases:**
- Debug why certain apps rank higher/lower than expected
- Understand search ranking algorithm behavior
- Analyze performance (filter timing)
- Track user interaction patterns
- Verify prefix_depth and other configuration settings

**Example log output:**
```
=== FSEL DEBUG SESSION STARTED ===
Timestamp: 2025-01-15 14:30:45.123
PID: 12345
Version: 2.4.0-seedclay
Log file: /home/user/.config/fsel/logs/fsel-debug-20250115-143045-pid12345.log

[STARTUP] Configuration:
  Prefix depth: 3
  Match mode: Fuzzy
  ...

[QUERY] User typed 'f': "" -> "f"
[SEARCH] Query: "f" (len: 1, prefix_depth: 3)
[SEARCH] Filter time: 2ms
[SEARCH] Total matches: 45 (showing top 50)
  [  1] Firefox (Score: 90000050)
       ├── Tier: Normal App Name Exact
       ├── Bucket Score: 90000000
       ├── Matcher Score: 50 (base: 0, 100x multiplier)
       └── Frecency: 0.500 (raw: 0.500, boost: +5)
...
```

### Prefix Depth Configuration

The `prefix_depth` setting controls how many characters must be typed before prefix matching gets priority over fuzzy matching:

```sh
# Set prefix depth via CLI
fsel --prefix-depth 5

# Or in config.toml
prefix_depth = 5
```

**How it works:**
- When query length ≤ prefix_depth: Prefix matches (word-start, exact, etc.) get higher priority
- When query length > prefix_depth: All matches use fuzzy scoring equally
- Default: 3 characters

**Example:**
- With `prefix_depth = 3`:
  - Typing "fi" (2 chars): Prefix matches prioritized
  - Typing "fire" (4 chars): Fuzzy matching takes over
- With `prefix_depth = 5`:
  - Typing "fire" (4 chars): Still uses prefix priority
  - Typing "firef" (5 chars): Prefix priority
  - Typing "firefo" (6 chars): Fuzzy matching

### Debugging
```sh
# Quick overview grouped by mode/flags
fsel -h

# Full tree-style reference covering every option
fsel -H

# Show verbose output
fsel -vvv
```

## Configuration

### Config File Structure

Configuration is stored in `~/.config/fsel/config.toml`. **Field placement is critical** - putting options in the wrong section will cause crashes.

#### Correct Structure:
```toml
# Root level - UI/Color options go here
highlight_color = "LightBlue"
main_border_color = "White"
pin_color = "Orange"
terminal_launcher = "kitty -e"

# App launcher specific options
[app_launcher]
filter_desktop = true
list_executables_in_path = false

# Dmenu mode overrides
[dmenu]
delimiter = " "
show_line_numbers = true

# Clipboard mode overrides  
[cclip]
image_preview = true
```

#### Common Mistakes (Will Crash):
```toml
# WRONG - Color options in app_launcher section
[app_launcher]
main_border_color = "White"  # This will crash!
filter_desktop = true

# WRONG - App launcher options at root level
filter_desktop = true  # This should be in [app_launcher]
```

### Error Messages

If you see errors like:
```
Error reading config file: unknown field `pin_color`, expected one of `filter_desktop`, `list_executables_in_path`...
```

This means you've placed a **color/UI option inside the [app_launcher] section**. Move it to the root level.

### Field Reference

**Root Level Fields:**
- Colors: `highlight_color`, `main_border_color`, `apps_border_color`, `input_border_color`, `main_text_color`, `apps_text_color`, `input_text_color`, `header_title_color`, `pin_color`
- UI: `cursor`, `rounded_borders`, `hard_stop`, `fancy_mode`, `pin_icon`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- General: `terminal_launcher`, `keybinds`

**[app_launcher] Section (strict validation):**
- `filter_desktop`, `list_executables_in_path`, `hide_before_typing`, `match_mode`, `confirm_first_launch`

**[dmenu] Section:**
- Colors: `highlight_color`, `main_border_color`, `items_border_color`, `input_border_color`, `main_text_color`, `items_text_color`, `input_text_color`, `header_title_color`
- UI: `cursor`, `hard_stop`, `rounded_borders`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- Parsing: `delimiter`, `show_line_numbers`, `wrap_long_lines`
- Behavior: `password_character`, `exit_if_empty`

**[cclip] Section:**
- Colors: `highlight_color`, `main_border_color`, `items_border_color`, `input_border_color`, `main_text_color`, `items_text_color`, `input_text_color`, `header_title_color`
- UI: `cursor`, `hard_stop`, `rounded_borders`, `disable_mouse`
- Layout: `title_panel_height_percent`, `input_panel_height`, `title_panel_position`
- Display: `show_line_numbers`, `wrap_long_lines`
- Images: `image_preview`, `hide_inline_image_message`
