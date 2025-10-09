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
- Rust 1.70+ **stable** (NOT nightly)
  - Install: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
  - Verify: `rustc --version` (should show stable, not nightly)
  - If using nightly: `rustup default stable`
- Cargo (comes with Rust)

**Runtime Requirements:**
- GNU/Linux or *BSD
- Terminal emulator

**Optional:**
- `cclip` - for clipboard history mode
- `chafa` - for image previews in cclip mode
- Kitty or Sixel-capable terminal - for best image support

## Quickstart

Get up and running in 30 seconds:

```sh
# Install with Nix (recommended)
nix run github:Mjoyufull/fsel

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

#### Option 2: Curl Install (Recommended for most users)

* Install directly from GitHub:
    ```sh
    $ curl -sSL https://raw.githubusercontent.com/Mjoyufull/fsel/main/install.sh | bash
    ```
* To update later, run the same command

#### Option 3: Build from source

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
* **chafa** - Terminal image viewer (for image previews in cclip mode)
* **Kitty terminal** - Recommended for best inline image support (Sixel terminals also supported)
* [**otter-launcher**](https://github.com/kuokuo123/otter-launcher) - Pairs nicely with fsel for a complete launcher setup

## Usage

### Interactive Mode

Run `fsel` from a terminal to open the interactive TUI launcher.

#### Features

- **Smart Matching**: Searches names, descriptions, keywords, and categories
- **Usage History**: Frequently used apps appear higher in results
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
```

#### Clipboard Features

- **Image Previews**: Inline rendering (Kitty/Sixel terminals)
- **Content Preview**: Full text preview panel
- **Fuzzy Search**: Filter clipboard history
- **Smart Copy**: Auto-copies selection to clipboard
- **Tagging**: Organize items with tags (Ctrl+T), filter by tag
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
Usage: fsel [options]

App Launcher Options:
  -s, --nosway                  Disable Sway integration
  -c, --config <config>         Specify a config file
  -r, --replace                 Replace existing fsel instances
      --clear_history           Clear launch history
  -p, --program [name]          Launch program directly (optional, min 2 chars)
  -ss <search>                  Pre-fill search in TUI (must be last option)
  -v, --verbose                 Increase verbosity level (multiple)
      --no-exec                 Print selected application to stdout instead of launching
      --systemd-run             Launch applications using systemd-run --user --scope
      --uwsm                    Launch applications using uwsm app
      --filter-desktop[=no]     Filter apps by OnlyShowIn/NotShowIn (default: yes)
      --list-executables-in-path Include executables from $PATH
      --hide-before-typing      Hide list until first character typed
      --match-mode <mode>       Match mode: 'fuzzy' or 'exact' (default: fuzzy)

Dmenu Mode Options:
      --dmenu                   Dmenu mode: read from stdin, output selection to stdout
      --dmenu0                  Like --dmenu but null-separated input
      --password[=char]         Password mode: mask input (default char: *)
      --index                   Output index instead of text
      --with-nth <cols>         Display only specified columns (comma-separated, e.g., 1,3)
      --accept-nth <cols>       Output only specified columns
      --match-nth <cols>        Match against only specified columns
      --delimiter <char>        Column delimiter (default: space)
      --only-match              Don't allow custom input, only return selected items
      --exit-if-empty           Exit immediately if stdin is empty
      --select <string>         Pre-select first matching entry
      --select-index <n>        Pre-select entry at index n
      --auto-select             Auto-select when only one match remains
      --prompt-only             Prompt-only mode: no list, just input

Clipboard Mode Options:
      --cclip                   Clipboard history mode: browse cclip history with previews

General Options:
  -h, --help                    Show this help message
  -V, --version                 Show the version number and quit
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
windowrule = float, ^(launcher)$
windowrule = size 500 430, ^(launcher)$
```

**dwm/bspwm/any WM:**
```sh
# Use dmenu mode
bindsym $mod+d exec "fsel --dmenu | xargs swaymsg exec --"
```

## Contributing

Feature requests and bug reports are welcome.

Pull requests for bug fixes or requested features are accepted.

Please use GitHub issues and pull requests for contributions.

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
- Use Kitty (best) or Sixel-capable terminal (Foot, WezTerm, etc.)
- Install `chafa` for image rendering
- Check `image_preview = true` in config
- Images automatically render inside content panel borders

**Fuzzy matching too loose?**
- Try `--match-mode=exact` for stricter matching
- Or set `match_mode = "exact"` in config

**Terminal apps not launching?**
- Set `terminal_launcher` in config
- Example: `terminal_launcher = "kitty -e"`

## Credits

Fork of [gyr](https://git.sr.ht/~nkeor/gyr) by Namkhai B.

## License

[BSD 2-Clause](./LICENSE) (c) 2020-2022 Namkhai B.
