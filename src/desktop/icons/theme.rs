use std::fs;
use std::path::{Path, PathBuf};

struct ThemeSetting {
    path: PathBuf,
    section: Option<&'static str>,
    key: &'static str,
}

pub(super) fn detect_icon_theme(config_home: Option<&Path>, home: Option<&Path>) -> Option<String> {
    let mut settings = Vec::new();
    if let Some(config_home) = config_home {
        settings.extend([
            ThemeSetting {
                path: config_home.join("gtk-4.0/settings.ini"),
                section: None,
                key: "gtk-icon-theme-name",
            },
            ThemeSetting {
                path: config_home.join("gtk-3.0/settings.ini"),
                section: None,
                key: "gtk-icon-theme-name",
            },
            ThemeSetting {
                path: config_home.join("kdeglobals"),
                section: Some("Icons"),
                key: "Theme",
            },
            ThemeSetting {
                path: config_home.join("lxqt/lxqt.conf"),
                section: Some("General"),
                key: "icon_theme",
            },
        ]);
    }
    if let Some(home) = home {
        settings.push(ThemeSetting {
            path: home.join(".gtkrc-2.0"),
            section: None,
            key: "gtk-icon-theme-name",
        });
    }

    settings.into_iter().find_map(|setting| {
        let contents = fs::read_to_string(setting.path).ok()?;
        find_setting(&contents, setting.section, setting.key)
    })
}

fn find_setting(contents: &str, required_section: Option<&str>, key: &str) -> Option<String> {
    let mut current_section = None;
    for line in contents.lines().map(str::trim) {
        if let Some(section) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            current_section = Some(section.trim());
            continue;
        }
        if required_section.is_some() && current_section != required_section {
            continue;
        }

        let Some((candidate, value)) = line.split_once('=') else {
            continue;
        };
        let value = value.trim().trim_matches(['\'', '"']).trim();
        if candidate.trim().eq_ignore_ascii_case(key) && !value.is_empty() {
            return Some(value.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::find_setting;

    #[test]
    fn reads_kde_icon_theme_only_from_icons_section() {
        let contents = "[General]\nTheme=Wrong\n[Icons]\nTheme=Breeze-Dark\n";

        assert_eq!(
            find_setting(contents, Some("Icons"), "Theme").as_deref(),
            Some("Breeze-Dark")
        );
    }

    #[test]
    fn reads_quoted_gtk_icon_theme() {
        let contents = "[Settings]\ngtk-icon-theme-name = 'Papirus-Dark'\n";

        assert_eq!(
            find_setting(contents, None, "gtk-icon-theme-name").as_deref(),
            Some("Papirus-Dark")
        );
    }
}
