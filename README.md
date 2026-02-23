<div align="center">

  ![Logo](./assets/fsel.png)

*(fast select)*

  [![License](https://img.shields.io/crates/l/fsel?style=flat-square)](https://github.com/Mjoyufull/fsel/blob/main/LICENSE)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  Fast TUI app launcher and fuzzy finder for GNU/Linux and \*BSD

  <img width="860" height="1019" alt="Screenshot_20251006-032156" src="https://github.com/user-attachments/assets/777bd0a4-eb52-4014-837b-d361ab57cfff" />



</div>

## Table of Contents

- [Quickstart](#quickstart)
- [Install](#install)
- [Usage](#usage)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Contributing](#contributing)
- [License](#license)

**More Info:** [Detailed Usage Guide](./USAGE.md)

## Requirements

**Build Requirements:**
- Rust 1.89+ **stable** (NOT nightly)
  - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify: `rustc --version` (should show stable, not nightly)
  - If using nightly: `rustup default stable`
- Cargo (comes with Rust)

**Runtime Requirements:**
- GNU/Linux or *BSD
- Terminal emulator

**Optional:**
- [`cclip`](https://github.com/heather7283/cclip) - for clipboard history mode
- Kitty, Sixel-, or Halfblocks-capable terminal - for native inline image previews in cclip mode (see [ratatui-image](https://github.com/benjajaja/ratatui-image))

**Note:** Image previews in cclip mode use built-in [ratatui-image](https://github.com/benjajaja/ratatui-image) (no external viewer). Versions before 3.1.0 required `chafa` for image previews; 3.1.0 and later do not.

## Quickstart

Get up and running in 30 seconds:

```sh
# Install with Nix (recommended)
nix run github:Mjoyufull/fsel

# Or the Aur
$ yay -S fsel-git
 # or
$ paru -S fsel-git

# Or build from source
git clone https://github.com/Mjoyufull/fsel && cd fsel
cargo build --release
sudo cp target/release/fsel /usr/local/bin/

# Launch it
fsel

# Use as dmenu replacement
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# Browse clipboard history (requires cclip)
fsel --cclip
```

That's it. Type to search, arrow keys to navigate, Enter to launch.

## Install

#### Option 1: Nix Flake (Recommended)

* Build and run with Nix flakes:
    ```sh
    $ nix run github:Mjoyufull/fsel
    ```

* Add to your profile:
    ```sh
    $ nix profile add github:Mjoyufull/fsel
    ```

* Add to your `flake.nix` inputs:
    ```nix
    {
      inputs.fsel.url = "github:Mjoyufull/fsel";
      # ... rest of your flake
    }
    ```

#### Option 2: Cargo

* Install from [crates.io](https://crates.io/crates/fsel):
    ```sh
    $ cargo install fsel@3.1.0-kiwicrab
    ```
* To update later:
    ```sh
    $ cargo install fsel@3.1.0-kiwicrab --force
    ```
* Or install latest version (check [releases](https://github.com/Mjoyufull/fsel/releases)):
    ```sh
    $ cargo search fsel  # See available versions
    $ cargo install fsel@<version>
    ```

#### Option 3: AUR (Arch Linux)

* Install the git version with your favorite AUR helper:
    ```sh
    $ yay -S fsel-git
    # or
    $ paru -S fsel-git
    ```
* Or manually:
    ```sh
    $ git clone https://aur.archlinux.org/fsel-git.git
    $ cd fsel-git
    $ makepkg -si
    ```

#### Option 4: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started) stable
* Build:
    ```sh
    $ git clone https://github.com/Mjoyufull/fsel && cd fsel
    $ cargo build --release
    ```
* Copy `target/release/fsel` to somewhere in your `$PATH`
* (Optional) Create a dmenu symlink for drop-in compatibility:
    ```sh
    $ ./create_dmenu_symlink.sh
    ```
    Or manually: `ln -s $(which fsel) ~/.local/bin/dmenu`

### optional dependencies

* **uwsm** - Universal Wayland Session Manager (for `--uwsm` flag)
* **systemd** - For `--systemd-run` flag (usually pre-installed)
* [**cclip**](https://github.com/heather7283/cclip) - Clipboard manager (for `--cclip` mode)
* **Kitty, Foot, WezTerm, or other Sixel/Kitty/Halfblocks-capable terminal** - For native inline image previews in cclip mode (powered by [ratatui-image](https://github.com/benjajaja/ratatui-image); no chafa needed in 3.1.0+)
* [**otter-launcher**](https://github.com/kuokuo123/otter-launcher) - Pairs nicely with fsel for a complete launcher setup

## Usage

### Interactive Mode

Run `fsel` from a terminal to open the interactive TUI launcher.

#### Features

- **Advanced Search Ranking**: 12-tier prioritization system ensures you find what you're looking for. Pinned apps, exact matches, and prefix matches are intelligently ranked with frecency-based scoring (zoxide-style) that learns from your usage patterns
- **Smart Matching**: Searches names, descriptions, keywords, and categories
- **Smart Ranking**: Apps ranked by frequency and recent usage with time-bucketed multipliers
- **Desktop Filtering**: Respects `OnlyShowIn`/`NotShowIn` fields
- **PATH Executables**: Optionally show CLI tools from `$PATH`
- **Match Modes**: Fuzzy (default) or exact matching
- **Pin/Favorite Apps**: Press Ctrl-Space to pin apps - they'll always appear first (marked with 📌)
- **Custom Keybinds**: All keyboard shortcuts are configurable
#### Navigation

**Keyboard:**
- Type to search/filter applications
- `↑`/`↓` or `Ctrl-P`/`Ctrl-N` to navigate up/down
- `←`/`→` to jump to top/bottom of list
- `Enter` or `Ctrl-Y` to launch selected application
- `Ctrl-Space` to pin/favorite selected app (pinned apps appear first)
- `Esc` or `Ctrl-Q` to exit
- `Backspace` to remove characters from search

**Mouse:**
- Hover over applications to select them
- Click on an application to launch it
- Scroll wheel to scroll through the application list
- All mouse interactions work alongside keyboard navigation

### Direct Launch Mode

Launch applications directly from the command line without opening the TUI:

```sh
# Launch Firefox directly
fsel -p firefox

# Launch first match for "terminal"
fsel -p terminal

# Works with partial names
fsel -p fire  # Finds Firefox

# Combine with launch options
fsel --uwsm -p discord
fsel --systemd-run -vv -p code
```

###  TTY mode
 Launch terminal applications inline in the current terminal session.
 In TTY mode fsel replaces itself with the selected terminal program (exec),

 so the launched app takes over the current terminal (useful for htop, vim, etc.).

Enable with -t or --tty, or set `terminal_launcher = "tty"` in config.
fsel -t
fsel --tty

### Pre-filled Search Mode

Open the TUI with a pre-filled search string. Works with app launcher, dmenu, and cclip modes:

```sh
# Open TUI with "firefox" already searched
fsel -ss firefox

# Multi-word search terms work
fsel -ss web browser

# Combine with other options (must be last)
fsel --uwsm -vv -r -ss text editor

# Works with dmenu mode
echo -e "option1\noption2\noption3" | fsel --dmenu -ss opt

# Works with cclip mode
fsel --cclip -ss image
```

### Dmenu Mode

Fsel includes a full dmenu replacement mode that reads from stdin and outputs selections to stdout:

```sh
# Basic dmenu replacement
echo -e "Option 1\nOption 2\nOption 3" | fsel --dmenu

# Display only specific columns (like cut)
ps aux | fsel --dmenu --with-nth 2,11  # Show only PID and command

# Use custom delimiter
echo "foo:bar:baz" | fsel --dmenu --delimiter ":"

# Pipe from any command
ls -la | fsel --dmenu
find . -name "*.rs" | fsel --dmenu
git log --oneline | fsel --dmenu
```

#### Dmenu Features

**Column Operations:**
- `--with-nth` - Display specific columns
- `--accept-nth` - Output specific columns
- `--match-nth` - Match against specific columns
- `--delimiter` - Custom column separator

**Input/Output:**
- `--password` - Mask input for passwords
- `--index` - Output index instead of text
- `--dmenu0` - Null-separated input
- `--only-match` - Force selection from list

**Selection:**
- `--select` - Pre-select by string
- `--select-index` - Pre-select by index
- `--auto-select` - Auto-select single match

**Modes:**
- `--prompt-only` - Text input without list
- `--match-mode=exact` - Exact matching only
- Drop-in dmenu replacement (symlink as `dmenu`)

### Clipboard History Mode
- must have cclip
<img width="853" height="605" alt="image" src="https://github.com/user-attachments/assets/0bf71952-f09a-4ce2-8807-bca1003c8daf" />

Browse and select from your clipboard history with image previews:

```sh
# Browse clipboard history with cclip integration
fsel --cclip

# Filter by tag
fsel --cclip --tag prompt

# List all tags
fsel --cclip --tag list

# List items with specific tag (verbose shows details)
fsel --cclip --tag list prompt -vv

# Clear tag metadata from fsel database
fsel --cclip --tag clear

# Show tag color names in display
fsel --cclip --cclip-show-tag-color-names
```

#### Clipboard Features

- **Native image previews**: Inline and fullscreen (Alt+i) image rendering via [ratatui-image](https://github.com/benjajaja/ratatui-image) — automatic protocol detection (Kitty, Sixel, Halfblocks); no external viewer required
- **Content Preview**: Full text preview panel
- **Fuzzy Search**: Filter clipboard history
- **Smart Copy**: Auto-copies selection to clipboard
- **Entry Deletion**: Press Alt+Delete to delete the selected clipboard entry; the selection remains at the same physical position (the next item becomes selected).
- **Tag System**: Organize clipboard items with tags (requires cclip with tag support)
- Requires [cclip](https://github.com/heather7283/cclip)

### Quick Examples

**Dmenu mode:**
```sh
# Simple selection
echo -e "Edit\nView\nDelete" | fsel --dmenu

# Password input
echo -e "pass1\npass2" | fsel --dmenu --password

# Process killer
ps aux | fsel --dmenu --with-nth 2,11 --accept-nth 2 | xargs kill

# Git branch switcher
git branch | fsel --dmenu --select main | xargs git checkout

# Drop-in dmenu replacement
ln -s $(which fsel) ~/.local/bin/dmenu
```

**Scripting:**
```sh
# SSH picker
grep "^Host " ~/.ssh/config | fsel --dmenu --with-nth 2 | xargs ssh

# File opener
find . -type f | fsel --dmenu | xargs xdg-open

# Window switcher (Sway)
swaymsg -t get_tree | jq -r '..|select(.name)|.name' | fsel --dmenu
```

See [USAGE.md](./USAGE.md) for more examples and advanced usage.

### Command Line Options

```
Usage:
  fsel [OPTIONS]

├─ Core Modes
│  ├─ -p, --program <NAME>         Launch program directly (bypass TUI)
│  ├─ --cclip                      Clipboard history mode
│  └─ --dmenu                      Dmenu-compatible mode
│
├─ Control Flags
│  ├─ -r, --replace                Replace running fsel/cclip instance
│  ├─ -d, --detach                 Detach launched applications (GUI-safe)
│  ├─ -v, --verbose                Increase verbosity (repeatable)
│  ├─ --systemd-run                Launch via systemd-run --user --scope
│  ├─ --uwsm                       Launch via uwsm app
│  ├─ --no-exec                    Print selection to stdout instead of launching
│  └─ -ss <SEARCH>                 Pre-fill TUI search (must be last option)
│
├─ Quick Extras
│  ├─ --clear-history              Clear launch history
│  ├─ --clear-cache                Clear app cache
│  ├─ --refresh-cache              Rescan desktop entries
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn (default: yes)
│  ├─ --hide-before-typing         Hide list until first character typed
│  ├─ --list-executables-in-path   Include executables from $PATH
│  ├─ --match-mode <MODE>          fuzzy | exact (default: fuzzy)
│  ├─ --prefix-depth <N>           Character depth for prefix matching priority (default: 3)
│  └─ -T, --test                   Enable debug/test mode with detailed logging
│
├─ Dmenu Mode Options
│  ├─ --dmenu0                     Like --dmenu but null-separated input
│  ├─ --password[=CHAR]            Password mode (mask input)
│  ├─ --index                      Output index instead of text
│  ├─ --with-nth <COLS>            Display only specific columns (e.g. 1,3)
│  ├─ --accept-nth <COLS>          Output only specified columns
│  ├─ --match-nth <COLS>           Match only specified columns
│  ├─ --delimiter <CHAR>           Column delimiter (default: space)
│  ├─ --only-match                 Disallow custom input
│  ├─ --exit-if-empty              Exit if stdin is empty
│  ├─ --select <STRING>            Preselect matching entry
│  ├─ --select-index <N>           Preselect entry by index
│  ├─ --auto-select                Auto-select when one match remains
│  └─ --prompt-only                Input-only mode (no list)
│
├─ Clipboard Mode Options
│  ├─ --cclip                      Clipboard history viewer with previews
│  ├─ --tag <NAME>                 Filter clipboard items by tag
│  ├─ --tag list                   List all tags
│  ├─ --tag list <NAME>            List items with specific tag
│  ├─ --tag clear                  Clear tag metadata from fsel database
│  ├─ --tag wipe                   Wipe ALL tags from cclip entries (cclip 3.2+)
│  └─ --cclip-show-tag-color-names Show tag color names in display
│
└─ General
   ├─ -h                           Show short help
   ├─ -H, --help                   Show detailed help
   ├─ -T, --test                   Enable debug/test mode (logs to ~/.config/fsel/logs/)
   ├─ -V, --version                Show version info
   └─ -t, --tty                    Launch in current terminal (TTY mode)

```

#### Launch Methods

- **Default**: Standard execution
- **Sway Integration**: Automatically enabled when `$SWAYSOCK` is set. Uses `swaymsg exec` to launch applications in the current workspace (requires Sway)
- **systemd-run**: `--systemd-run` launches applications in isolated systemd user scopes (requires systemd)
- **uwsm**: `--uwsm` launches applications through the Universal Wayland Session Manager (requires uwsm to be installed)

#### Verbosity Levels

- `-v`: Show application execution details
- `-vv`: Show application paths and additional metadata
- `-vvv`: Show debug information including usage statistics

#### Debug/Test Mode

Use `-T` or `--test` to enable detailed debug logging:

```sh
# Enable debug mode
fsel -T

# Debug logs are written to ~/.config/fsel/logs/
# Filename format: fsel-debug-YYYYMMDD-HHMMSS-pidXXXXX.log
```

Debug mode logs:
- Startup configuration and loaded data
- Query changes (each character typed)
- Search snapshots with full scoring breakdown
- Selection changes
- Launch events
- Session timing and statistics

Useful for debugging search ranking, understanding why apps appear in a certain order, or analyzing performance.

## Configuration

Config file: `~/.config/fsel/config.toml`

### Basic Setup

```toml
# Colors
highlight_color = "LightBlue"
cursor = "█"

# App launcher
terminal_launcher = "alacritty -e"

[app_launcher]
filter_desktop = true              # Filter apps by desktop environment
list_executables_in_path = false   # Show CLI tools from $PATH
hide_before_typing = false         # Hide list until you start typing
match_mode = "fuzzy"               # "fuzzy" or "exact"
confirm_first_launch = false       # Confirm before launching new apps with -p
prefix_depth = 3                   # Character depth for prefix matching priority

# Pin/favorite settings
pin_color = "rgb(255,165,0)"       # Color for pin icon (orange)
pin_icon = "📌"                     # Icon for pinned apps
```

### Advanced Options

```toml
# UI customization
rounded_borders = true
main_border_color = "White"
apps_border_color = "White"
input_border_color = "White"

# Layout (percentages)
title_panel_height_percent = 30    # Top panel height (10-70%)
input_panel_height = 3             # Input panel height in lines
title_panel_position = "top"       # "top", "middle", or "bottom"

# Dmenu mode
[dmenu]
password_character = "*"
exit_if_empty = false

# Clipboard mode
[cclip]
image_preview = true
hide_inline_image_message = false

# Custom keybinds (optional)
[keybinds]
up = ["up", { key = "k", modifiers = "ctrl" }]
down = ["down", { key = "j", modifiers = "ctrl" }]
select = ["enter"]
exit = ["esc", { key = "q", modifiers = "ctrl" }]
pin = [{ key = "space", modifiers = "ctrl" }]
```

See [config.toml](./config.toml) and [keybinds.toml](./keybinds.toml) for all options with detailed comments.

### Window Manager Integration

**Sway/i3:**
```sh
# ~/.config/sway/config
set $menu alacritty --title launcher -e fsel
bindsym $mod+d exec $menu
for_window [title="^launcher$"] floating enable, resize set width 500 height 430, border none

# Clipboard history
bindsym $mod+v exec 'alacritty --title clipboard -e fsel --cclip'
```

**Hyprland:**
```sh
# ~/.config/hypr/hyprland.conf
bind = $mod, D, exec, alacritty --title launcher -e fsel
windowrule = match:class ^launcher$, float 1
windowrule = match:class ^launcher$, size 500 430
windowrule = match:class ^launcher$, center 1
```

**dwm/bspwm/any WM:**
```sh
# Use dmenu mode
bindsym $mod+d exec "fsel --dmenu | xargs swaymsg exec --"
```

## Contributing

Contributions are welcome! Whether you're reporting bugs, suggesting features, or submitting code, we appreciate your help making fsel better.

### How to Contribute

1. **Bug Reports & Feature Requests**: Open an issue on [GitHub Issues](https://github.com/Mjoyufull/fsel/issues)
2. **Pull Requests**: Fork the repo, create a feature branch, and submit a PR
3. **Code Style**: Run `cargo fmt` and `cargo clippy` before submitting
4. **Testing**: Ensure `cargo test` and `cargo build --release` pass

### Development Workflow

See [CONTRIBUTING.md](./CONTRIBUTING.md) for detailed guidelines on:
- Branch naming conventions
- Commit message format
- Pull request process
- Code review standards
- Release procedures

All contributors are valued and appreciated. Your name will be added to the contributors list, and significant contributions will be highlighted in release notes.

Thank you for helping improve fsel!

## Philosophy

fsel is a **unified TUI workflow tool** built for terminal-centric setups. It combines app launching, dmenu functionality, and clipboard history into one scriptable interface with consistent keybinds and theming.

**This means:**
- It's built for my workflow first, but PRs for bug fixes and useful features are welcome as long as they fit in scope.
- Older versions and the original gyr exist if you want something more minimal.
---

## Troubleshooting

**Apps not showing up?**
- Check `$XDG_DATA_DIRS` includes `/usr/share/applications`
- Try `--filter-desktop=no` to disable desktop filtering
- Use `-vvv` for debug info

**Mouse not working?**
- Check your terminal supports mouse input
- Try `disable_mouse = false` in config

**Images not showing in cclip mode?**
- Use a Kitty-, Sixel-, or Halfblocks-capable terminal (e.g. Kitty, Foot, WezTerm). Image preview uses built-in [ratatui-image](https://github.com/benjajaja/ratatui-image); no chafa or other external viewer is needed (3.1.0+).
- Check `image_preview = true` in config
- Images render inside the content panel; press Alt+i for fullscreen preview

**Fuzzy matching too loose?**
- Try `--match-mode=exact` for stricter matching
- Or set `match_mode = "exact"` in config

**Terminal apps not launching?**
- Set `terminal_launcher` in config
- Example: `terminal_launcher = "kitty -e"`

## Credits

Fork of [gyr](https://git.sr.ht/~nkeor/gyr) by Namkhai B.

## License

[BSD 2-Clause](./LICENSE) (c) 2020-2022 Namkhai B., Mjoyufull
