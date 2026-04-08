use super::error::CliError;
use super::launch::active_launch_method_count;
use super::types::Opts;

pub(super) fn validate(default: &mut Opts, cli_launch_methods: usize) -> Result<(), CliError> {
    if default.program.is_some() && default.search_string.is_some() {
        return Err(CliError::message(
            "Error: Cannot use -p/--program and -ss together\n\
Use -p for direct launch or -ss for pre-filled TUI search\n",
        ));
    }

    if default.dmenu_mode {
        if default.program.is_some() {
            return Err(CliError::message(
                "Error: --dmenu cannot be used with -p/--program\n\
Dmenu mode reads from stdin and outputs to stdout\n",
            ));
        }
        default.no_exec = true;
    }

    if default.dmenu_prompt_only && default.dmenu_mode {
        default.dmenu_show_line_numbers = false;
    }

    if default.dmenu_select.is_some() && default.dmenu_select_index.is_some() {
        return Err(CliError::message(
            "Error: Cannot use --select and --select-index together\n",
        ));
    }

    if default.cclip_mode {
        if default.program.is_some() {
            return Err(CliError::message(
                "Error: --cclip cannot be used with -p/--program\n\
Cclip mode browses clipboard history and copies selection\n",
            ));
        }
        default.no_exec = true;
    }

    if (default.cclip_tag.is_some()
        || default.cclip_tag_list
        || default.cclip_clear_tags
        || default.cclip_wipe_tags)
        && !default.cclip_mode
    {
        return Err(CliError::message(
            "Error: --tag requires --cclip mode\n\
Use one of these forms:\n\
  fsel --cclip --tag <name>\n\
  fsel --cclip --tag list\n\
  fsel --cclip --tag list <name>\n\
  fsel --cclip --tag clear\n\
  fsel --cclip --tag wipe\n",
        ));
    }

    if default.dmenu_mode && default.cclip_mode {
        return Err(CliError::message(
            "Error: --dmenu and --cclip cannot be used together\n",
        ));
    }

    if default.no_exec {
        if active_launch_method_count(default) > 0 {
            eprintln!("Warning: --no-exec overrides other launch method flags");
        }
        return Ok(());
    }

    if cli_launch_methods > 1 || active_launch_method_count(default) > 1 {
        return Err(CliError::message(
            "Error: Only one launch method can be specified at a time\n\
Available methods: --launch-prefix, --systemd-run, --uwsm\n",
        ));
    }

    Ok(())
}
