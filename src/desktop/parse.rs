use super::{Action, App};
use eyre::eyre;
use std::convert::AsRef;
use std::env;

static LOCALE_CACHE: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();

fn get_locale() -> &'static [String] {
    LOCALE_CACHE.get_or_init(|| {
        let mut locales = Vec::new();
        let locale_var = env::var("LC_MESSAGES")
            .or_else(|_| env::var("LANG"))
            .or_else(|_| env::var("LC_ALL"))
            .unwrap_or_else(|_| "C".to_string());

        if locale_var != "C" && locale_var != "POSIX" {
            let base_locale = locale_var.split('.').next().unwrap_or(&locale_var);
            locales.push(base_locale.to_string());

            if let Some(language) = base_locale.split('_').next()
                && language != base_locale
            {
                locales.push(language.to_string());
            }
        }

        locales
    })
}

#[inline]
fn parse_semicolon_list(value: &str) -> Vec<String> {
    if value.is_empty() {
        return Vec::new();
    }

    value
        .split(';')
        .filter_map(|part| {
            let trimmed = part.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        })
        .collect()
}

fn get_localized_value(
    key: &str,
    value: &str,
    existing_value: &Option<String>,
    locales: &[String],
) -> Option<String> {
    if let Some(bracket_pos) = key.find('[') {
        let locale_part = &key[bracket_pos + 1..key.len() - 1];
        if existing_value.is_none() && locales.iter().any(|locale| locale == locale_part) {
            return Some(value.to_string());
        }
        None
    } else if existing_value.is_none() {
        Some(value.to_string())
    } else {
        None
    }
}

impl App {
    /// Parse an application from the main `[Desktop Entry]` section.
    pub fn parse<T: AsRef<str>>(contents: T, filter_desktop: bool) -> eyre::Result<App> {
        Self::parse_section(contents, None, filter_desktop)
    }

    fn parse_section<T: AsRef<str>>(
        contents: T,
        action: Option<&Action>,
        filter_desktop: bool,
    ) -> eyre::Result<App> {
        let contents = contents.as_ref();
        let locales = get_locale();
        let pattern = match action {
            Some(action) if action.name.is_empty() => return Err(eyre!("Action is empty")),
            Some(action) => format!("[Desktop Action {}]", action.name),
            None => "[Desktop Entry]".to_string(),
        };

        let mut name = None;
        let mut generic_name = None;
        let mut exec = None;
        let mut description = None;
        let mut keywords = Vec::with_capacity(4);
        let mut categories = Vec::with_capacity(2);
        let mut mime_types = Vec::new();
        let mut icon = None;
        let mut terminal_exec = false;
        let mut path = None;
        let mut only_show_in = Vec::new();
        let mut not_show_in = Vec::new();
        let mut hidden = false;
        let mut no_display = false;
        let mut startup_notify = false;
        let mut startup_wm_class = None;
        let mut try_exec = None;
        let mut entry_type = None;
        let mut actions = None;
        let mut in_target_section = false;

        for line in contents.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && in_target_section && line != pattern {
                break;
            }

            if line == pattern {
                in_target_section = true;
                continue;
            }

            if in_target_section && let Some((key, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                match key.split('[').next().unwrap_or(key) {
                    "Type" => entry_type = Some(value.to_string()),
                    "Name" => {
                        if let Some(localized) = get_localized_value(key, value, &name, locales) {
                            name = Some(match action {
                                Some(action) => format!("{} ({localized})", action.from),
                                None => localized,
                            });
                        }
                    }
                    "GenericName" => {
                        if let Some(localized) =
                            get_localized_value(key, value, &generic_name, locales)
                        {
                            generic_name = Some(localized);
                        }
                    }
                    "Comment" => {
                        if let Some(localized) =
                            get_localized_value(key, value, &description, locales)
                        {
                            description = Some(localized);
                        }
                    }
                    "Keywords" if keywords.is_empty() => keywords = parse_semicolon_list(value),
                    "Categories" if categories.is_empty() => {
                        categories = parse_semicolon_list(value);
                    }
                    "MimeType" if mime_types.is_empty() => {
                        mime_types = parse_semicolon_list(value);
                    }
                    "Icon" if icon.is_none() => icon = Some(value.to_string()),
                    "Terminal" => terminal_exec = value.eq_ignore_ascii_case("true"),
                    "Exec" if exec.is_none() => {
                        let cleaned = value
                            .split_whitespace()
                            .filter(|part| !part.starts_with('%'))
                            .collect::<Vec<_>>()
                            .join(" ");
                        exec = Some(cleaned);
                    }
                    "Path" if path.is_none() => path = Some(value.to_string()),
                    "TryExec" if try_exec.is_none() => try_exec = Some(value.to_string()),
                    "OnlyShowIn" if only_show_in.is_empty() => {
                        only_show_in = parse_semicolon_list(value);
                    }
                    "NotShowIn" if not_show_in.is_empty() => {
                        not_show_in = parse_semicolon_list(value);
                    }
                    "Hidden" => hidden = value.eq_ignore_ascii_case("true"),
                    "NoDisplay" => no_display = value.eq_ignore_ascii_case("true"),
                    "StartupNotify" => startup_notify = value.eq_ignore_ascii_case("true"),
                    "StartupWMClass" if startup_wm_class.is_none() => {
                        startup_wm_class = Some(value.to_string());
                    }
                    "Actions" if actions.is_none() && action.is_none() => {
                        actions = Some(parse_semicolon_list(value));
                    }
                    _ => {}
                }
            }
        }

        let entry_type = entry_type.unwrap_or_else(|| "Application".to_string());
        if entry_type != "Application" {
            return Err(eyre!("Not an Application type desktop entry"));
        }

        let name = name.unwrap_or_else(|| "Unknown".to_string());
        let command = exec.ok_or_else(|| eyre!("Missing required Exec field"))?;

        if hidden || (filter_desktop && no_display) {
            return Err(eyre!("Application is hidden"));
        }

        Ok(App {
            score: 0,
            history: 0,
            pinned: false,
            last_access: None,
            name,
            command,
            description: description.unwrap_or_default(),
            generic_name,
            keywords,
            categories,
            mime_types,
            icon,
            is_terminal: terminal_exec,
            path,
            only_show_in,
            not_show_in,
            hidden,
            startup_notify,
            startup_wm_class,
            try_exec,
            entry_type,
            desktop_id: None,
            actions,
            breakdown: None,
        })
    }

    /// Parse an application from a named `[Desktop Action ...]` section.
    pub(super) fn parse_action<T: AsRef<str>>(
        contents: T,
        action: &Action,
        filter_desktop: bool,
    ) -> eyre::Result<App> {
        Self::parse_section(contents, Some(action), filter_desktop)
    }
}

#[cfg(test)]
mod tests {
    use super::{Action, App};

    #[test]
    fn parse_strips_exec_field_codes() {
        let app = App::parse(
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=/usr/bin/editor %F %u\nComment=Editor",
            false,
        )
        .expect("desktop entry should parse");

        assert_eq!(app.command, "/usr/bin/editor");
    }

    #[test]
    fn parse_action_uses_action_section_name() {
        let action = Action::default().name("OpenWindow").from("Editor");
        let app = App::parse_action(
            "[Desktop Entry]\nType=Application\nName=Editor\nExec=/usr/bin/editor\nActions=OpenWindow;\n\n[Desktop Action OpenWindow]\nName=Open Window\nExec=/usr/bin/editor --new-window",
            &action,
            false,
        )
        .expect("desktop action should parse");

        assert_eq!(app.name, "Editor (Open Window)");
        assert_eq!(app.command, "/usr/bin/editor --new-window");
    }

    #[test]
    fn parse_allows_no_display_when_desktop_filter_is_disabled() {
        let app = App::parse(
            "[Desktop Entry]\nType=Application\nName=Hidden Tool\nExec=/usr/bin/hidden-tool\nNoDisplay=true",
            false,
        )
        .expect("NoDisplay entries should still parse when desktop filtering is disabled");

        assert_eq!(app.name, "Hidden Tool");
    }
}
