use super::error::CliError;
use super::help::unknown_argument_help;
use super::launch::{parse_launch_prefix, set_launch_prefix, set_systemd_run, set_uwsm};
use super::types::{MatchMode, Opts};
use super::{CliCommand, validate};
use crate::config::FselConfig;
use lexopt::prelude::*;
use std::env;
use std::path::PathBuf;

enum OverridesResult {
    Continue(usize),
    Command(Box<CliCommand>),
}

pub fn parse() -> Result<CliCommand, CliError> {
    parse_from(env::args())
}

pub(crate) fn parse_from<I, S>(args: I) -> Result<CliCommand, CliError>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let args = collect_args(args);
    let config = FselConfig::new(find_config_path(&args))?;
    parse_with_config(&args, config)
}

fn parse_with_config(args: &[String], config: FselConfig) -> Result<CliCommand, CliError> {
    let program_name = args.first().cloned().unwrap_or_else(|| "fsel".to_string());
    let mut default = Opts::default();
    super::from_config::apply_config_defaults(&mut default, &config);

    if program_name.ends_with("dmenu") {
        default.dmenu_mode = true;
    }

    let mut parser = build_parser(args, &mut default);
    let cli_launch_methods = match parse_cli_overrides(&mut parser, &mut default, &program_name)? {
        OverridesResult::Continue(count) => count,
        OverridesResult::Command(command) => return Ok(*command),
    };

    validate::validate(&mut default, cli_launch_methods)?;
    Ok(CliCommand::Run(Box::new(default)))
}

fn collect_args<I, S>(args: I) -> Vec<String>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        args.push("fsel".to_string());
    }
    args
}

fn find_config_path(args: &[String]) -> Option<PathBuf> {
    let mut index = 1;
    while index < args.len() {
        if (args[index] == "-c" || args[index] == "--config") && index + 1 < args.len() {
            return Some(PathBuf::from(&args[index + 1]));
        }
        index += 1;
    }
    None
}

fn build_parser(args: &[String], default: &mut Opts) -> lexopt::Parser {
    if let Some(search_pos) = args.iter().position(|arg| arg == "-ss") {
        default.search_string = Some(args[search_pos + 1..].join(" "));
        return lexopt::Parser::from_args(args[..search_pos].iter().skip(1).cloned());
    }

    lexopt::Parser::from_args(args.iter().skip(1).cloned())
}

fn parse_cli_overrides(
    parser: &mut lexopt::Parser,
    default: &mut Opts,
    program_name: &str,
) -> Result<OverridesResult, CliError> {
    let mut cli_launch_methods = 0;

    while let Some(arg) = parser.next()? {
        match arg {
            Short('t') | Long("tty") => {
                default.tty = true;
                default.terminal_launcher.clear();
            }
            Short('r') | Long("replace") => {
                default.replace = true;
            }
            Short('c') | Long("config") => {
                let _ = parser.value()?;
            }
            Long("clear-history") => {
                default.clear_history = true;
            }
            Long("clear-cache") => {
                default.clear_cache = true;
            }
            Long("refresh-cache") => {
                default.refresh_cache = true;
            }
            Long("no-exec") => {
                default.no_exec = true;
            }
            Long("launch-prefix") => {
                cli_launch_methods += 1;
                let prefix = value_as_string(parser, "Launch prefix must be valid UTF-8")?;
                set_launch_prefix(
                    default,
                    parse_launch_prefix(&prefix).map_err(CliError::message)?,
                );
            }
            Long("systemd-run") => {
                cli_launch_methods += 1;
                set_systemd_run(default);
            }
            Long("uwsm") => {
                cli_launch_methods += 1;
                set_uwsm(default);
            }
            Short('d') | Long("detach") => {
                default.detach = true;
            }
            Long("dmenu") => {
                default.dmenu_mode = true;
            }
            Long("cclip") => {
                default.cclip_mode = true;
            }
            Long("tag") => parse_tag(parser, default)?,
            Long("cclip-show-tag-color-names") => {
                default.cclip_show_tag_color_names = Some(true);
            }
            Long("dmenu0") => {
                default.dmenu_mode = true;
                default.dmenu_null_separated = true;
            }
            Long("password") => {
                default.dmenu_password_mode = true;
                if let Some(value) = parser.optional_value() {
                    default.dmenu_password_character = value
                        .into_string()
                        .map_err(|_| CliError::message("Password character must be valid UTF-8"))?;
                }
            }
            Long("index") => {
                default.dmenu_index_mode = true;
            }
            Long("accept-nth") => {
                default.dmenu_accept_nth =
                    Some(parse_column_list(parser, "Invalid column specification")?);
            }
            Long("match-nth") => {
                default.dmenu_match_nth =
                    Some(parse_column_list(parser, "Invalid column specification")?);
            }
            Long("only-match") => {
                default.dmenu_only_match = true;
            }
            Long("exit-if-empty") => {
                default.dmenu_exit_if_empty = true;
            }
            Long("select") => {
                default.dmenu_select = Some(value_as_string(
                    parser,
                    "Select string must be valid UTF-8",
                )?);
            }
            Long("select-index") => {
                let index = value_as_string(parser, "Index must be valid UTF-8")?;
                default.dmenu_select_index = Some(
                    index
                        .parse::<usize>()
                        .map_err(|_| CliError::message("Invalid index"))?,
                );
            }
            Long("auto-select") => {
                default.dmenu_auto_select = true;
            }
            Long("prompt-only") => {
                default.dmenu_prompt_only = true;
            }
            Long("hide-before-typing") => {
                default.hide_before_typing = true;
            }
            Long("filter-desktop") => {
                if let Some(value) = parser.optional_value() {
                    let value = value.into_string().map_err(|_| {
                        CliError::message("filter-desktop value must be valid UTF-8")
                    })?;
                    default.filter_desktop = value != "no";
                } else {
                    default.filter_desktop = true;
                }
            }
            Long("list-executables-in-path") => {
                default.list_executables_in_path = true;
            }
            Long("match-mode") => {
                let mode = value_as_string(parser, "Match mode must be valid UTF-8")?;
                default.match_mode = mode
                    .parse::<MatchMode>()
                    .map_err(|_| CliError::message("Invalid match mode. Use 'exact' or 'fuzzy'"))?;
            }
            Long("prefix-depth") => {
                let depth = value_as_string(parser, "Prefix depth must be valid UTF-8")?;
                default.prefix_depth = depth
                    .parse::<usize>()
                    .map_err(|_| CliError::message("Invalid prefix depth"))?;
            }
            Short('T') | Long("test") => {
                default.test_mode = true;
                default.verbose = Some(3);
            }
            Long("with-nth") => {
                default.dmenu_with_nth = Some(parse_column_list(
                    parser,
                    "Invalid column specification. Use comma-separated numbers like: 1,2,4",
                )?);
            }
            Long("delimiter") => {
                default.dmenu_delimiter = value_as_string(parser, "Delimiter must be valid UTF-8")?;
            }
            Short('p') | Long("program") => {
                default.program =
                    Some(value_as_string(parser, "Program name must be valid UTF-8")?);
            }
            Short('v') | Long("verbose") => {
                default.verbose = Some(default.verbose.unwrap_or(0) + 1);
            }
            Short('h') => {
                return Ok(OverridesResult::Command(Box::new(
                    CliCommand::PrintShortHelp {
                        program_name: program_name.to_string(),
                    },
                )));
            }
            Short('H') | Long("help") => {
                return Ok(OverridesResult::Command(Box::new(
                    CliCommand::PrintLongHelp {
                        program_name: program_name.to_string(),
                    },
                )));
            }
            Short('V') | Long("version") => {
                return Ok(OverridesResult::Command(Box::new(CliCommand::PrintVersion)));
            }
            Value(_) => return Err(arg.unexpected().into()),
            _ => return Err(report_unknown_argument(arg)),
        }
    }

    Ok(OverridesResult::Continue(cli_launch_methods))
}

fn parse_tag(parser: &mut lexopt::Parser, default: &mut Opts) -> Result<(), CliError> {
    let tag_arg = value_as_string(parser, "Tag argument must be valid UTF-8")?;
    match tag_arg.as_str() {
        "list" => {
            default.cclip_tag_list = true;
            if let Ok(value) = parser.value() {
                default.cclip_tag = Some(
                    value
                        .into_string()
                        .map_err(|_| CliError::message("Tag name must be valid UTF-8"))?,
                );
            }
        }
        "clear" => {
            default.cclip_clear_tags = true;
        }
        "wipe" => {
            default.cclip_wipe_tags = true;
        }
        _ => {
            default.cclip_tag = Some(tag_arg);
        }
    }

    Ok(())
}

fn value_as_string(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<String, CliError> {
    parser
        .value()?
        .into_string()
        .map_err(|_| CliError::message(error_message))
}

fn parse_column_list(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<Vec<usize>, CliError> {
    let columns = value_as_string(parser, "Column specification must be valid UTF-8")?;
    columns
        .split(',')
        .map(|part| part.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| CliError::message(error_message))
}

fn report_unknown_argument(arg: lexopt::Arg<'_>) -> CliError {
    let error_msg = match arg {
        Long(name) => match name {
            "clip" => "Unknown option '--clip'. Did you mean '--cclip'?",
            "menu" => "Unknown option '--menu'. Did you mean '--dmenu'?",
            "dme" | "dmen" => "Unknown option. Did you mean '--dmenu'?",
            "cc" | "ccli" => "Unknown option. Did you mean '--cclip'?",
            _ => "Unknown option. Use '-h' or '--help' to see available options.",
        },
        Short(c) => match c {
            'C' => "Unknown option '-C'. Did you mean '-c' for --config?",
            'P' => "Unknown option '-P'. Did you mean '-p' for --program?",
            'R' => "Unknown option '-R'. Did you mean '-r' for --replace?",
            'H' => "Unknown option '-H'. Did you mean '-h' for --help?",
            _ => "Unknown option. Use '-h' or '--help' to see available options.",
        },
        Value(_) => unreachable!(),
    };

    CliError::message(unknown_argument_help(error_msg))
}

#[cfg(test)]
mod tests {
    use super::{CliCommand, CliError, parse_with_config};
    use crate::config::FselConfig;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| value.to_string()).collect()
    }

    #[test]
    fn short_help_returns_command_without_exiting() {
        let command = parse_with_config(&args(&["fsel", "-h"]), FselConfig::default()).unwrap();
        assert!(matches!(command, CliCommand::PrintShortHelp { .. }));
    }

    #[test]
    fn version_returns_command_without_exiting() {
        let command =
            parse_with_config(&args(&["fsel", "--version"]), FselConfig::default()).unwrap();
        assert!(matches!(command, CliCommand::PrintVersion));
    }

    #[test]
    fn invalid_tag_mode_returns_typed_error() {
        let error = parse_with_config(&args(&["fsel", "--tag", "list"]), FselConfig::default())
            .unwrap_err();
        assert!(
            matches!(error, CliError::Message(message) if message.contains("--tag requires --cclip mode"))
        );
    }
}
