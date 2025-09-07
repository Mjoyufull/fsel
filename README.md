<div align="center">

  ![Logo](./assets/gyr.png)

  [![License](https://img.shields.io/crates/l/gyr?style=flat-square)](https://github.com/Mjoyufull/gyr/blob/main/LICENSE)
  [![Latest version](https://img.shields.io/crates/v/gyr?style=flat-square)](https://crates.io/crates/gyr)
  [![standard-readme compliant](https://img.shields.io/badge/readme%20style-standard-brightgreen.svg?style=flat-square)](https://github.com/RichardLitt/standard-readme)
  ![written in Rust](https://img.shields.io/badge/language-rust-red.svg?style=flat-square)

  Fast TUI launcher for GNU/Linux and \*BSD

  [![asciicast](https://asciinema.org/a/n34HCGxXINEoryRkuM8XOIVbJ.svg)](https://asciinema.org/a/n34HCGxXINEoryRkuM8XOIVbJ)

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

* Install to your profile:
    ```sh
    $ nix profile install github:Mjoyufull/gyr
    ```

* Add to your `flake.nix` inputs:
    ```nix
    {
      inputs.gyr.url = "github:Mjoyufull/gyr";
      # ... rest of your flake
    }
    ```

#### Option 2: Build from source

* Install [Rust](https://www.rust-lang.org/learn/get-started)
* Build:
    ```sh
    $ git clone https://github.com/Mjoyufull/gyr && cd gyr
    $ cargo build --release
    ```
* Copy `target/release/gyr` to somewhere in your `$PATH`


## Usage

Run `gyr` from a terminal. Scroll through the app list, find some app typing chars, run selected pressing ENTER. Pretty straightforward.
Oh, yes: go to the bottom with the left arrow, top with right. Cancel pressing Esc.

Alternative bindings are Ctrl-Q to cancel, Ctrl-Y to run the app, Ctrl-N scroll down and Ctrl-P to scroll up (VIM bindings).

Gyr works well with tiling window managers like [Sway](https://swaywm.org/) or [i3](https://i3wm.org/).

> Note for Sway: When `$SWAYSOCK` is set, `swaymsg exec` is used to run the program.
> This allows Sway to spawn the program in the workspace Gyr was run in.

You can configure some stuff with cli flags, see `gyr --help`

Gyr also has a history feature, so most used entries will be sorted first. This can be reset with `gyr --clear_history`

There's also a config file which can be placed in `$HOME/.config/gyr/config.toml` or `$XDG_DATA_HOME/gyr/config.toml` ([sample](./config.toml))

Verbosity levels (`-v`, `-vv`, `-vvv`, each level adds logs to the previous one):

* `-v`: will make the launched binary inherit Gyr's `stdio`. (which means you'll see the logs)
* `-vv`: will show the path of each app in the info
* `-vvv`: adds some debug information (number of times the apps were run, etc.)

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
* [X] Cached entries
* [X] Multiple launch backends (systemd-run, uwsm, sway)
* [X] UI customization options
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
