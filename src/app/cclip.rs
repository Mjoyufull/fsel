use std::process::ExitCode;

use crate::cli;
use crate::modes;

pub(crate) fn run(cli: &cli::Opts) -> eyre::Result<ExitCode> {
    if !modes::cclip::check_cclip_available() {
        eprintln!("error: cclip is not installed or not in PATH");
        eprintln!("install cclip from: https://github.com/heather7283/cclip");
        return Ok(ExitCode::from(1));
    }

    let lock_path = super::paths::cclip_lock_path()?;
    let is_non_interactive = cli.cclip_clear_tags || cli.cclip_tag_list || cli.cclip_wipe_tags;
    let _session = if is_non_interactive {
        None
    } else {
        Some(modes::cclip::CclipSession::start(&lock_path, cli.replace)?)
    };

    let rt = tokio::runtime::Runtime::new()?;
    if let Err(error) = rt.block_on(modes::cclip::run(cli)) {
        if let Err(database_error) = modes::cclip::check_cclip_database() {
            eprintln!("error: {}", database_error);
            eprintln!("\nto use cclip mode, you need to:");
            eprintln!("1. start cclipd daemon:");
            eprintln!(
                "   cclipd -s 2 -t \"image/png\" -t \"image/*\" -t \"text/plain;charset=utf-8\" -t \"text/*\" -t \"*\""
            );
            eprintln!("2. copy some stuff to build up history");
            eprintln!("\nfor more info: https://github.com/heather7283/cclip");
            return Ok(ExitCode::from(1));
        }

        return Err(error);
    }

    Ok(ExitCode::SUCCESS)
}
