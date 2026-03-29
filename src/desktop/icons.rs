<<<<<<< HEAD
use std::path::PathBuf;

/// Resolve an icon name to its filesystem path using XDG theme specs
#[allow(dead_code)]
pub fn lookup(name: &str, size: u16) -> Option<PathBuf> {
    // Handle absolute paths if provided directly in the Icon field
=======
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

static ICON_THEME: OnceLock<String> = OnceLock::new();
static PATH_CACHE: OnceLock<Mutex<HashMap<(String, u16), Option<PathBuf>>>> = OnceLock::new();

fn cached_theme() -> &'static str {
    ICON_THEME.get_or_init(|| {
        linicon_theme::get_icon_theme().unwrap_or_else(|| "hicolor".to_string())
    })
}

pub fn lookup(name: &str, size: u16) -> Option<PathBuf> {
    // 1. Check for absolute paths FIRST before stripping extensions
>>>>>>> 10356ad (feat: implement asynchronous icon loading and rendering in app launcher using freedesktop-icons and ratatui-image)
    if name.starts_with('/') {
        let path = PathBuf::from(name);
        if path.exists() {
            return Some(path);
        }
    }

<<<<<<< HEAD

    // Get icon theme
    let theme = linicon_theme::get_icon_theme().unwrap_or_else(|| "hicolor".to_string());

    // Lookup using freedesktop
    freedesktop_icons::lookup(name)
        .with_size(size)
        .with_theme(&theme)
        .find()
        .or_else(|| {
            // Check 'hicolor' if theme fails
=======
    // 2. Strip extensions for theme lookup
    let name = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".svg"))
        .or_else(|| name.strip_suffix(".xpm"))
        .unwrap_or(name);

    let cache = PATH_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let key = (name.to_string(), size);

    // Check cache first
    {
        let lock = cache.lock().unwrap();
        if let Some(cached) = lock.get(&key) {
            return cached.clone();
        }
    }

    // Expensive XDG traversal
    let theme = cached_theme();
    let result = freedesktop_icons::lookup(name)
        .with_size(size)
        .with_theme(theme)
        .find()
        .or_else(|| {
>>>>>>> 10356ad (feat: implement asynchronous icon loading and rendering in app launcher using freedesktop-icons and ratatui-image)
            if theme != "hicolor" {
                freedesktop_icons::lookup(name)
                    .with_size(size)
                    .with_theme("hicolor")
                    .find()
            } else {
                None
            }
<<<<<<< HEAD
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
=======
        });

    cache.lock().unwrap().insert(key, result.clone());
    result
>>>>>>> 10356ad (feat: implement asynchronous icon loading and rendering in app launcher using freedesktop-icons and ratatui-image)
}
