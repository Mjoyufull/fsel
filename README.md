<div align="center">

  ![Logo](./assets/gyr.png)

  [![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://github.com/Mjoyufull/gyr/blob/main/LICENSE)
  [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  Fast TUI launcher for GNU/Linux and \*BSD

  <img width="840" height="953" alt="image" src="https://github.com/user-attachments/assets/d2af06f1-0331-4fa9-a24a-54e562fc4c6f" />


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
