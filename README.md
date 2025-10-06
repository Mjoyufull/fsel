<div align="center">

  ![Logo](./assets/gyr.png)

  [![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://github.com/Mjoyufull/gyr/blob/main/LICENSE)
  [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  Fast TUI app launcher and fuzzy finder for GNU/Linux and \*BSD

  <img width="860" height="1019" alt="Screenshot_20251006-032156" src="https://github.com/user-attachments/assets/777bd0a4-eb52-4014-837b-d361ab57cfff" />



</div>

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [TODO](#todo)
- [Contributing](#contributing)
- [Changelog](#changelog)
- [License](#license)

## Install

#### Option 1: Nix Flake (Recommended)

* Build and run with Nix flakes:
    ```sh
    $ nix run github:Mjoyufull/gyr
    ```

* Add to your profile:
    ```sh
    $ nix profile add github:Mjoyufull/gyr
    ```

* Add to your `flake.nix` inputs:
    ```nix
    {
      inputs.gyr.url = "github:Mjoyufull/gyr";
      # ... rest of your flake
    }
    ```

#### Option 2: Curl Install (Recommended for most users)

* Install directly from GitHub:
    ```sh
    $ curl -sSL https://raw.githubusercontent.com/Mjoyufull/gyr/main/install.sh | bash
    ```
* To update later, run the same command

#### Option 3: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ git clone https://github.com/Mjoyufull/gyr && cd gyr
    $ cargo build --release
    ```
* Copy `target/release/gyr` to somewhere in your `$PATH`

### Optional Dependencies

* **uwsm** - Universal Wayland Session Manager (for `--uwsm` flag)
* **systemd** - For `--systemd-run` flag (usually pre-installed on most Linux distributions)
* **sway** - For automatic Sway integration when `$SWAYSOCK` is set
* [**heather7283/cclip** ](https://github.com/heather7283/cclip) - Clipboard manager (for `--cclip` mode with clipboard history browsing)
* **chafa** - Terminal image viewer (for image previews in cclip mode)
* **Kitty** or **Sixel-capable terminal** - For inline image rendering support

## Usage

### Interactive Mode

Run `gyr` from a terminal to open the interactive TUI launcher.

#### Navigation

**Keyboard:**
- Type to search/filter applications
- `↑`/`↓` or `Ctrl-P`/`Ctrl-N` to navigate up/down
- `←`/`→` to jump to top/bottom of list
- `Enter` or `Ctrl-Y` to launch selected application
- `Esc` or `Ctrl-Q` to exit
- `Backspace` to remove characters from search

**Mouse:**
- Hover over applications to select them
- Click on an application to launch it
- Scroll wheel to scroll through the application list
- All mouse interactions work alongside keyboard navigation

#### Features

- **Fuzzy Search**: Type partial names to find applications quickly
- **Smart Matching**: Searches app names, descriptions, keywords, and categories
- **Usage History**: Frequently used applications appear higher in results
- **Real-time Filtering**: Results update as you type

### Direct Launch Mode

Launch applications directly from the command line without opening the TUI:

```sh
# Launch Firefox directly
gyr -p firefox

# Launch first match for "terminal"
gyr -p terminal

# Works with partial names
gyr -p fire  # Finds Firefox

# Combine with launch options
gyr --uwsm -p discord
gyr --systemd-run -vv -p code
```

### Pre-filled Search Mode

Open the TUI with a pre-filled search string:

```sh
# Open TUI with "firefox" already searched
gyr -ss firefox

# Multi-word search terms work
gyr -ss web browser

# Combine with other options (must be last)
gyr --uwsm -vv -r -ss text editor

# Search for anything, even non-existent apps
gyr -ss asdhaskdjahs
```

### Dmenu Mode

Gyr now includes a full dmenu replacement mode that reads from stdin and outputs selections to stdout:

```sh
# Basic dmenu replacement
echo -e "Option 1\nOption 2\nOption 3" | gyr --dmenu

# Display only specific columns (like cut)
ps aux | gyr --dmenu --with-nth 2,11  # Show only PID and command

# Use custom delimiter
echo "foo:bar:baz" | gyr --dmenu --delimiter ":"

# Pipe from any command
ls -la | gyr --dmenu
find . -name "*.rs" | gyr --dmenu
git log --oneline | gyr --dmenu
```

#### Dmenu Features
- **Column Filtering**: Use `--with-nth` to display only specific columns
- **Custom Delimiters**: Use `--delimiter` to specify column separators
- **Content Preview**: Selected line content is shown in the top panel
- **Fuzzy Matching**: Same powerful fuzzy search as regular mode
- **Line Numbers**: Optional line numbers in content display

### Clipboard History Mode
<img width="853" height="605" alt="image" src="https://github.com/user-attachments/assets/0bf71952-f09a-4ce2-8807-bca1003c8daf" />

Browse and select from your clipboard history with image previews:

```sh
# Browse clipboard history with cclip integration
gyr --cclip
```

#### Clipboard Features
- **Image Previews**: Inline image rendering for copied images (Kitty/Sixel protocols)
- **Content Preview**: Full text preview in the top panel
- **History Navigation**: Browse through clipboard history with fuzzy search
- **Line Numbers**: Shows actual cclip rowid for each entry
- **Smart Copying**: Automatically copies selection back to clipboard

### Usage Examples

#### As a dmenu replacement:
```sh
# Simple menu selection
echo -e "Edit\nView\nDelete" | gyr --dmenu

# Process selection with column filtering
ps aux | gyr --dmenu --with-nth 2,11 | xargs kill  # Select and kill process

# File browser
find . -type f | gyr --dmenu | xargs open  # Select and open file

# Git branch switcher
git branch | gyr --dmenu | xargs git checkout

# SSH connection picker
grep "^Host " ~/.ssh/config | gyr --dmenu --with-nth 2 | xargs ssh
```

#### Window manager integration:
```sh
# Sway/i3 window switcher
swaymsg -t get_tree | jq -r '..|select(.type=="con" and .name!=null)|.name' | gyr --dmenu | xargs swaymsg '[title="^.*"] focus'

# Application launcher (traditional usage)
sway: bindsym $mod+d exec 'alacritty --title launcher -e gyr'

# Clipboard history browser
sway: bindsym $mod+v exec 'alacritty --title clipboard -e gyr --cclip'
```

### Command Line Options

```
Usage: gyr [options]

  -s, --nosway           Disable Sway integration
  -c, --config <config>  Specify a config file
  -r, --replace          Replace existing gyr instances
      --clear_history    Clear launch history
  -p, --program <name>   Launch program directly (bypass TUI)
  -ss <search>           Pre-fill search in TUI (must be last option)
  -v, --verbose          Increase verbosity level (multiple)
      --no-exec          Print selected application to stdout instead of launching
      --systemd-run      Launch applications using systemd-run --user --scope
      --uwsm             Launch applications using uwsm app
      --dmenu            Dmenu mode: read from stdin, output selection to stdout
      --cclip            Clipboard history mode: browse cclip history with previews
      --with-nth <cols>  Display only specified columns (comma-separated, e.g., 1,3)
      --delimiter <char> Column delimiter for --with-nth (default: space)
  -h, --help             Show this help message
  -V, --version          Show the version number and quit
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

### Configuration

Gyr supports extensive customization through a configuration file located at:
- `$XDG_CONFIG_HOME/gyr/config.toml` or
- `$HOME/.config/gyr/config.toml`

See the [sample configuration](./config.toml) for available options including:
- Color schemes and UI customization
- Panel layout and sizing
- Border styles and cursor appearance
- Terminal launcher configuration

### Sway-specific usage

Example Sway configuration:

```shell
$ cat ~/.config/sway/config
...
set $menu alacritty --title launcher -e gyr
bindsym $mod+d exec $menu
for_window [title="^launcher$"] floating enable, resize set width 500 height 430, border none
...
```

## TODO

* [X] Most used entries first
* [X] Mouse support (hover, click, scroll wheel)
* [X] Direct launch mode for command-line usage
* [X] Multiple launch backends (systemd-run, uwsm, sway)
* [X] UI customization options
* [X] XDG Desktop Entry specification compliance
* [X] Nix flake for universal installation
* [X] Dmenu mode with stdin/stdout interface
* [X] Column filtering and custom delimiters for dmenu mode
* [X] Clipboard history integration (cclip mode)
* [X] Image preview support for clipboard content
* [X] Configurable UI themes for different modes

## Contributing

Feature requests and bug reports are welcome.

Pull requests for bug fixes or requested features are accepted.

Please use GitHub issues and pull requests for contributions.

## Changelog

Notable changes will be documented in the [CHANGELOG](./CHANGELOG.md) file

## Credits

This project is a fork of the original [gyr](https://git.sr.ht/~nkeor/gyr) by Namkhai B. 
Major appreciation to the original developer for creating this excellent TUI launcher.

## License

[BSD 2-Clause](./LICENSE) (c) 2020-2022 Namkhai B.
