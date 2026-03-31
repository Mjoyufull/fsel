use super::from_config::{ConfigDefaultsError, apply_config_defaults};
use super::help::{detailed_usage, usage};
use super::launch::{parse_launch_prefix, set_launch_prefix, set_systemd_run, set_uwsm};
use super::types::{MatchMode, Opts};
use super::validate;
use lexopt::prelude::*;
use std::env;
use std::path::PathBuf;

pub fn parse() -> Result<Opts, lexopt::Error> {
    let args: Vec<String> = env::args().collect();
    let config_path_cli = find_config_path(&args);
    let fsel_config = match crate::config::FselConfig::new(config_path_cli) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("Error loading configuration: {error}");
            std::process::exit(1);
        }
    };

    let mut default = Opts::default();
    if let Err(error) = apply_config_defaults(&mut default, &fsel_config) {
        report_config_defaults_error(error);
    }

    if args.first().is_some_and(|arg0| arg0.ends_with("dmenu")) {
        default.dmenu_mode = true;
    }

    let mut parser = build_parser(&args, &mut default);
    let cli_launch_methods = parse_cli_overrides(&mut parser, &mut default)?;
    validate::validate(&mut default, cli_launch_methods);
    Ok(default)
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
    if let Some(ss_pos) = args.iter().position(|arg| arg == "-ss") {
        default.search_string = Some(args[ss_pos + 1..].join(" "));
        return lexopt::Parser::from_args(args[..ss_pos].iter().skip(1).cloned());
    }

    lexopt::Parser::from_args(args.iter().skip(1).cloned())
}

fn parse_cli_overrides(
    parser: &mut lexopt::Parser,
    default: &mut Opts,
) -> Result<usize, lexopt::Error> {
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
                    parse_launch_prefix(&prefix).map_err(lexopt::Error::from)?,
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
            Long("tag") => {
                let tag_arg = value_as_string(parser, "Tag argument must be valid UTF-8")?;
                match tag_arg.as_str() {
                    "list" => {
                        default.cclip_tag_list = true;
                        if let Ok(value) = parser.value() {
                            default.cclip_tag = Some(
                                value
                                    .into_string()
                                    .map_err(|_| "Tag name must be valid UTF-8")?,
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
            }
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
                        .map_err(|_| "Password character must be valid UTF-8")?;
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
                default.dmenu_select_index =
                    Some(index.parse::<usize>().map_err(|_| "Invalid index")?);
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
                    let value = value
                        .into_string()
                        .map_err(|_| "filter-desktop value must be valid UTF-8")?;
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
                    .map_err(|_| "Invalid match mode. Use 'exact' or 'fuzzy'")?;
            }
            Long("prefix-depth") => {
                let depth = value_as_string(parser, "Prefix depth must be valid UTF-8")?;
                default.prefix_depth =
                    depth.parse::<usize>().map_err(|_| "Invalid prefix depth")?;
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
                usage();
            }
            Short('H') | Long("help") => {
                detailed_usage();
            }
            Short('V') | Long("version") => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            Value(_) => {
                return Err(arg.unexpected());
            }
            _ => report_unknown_argument(arg),
        }
    }

    Ok(cli_launch_methods)
}

fn value_as_string(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<String, lexopt::Error> {
    parser
        .value()?
        .into_string()
        .map_err(|_| lexopt::Error::from(error_message))
}

fn parse_column_list(
    parser: &mut lexopt::Parser,
    error_message: &'static str,
) -> Result<Vec<usize>, lexopt::Error> {
    let columns = value_as_string(parser, "Column specification must be valid UTF-8")?;
    columns
        .split(',')
        .map(|part| part.trim().parse::<usize>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| error_message.into())
}

fn report_config_defaults_error(error: ConfigDefaultsError) -> ! {
    eprintln!("Error: {error}");
    if matches!(error, ConfigDefaultsError::MultipleLaunchMethods) {
        eprintln!("Available methods: --systemd-run, --uwsm");
    }
    std::process::exit(1);
}

fn report_unknown_argument(arg: lexopt::Arg<'_>) -> ! {
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

    eprintln!("Error: {error_msg}");
    eprintln!();
    eprintln!("Quick help:");
    eprintln!("  -c, --config <FILE>    Read config from FILE");
    eprintln!("  -p, --program <NAME>   Launch one app immediately and skip the TUI");
    eprintln!("  -ss <SEARCH>           Pre-fill the search box; place this last");
    eprintln!("  --dmenu                Read choices from stdin and print the selection");
    eprintln!("  --cclip                Browse clipboard history and copy the selection");
    eprintln!("  --no-exec              Print the selected item instead of launching it");
    eprintln!("  -r, --replace          Replace an existing fsel/cclip instance");
    eprintln!("  -d, --detach           Start launched apps without keeping the terminal attached");
    eprintln!("  -v, --verbose          Print more diagnostics; repeat as -vv or -vvv");
    eprintln!("  -h                     Show the short summary");
    eprintln!("  -H, --help             Show the full option tree");
    eprintln!("  -V, --version          Print the version and exit");
    eprintln!();
    eprintln!("Run 'fsel -h' for a summary or 'fsel --help' for the full option tree.");
    std::process::exit(1);
}
