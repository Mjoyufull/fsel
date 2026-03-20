use std::path::PathBuf;

/// Resolve an icon name to its filesystem path using XDG theme specs
pub fn lookup(name: &str, size: u16) -> Option<PathBuf> {
    // Handle absolute paths if provided directly in the Icon field
    if name.starts_with('/') {
        let path = PathBuf::from(name);
        if path.exists() {
            return Some(path);
        }
    }

    // Get icon theme
    let theme = linicon_theme::get_icon_theme().unwrap_or_else(|| "hicolor".to_string());

    // Lookup using freedesktop
    freedesktop_icons::lookup(name)
        .with_size(size)
        .with_theme(&theme)
        .find()
        .or_else(|| {
            // Check 'hicolor' if theme fails
            if theme != "hicolor" {
                freedesktop_icons::lookup(name)
                    .with_size(size)
                    .with_theme("hicolor")
                    .find()
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_icon_lookup() {
        let path = lookup("firefox", 32);
        println!("Found firefox icon at: {:?}", path);
    }

    #[test]
    fn test_icon_fallback() {
        let path = lookup("system-file-manager", 32);
        println!("Found file manager icon at: {:?}", path);
    }
}
