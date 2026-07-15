use jwalk::WalkDir;
use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

mod theme;

use theme::detect_icon_theme;

const ICON_EXTENSIONS: [&str; 2] = ["svg", "png"];
const MAX_THEME_DEPTH: usize = 16;

/// Resolves desktop-entry icon names through the active XDG icon theme.
pub(crate) struct IconResolver {
    theme: String,
    size: u16,
    icon_roots: Vec<PathBuf>,
    pixmap_roots: Vec<PathBuf>,
    cache: HashMap<String, Option<PathBuf>>,
}

impl IconResolver {
    /// Build a resolver from the process environment and optional configured theme.
    pub(crate) fn from_environment(theme: Option<&str>, size: u16) -> Self {
        let home = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf());
        let config_home = env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".config")));
        let data_home = env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home.as_ref().map(|path| path.join(".local/share")));

        let mut icon_roots = Vec::new();
        if let Some(data_home) = data_home {
            push_unique(&mut icon_roots, data_home.join("icons"));
        }
        if let Some(home) = &home {
            push_unique(&mut icon_roots, home.join(".icons"));
        }

        let data_dirs =
            env::var("XDG_DATA_DIRS").unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());
        let mut pixmap_roots = Vec::new();
        for data_dir in data_dirs.split(':').filter(|entry| !entry.is_empty()) {
            let data_dir = PathBuf::from(data_dir);
            push_unique(&mut icon_roots, data_dir.join("icons"));
            push_unique(&mut pixmap_roots, data_dir.join("pixmaps"));
        }

        let theme = theme
            .filter(|value| !value.trim().is_empty())
            .map(str::to_string)
            .or_else(|| detect_icon_theme(config_home.as_deref(), home.as_deref()))
            .unwrap_or_else(|| "hicolor".to_string());

        Self {
            theme,
            size,
            icon_roots,
            pixmap_roots,
            cache: HashMap::new(),
        }
    }

    /// Resolve an absolute path or themed icon name to an existing image file.
    pub(crate) fn resolve(&mut self, icon: &str) -> Option<PathBuf> {
        if let Some(path) = absolute_icon_path(icon) {
            return Some(path);
        }
        if let Some(cached) = self.cache.get(icon) {
            return cached.clone();
        }

        let icon_name = strip_icon_extension(icon);
        let resolved = self
            .theme_chain()
            .into_iter()
            .find_map(|theme| self.find_in_theme(&theme, icon_name))
            .or_else(|| self.find_in_pixmaps(icon_name));
        self.cache.insert(icon.to_string(), resolved.clone());
        resolved
    }

    fn theme_chain(&self) -> Vec<String> {
        let mut queue = VecDeque::from([self.theme.clone()]);
        let mut seen = HashSet::new();
        let mut themes = Vec::new();

        while let Some(theme) = queue.pop_front() {
            if themes.len() >= MAX_THEME_DEPTH || !seen.insert(theme.clone()) {
                continue;
            }
            for root in &self.icon_roots {
                if let Some(metadata) = read_theme_metadata(&root.join(&theme)) {
                    queue.extend(metadata.inherits);
                }
            }
            themes.push(theme);
        }
        if seen.insert("hicolor".to_string()) {
            themes.push("hicolor".to_string());
        }
        themes
    }

    fn find_in_theme(&self, theme: &str, icon: &str) -> Option<PathBuf> {
        for root in &self.icon_roots {
            let theme_root = root.join(theme);
            if !theme_root.is_dir() {
                continue;
            }

            let mut candidates = Vec::new();
            if let Some(metadata) = read_theme_metadata(&theme_root) {
                for directory in metadata.directories {
                    collect_named_candidates(&theme_root.join(directory), icon, &mut candidates);
                }
            }
            if candidates.is_empty() {
                for entry in WalkDir::new(&theme_root)
                    .min_depth(1)
                    .max_depth(5)
                    .into_iter()
                    .filter_map(Result::ok)
                {
                    let path = entry.path();
                    if path.is_file() && has_icon_name(&path, icon) {
                        candidates.push(path);
                    }
                }
            }
            if let Some(path) = best_candidate(candidates, self.size) {
                return Some(path);
            }
        }
        None
    }

    fn find_in_pixmaps(&self, icon: &str) -> Option<PathBuf> {
        for root in &self.pixmap_roots {
            for extension in ICON_EXTENSIONS {
                let path = root.join(format!("{icon}.{extension}"));
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        None
    }
}

struct ThemeMetadata {
    directories: Vec<PathBuf>,
    inherits: Vec<String>,
}

fn read_theme_metadata(theme_root: &Path) -> Option<ThemeMetadata> {
    let contents = fs::read_to_string(theme_root.join("index.theme")).ok()?;
    let mut in_icon_theme = false;
    let mut directories = Vec::new();
    let mut inherits = Vec::new();

    for line in contents.lines().map(str::trim) {
        if line.starts_with('[') {
            in_icon_theme = line.eq_ignore_ascii_case("[Icon Theme]");
            continue;
        }
        if !in_icon_theme {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        match key.trim() {
            "Directories" | "ScaledDirectories" => directories.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(PathBuf::from),
            ),
            "Inherits" => inherits.extend(
                value
                    .split(',')
                    .map(str::trim)
                    .filter(|entry| !entry.is_empty())
                    .map(str::to_string),
            ),
            _ => {}
        }
    }

    Some(ThemeMetadata {
        directories,
        inherits,
    })
}

fn collect_named_candidates(directory: &Path, icon: &str, candidates: &mut Vec<PathBuf>) {
    for extension in ICON_EXTENSIONS {
        let path = directory.join(format!("{icon}.{extension}"));
        if path.is_file() {
            candidates.push(path);
        }
    }
}

fn best_candidate(mut candidates: Vec<PathBuf>, requested_size: u16) -> Option<PathBuf> {
    candidates.sort_by_key(|path| candidate_score(path, requested_size));
    candidates.into_iter().next()
}

fn candidate_score(path: &Path, requested_size: u16) -> (u16, u8) {
    let size_distance = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .find_map(parse_directory_size)
        .map_or(0, |size| size.abs_diff(requested_size));
    let extension_rank = match path.extension().and_then(|value| value.to_str()) {
        Some("svg") => 0,
        Some("png") => 1,
        _ => 2,
    };
    (size_distance, extension_rank)
}

fn parse_directory_size(component: &str) -> Option<u16> {
    let leading = component.split('x').next()?;
    leading.parse().ok()
}

fn has_icon_name(path: &Path, icon: &str) -> bool {
    path.file_stem().and_then(|stem| stem.to_str()) == Some(icon)
        && path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| ICON_EXTENSIONS.contains(&extension))
}

fn strip_icon_extension(icon: &str) -> &str {
    ICON_EXTENSIONS
        .iter()
        .find_map(|extension| icon.strip_suffix(&format!(".{extension}")))
        .unwrap_or(icon)
}

fn absolute_icon_path(icon: &str) -> Option<PathBuf> {
    let path = PathBuf::from(icon);
    (path.is_absolute() && path.is_file()).then_some(path)
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.contains(&path) {
        paths.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::IconResolver;
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir() -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("fsel-icons-{unique}"));
        fs::create_dir_all(&path).expect("temporary icon root should be created");
        path
    }

    #[test]
    fn resolves_inherited_theme_icons_at_the_requested_size() {
        let root = temp_dir();
        let selected_theme = root.join("Selected");
        let inherited_theme = root.join("Inherited");
        fs::create_dir_all(&selected_theme).expect("selected theme should be created");
        fs::write(
            selected_theme.join("index.theme"),
            "[Icon Theme]\nInherits=Inherited\n",
        )
        .expect("selected theme metadata should be written");
        fs::create_dir_all(inherited_theme.join("32x32/apps"))
            .expect("32px directory should be created");
        fs::create_dir_all(inherited_theme.join("128x128/apps"))
            .expect("128px directory should be created");
        fs::write(
            inherited_theme.join("index.theme"),
            "[Icon Theme]\nDirectories=32x32/apps,128x128/apps\n",
        )
        .expect("inherited theme metadata should be written");
        fs::write(inherited_theme.join("32x32/apps/editor.png"), b"small")
            .expect("small icon should be written");
        let expected = inherited_theme.join("128x128/apps/editor.png");
        fs::write(&expected, b"large").expect("large icon should be written");

        let mut resolver = IconResolver {
            theme: "Selected".to_string(),
            size: 128,
            icon_roots: vec![root.clone()],
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve("editor"), Some(expected));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn absolute_paths_bypass_theme_lookup() {
        let root = temp_dir();
        let icon = root.join("custom.png");
        fs::write(&icon, b"icon").expect("absolute icon should be written");
        let mut resolver = IconResolver {
            theme: "hicolor".to_string(),
            size: 64,
            icon_roots: Vec::new(),
            pixmap_roots: Vec::new(),
            cache: std::collections::HashMap::new(),
        };

        assert_eq!(resolver.resolve(icon.to_str().unwrap()), Some(icon.clone()));
        let _ = fs::remove_dir_all(root);
    }
}
