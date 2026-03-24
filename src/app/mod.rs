use eyre::WrapErr;

use crate::cli;
use crate::modes;
use crate::ui::terminal;

mod cclip;

pub(crate) fn run() -> eyre::Result<()> {
    let cli = cli::parse()?;

    if cli.dmenu_mode {
        return modes::dmenu::run(&cli);
    }

    if cli.cclip_mode {
        return cclip::run(&cli);
    }

    let rt = tokio::runtime::Runtime::new().wrap_err("Failed to create tokio runtime")?;
    rt.block_on(modes::app_launcher::run(cli))
}

pub(crate) fn cleanup_after_error() {
    let _ = terminal::shutdown_terminal(false);
}
