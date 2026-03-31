use super::types::Opts;

pub(super) fn set_launch_prefix(opts: &mut Opts, prefix: Vec<String>) {
    clear_launch_method(opts);
    opts.launch_prefix_set = !prefix.is_empty();
    opts.launch_prefix = prefix;
}

pub(super) fn set_systemd_run(opts: &mut Opts) {
    clear_launch_method(opts);
    opts.systemd_run = true;
    opts.launch_prefix = systemd_run_prefix();
}

pub(super) fn set_uwsm(opts: &mut Opts) {
    clear_launch_method(opts);
    opts.uwsm = true;
    opts.launch_prefix = uwsm_prefix();
}

pub(super) fn active_launch_method_count(opts: &Opts) -> usize {
    [opts.systemd_run, opts.uwsm, opts.launch_prefix_set]
        .iter()
        .filter(|&&enabled| enabled)
        .count()
}

pub(super) fn parse_launch_prefix(value: &str) -> Result<Vec<String>, &'static str> {
    let prefix =
        shell_words::split(value).map_err(|_| "Launch prefix must use valid shell syntax")?;
    if prefix.is_empty() {
        return Err("Launch prefix cannot be empty");
    }
    Ok(prefix)
}

fn systemd_run_prefix() -> Vec<String> {
    vec!["systemd-run".into(), "--user".into(), "--scope".into()]
}

fn uwsm_prefix() -> Vec<String> {
    vec!["uwsm".into(), "app".into(), "--".into()]
}

fn clear_launch_method(opts: &mut Opts) {
    opts.systemd_run = false;
    opts.uwsm = false;
    opts.launch_prefix_set = false;
    opts.launch_prefix.clear();
}

#[cfg(test)]
mod tests {
    use super::{
        active_launch_method_count, parse_launch_prefix, set_launch_prefix, set_systemd_run,
    };
    use crate::cli::Opts;

    #[test]
    fn parse_launch_prefix_supports_shell_words() {
        assert_eq!(
            parse_launch_prefix("runapp --tag \"gui apps\" --").unwrap(),
            ["runapp", "--tag", "gui apps", "--"]
        );
    }

    #[test]
    fn parse_launch_prefix_rejects_empty_values() {
        assert!(parse_launch_prefix("").is_err());
    }

    #[test]
    fn later_launch_method_overrides_previous_state() {
        let mut opts = Opts::default();
        set_systemd_run(&mut opts);
        set_launch_prefix(&mut opts, vec!["runapp".into(), "--".into()]);
        assert_eq!(active_launch_method_count(&opts), 1);
        assert!(!opts.systemd_run);
        assert!(!opts.uwsm);
        assert!(opts.launch_prefix_set);
    }
}
