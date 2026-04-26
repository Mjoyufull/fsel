pub(crate) fn short_usage(program_name: &str) -> String {
    format!(
        "fsel — Fast terminal application launcher
Usage:
  {program_name} [OPTIONS]

├─ Core Modes
│  ├─ -p, --program <NAME>         Launch one app immediately; exact mode refuses near matches
│  ├─ --dmenu                      Read choices from stdin and print the selection
│  └─ --cclip                      Browse clipboard history and copy the selection
│
├─ Common Flags
│  ├─ -c, --config <FILE>          Read config from FILE instead of ~/.config/fsel/config.toml
│  ├─ -r, --replace                Replace an existing fsel/cclip instance before starting
│  ├─ -d, --detach                 Start launched apps without keeping this terminal attached
│  ├─ -t, --tty                    Run terminal apps in this TTY instead of a terminal launcher
│  ├─ -v, --verbose                Print more diagnostics; repeat as -vv or -vvv for more detail
│  ├─ -T, --test                   Enable debug logging and imply maximum verbosity
│  └─ -ss <SEARCH>                 Pre-fill the search box; place this last on the command line
│
├─ Launch Methods
│  ├─ --no-exec                    Print the selected item instead of launching it
│  ├─ --launch-prefix <CMD>        Prefix launches with a custom command such as 'runapp --'
│  ├─ --systemd-run                Launch through systemd-run --user --scope
│  └─ --uwsm                       Launch through uwsm app --
│
├─ App Launcher Extras
│  ├─ --clear-history              Delete launch history, then exit
│  ├─ --clear-cache                Delete the desktop entry cache, then exit
│  ├─ --refresh-cache              Rescan desktop entries before showing results
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn; pass =no to ignore them
│  ├─ --filter-actions[=no]        Hide desktop actions; pass =no to keep entries like new window
│  ├─ --hide-before-typing         Keep the list hidden until you type the first character
│  ├─ --stdout                     Print filtered desktop entries to stdout in json form
│  ├─ --list-executables-in-path   Include executables from $PATH in launcher mode
│  ├─ --match-mode <MODE>          Choose fuzzy or exact matching
│  └─ --prefix-depth <N>           Tune how long prefix matches outrank fuzzy matches
│
├─ Mode-Specific Flags
│  ├─ Dmenu: --dmenu0 --password[=CHAR] --index --with-nth --accept-nth
│  ├─        --match-nth --delimiter --only-match --exit-if-empty
│  ├─        --select --select-index --auto-select --prompt-only
│  └─ Cclip: --tag <NAME|list|clear|wipe> --cclip-show-tag-color-names
│
└─ Help
   ├─ -h                           Show this summary
   ├─ -H, --help                   Show the full option tree with notes
   └─ -V, --version                Print the version and exit
"
    )
}

pub(crate) fn detailed_usage(program_name: &str) -> String {
    format!(
        "fsel — Fast terminal application launcher
Usage:
  {program_name} [OPTIONS]

├─ Core Modes
│  ├─ -p, --program <NAME>         Launch one app immediately; exact mode requires an exact hit
│  ├─ --cclip                      Browse clipboard history and copy the selected item
│  └─ --dmenu                      Read choices from stdin and print the selection to stdout
│
├─ Startup and Output
│  ├─ -c, --config <FILE>          Read config from FILE before applying CLI overrides
│  ├─ -r, --replace                Replace an existing fsel/cclip instance before starting
│  ├─ -d, --detach                 Start launched GUI apps without keeping this terminal attached
│  ├─ -t, --tty                    Run terminal apps in this TTY and replace the fsel process
│  ├─ -v, --verbose                Print more diagnostics; repeat as -vv or -vvv for more detail
│  ├─ -T, --test                   Enable debug logging, write logs under ~/.config/fsel/logs/, and imply -vvv
│  ├─ --no-exec                    Print the selected item instead of launching it
│  └─ -ss <SEARCH>                 Pre-fill the search box; place this last so it captures the rest
│
├─ Launch Methods
│  ├─ --launch-prefix <CMD>        Prefix launches with a custom command such as 'runapp --'
│  ├─ --systemd-run                Launch through systemd-run --user --scope
│  └─ --uwsm                       Launch through uwsm app --
│
├─ App Launcher Tuning
│  ├─ --clear-history              Delete launch history, then exit
│  ├─ --clear-cache                Delete the desktop entry cache, then exit
│  ├─ --refresh-cache              Rescan desktop entries before showing results
│  ├─ --filter-desktop[=no]        Respect OnlyShowIn/NotShowIn; pass =no to ignore them
│  ├─ --filter-actions[=no]        Hide desktop actions; pass =no to keep entries like new window
│  ├─ --hide-before-typing         Keep the list hidden until you type the first character
│  ├─ --list-executables-in-path   Include executables from $PATH in launcher mode
│  ├─ --match-mode <MODE>          Choose fuzzy or exact matching (default: fuzzy)
│  └─ --prefix-depth <N>           Set how long prefix matches outrank fuzzy matches (default: 3)
│
├─ Dmenu Mode Options
│  ├─ --dmenu0                     Read NUL-separated input instead of newline-separated input
│  ├─ --password[=CHAR]            Mask typed input; optionally choose the mask character
│  ├─ --index                      Print the selected row index instead of the row text
│  ├─ --with-nth <COLS>            Show only these 1-based columns (example: 1,3)
│  ├─ --accept-nth <COLS>          Print only these columns after selection
│  ├─ --match-nth <COLS>           Search only within these columns
│  ├─ --delimiter <CHAR>           Split columns on CHAR instead of spaces
│  ├─ --only-match                 Reject custom text and require a selection from stdin
│  ├─ --exit-if-empty              Quit immediately when stdin provides no items
│  ├─ --select <STRING>            Start with the first matching row preselected
│  ├─ --select-index <N>           Start with row N preselected
│  ├─ --auto-select                Accept automatically when the filtered list reaches one row
│  └─ --prompt-only                Show only the input prompt and hide the list pane
│
├─ Clipboard Mode Options
│  ├─ --tag <NAME>                 Show only clipboard entries tagged NAME
│  ├─ --tag list                   List known tags, then exit
│  ├─ --tag list <NAME>            List clipboard entries carrying NAME, then exit
│  ├─ --tag clear                  Remove stored tag metadata
│  ├─ --tag wipe                   Remove all tags from every clipboard entry
│  └─ --cclip-show-tag-color-names Show tag color names next to tags in cclip mode
│
├─ General
│  ├─ -h                           Show the short summary
│  ├─ -H, --help                   Show this full option tree
│  └─ -V, --version                Print the version and exit
│
└─ Notes
   ├─ Pick only one launch method: --launch-prefix, --systemd-run, or --uwsm
   ├─ --dmenu and --cclip both imply --no-exec
   ├─ --program respects --match-mode: exact requires an exact app or executable name
   ├─ --select and --select-index cannot be combined
   └─ Default config path: ~/.config/fsel/config.toml
"
    )
}

pub(super) fn unknown_argument_help(error_message: &str) -> String {
    format!(
        "Error: {error_message}

Quick help:
  -c, --config <FILE>    Read config from FILE
  -p, --program <NAME>   Launch one app immediately and skip the TUI
  -ss <SEARCH>           Pre-fill the search box; place this last
  --dmenu                Read choices from stdin and print the selection
  --cclip                Browse clipboard history and copy the selection
  --no-exec              Print the selected item instead of launching it
  -r, --replace          Replace an existing fsel/cclip instance
  -d, --detach           Start launched apps without keeping the terminal attached
  -v, --verbose          Print more diagnostics; repeat as -vv or -vvv
  -h                     Show the short summary
  -H, --help             Show the full option tree
  -V, --version          Print the version and exit

Run 'fsel -h' for a summary or 'fsel --help' for the full option tree.
"
    )
}
