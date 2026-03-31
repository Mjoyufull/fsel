use super::launch::active_launch_method_count;
use super::types::Opts;

pub(super) fn validate(default: &mut Opts, cli_launch_methods: usize) {
    if default.program.is_some() && default.search_string.is_some() {
        eprintln!("Error: Cannot use -p/--program and -ss together");
        eprintln!("Use -p for direct launch or -ss for pre-filled TUI search");
        std::process::exit(1);
    }

    if default.dmenu_mode {
        if default.program.is_some() {
            eprintln!("Error: --dmenu cannot be used with -p/--program");
            eprintln!("Dmenu mode reads from stdin and outputs to stdout");
            std::process::exit(1);
        }
        default.no_exec = true;
    }

    if default.dmenu_prompt_only && default.dmenu_mode {
        default.dmenu_show_line_numbers = false;
    }

    if default.dmenu_select.is_some() && default.dmenu_select_index.is_some() {
        eprintln!("Error: Cannot use --select and --select-index together");
        std::process::exit(1);
    }

    if default.cclip_mode {
        if default.program.is_some() {
            eprintln!("Error: --cclip cannot be used with -p/--program");
            eprintln!("Cclip mode browses clipboard history and copies selection");
            std::process::exit(1);
        }
        default.no_exec = true;
    }

    if (default.cclip_tag.is_some()
        || default.cclip_tag_list
        || default.cclip_clear_tags
        || default.cclip_wipe_tags)
        && !default.cclip_mode
    {
        eprintln!("Error: --tag requires --cclip mode");
        eprintln!("Use one of these forms:");
        eprintln!("  fsel --cclip --tag <name>");
        eprintln!("  fsel --cclip --tag list");
        eprintln!("  fsel --cclip --tag list <name>");
        eprintln!("  fsel --cclip --tag clear");
        eprintln!("  fsel --cclip --tag wipe");
        std::process::exit(1);
    }

    if default.dmenu_mode && default.cclip_mode {
        eprintln!("Error: --dmenu and --cclip cannot be used together");
        std::process::exit(1);
    }

    if default.no_exec {
        if active_launch_method_count(default) > 0 {
            eprintln!("Warning: --no-exec overrides other launch method flags");
        }
        return;
    }

    if cli_launch_methods > 1 || active_launch_method_count(default) > 1 {
        eprintln!("Error: Only one launch method can be specified at a time");
        eprintln!("Available methods: --launch-prefix, --systemd-run, --uwsm");
        std::process::exit(1);
    }
}
