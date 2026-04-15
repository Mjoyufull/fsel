use directories::ProjectDirs;
use eyre::{Result, eyre};
use std::fs;
use std::path::PathBuf;

fn runtime_project_dirs() -> Result<ProjectDirs> {
    ProjectDirs::from("ch", "forkbomb9", env!("CARGO_PKG_NAME"))
        .ok_or_else(|| eyre!("can't find runtime dirs for {}", env!("CARGO_PKG_NAME")))
}

fn ensure_dir(path: &PathBuf) -> Result<()> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(eyre!(
            "runtime path exists but is not a directory: {}",
            path.display()
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            fs::create_dir_all(path)?;
            Ok(())
        }
        Err(error) => Err(error.into()),
    }
}

pub(crate) fn runtime_data_dir() -> Result<PathBuf> {
    let dir = runtime_project_dirs()?.data_local_dir().to_path_buf();
    ensure_dir(&dir)?;
    Ok(dir)
}

pub(crate) fn runtime_cache_dir() -> Result<PathBuf> {
    let dir = runtime_project_dirs()?.cache_dir().to_path_buf();
    ensure_dir(&dir)?;
    Ok(dir)
}

pub(crate) fn history_db_path() -> Result<PathBuf> {
    let mut path = runtime_data_dir()?;
    path.push("hist_db.redb");
    Ok(path)
}

pub(crate) fn launcher_lock_path() -> Result<PathBuf> {
    let mut path = runtime_data_dir()?;
    path.push("fsel-fsel.lock");
    Ok(path)
}

pub(crate) fn cclip_lock_path() -> Result<PathBuf> {
    let mut path = runtime_cache_dir()?;
    path.push("fsel-cclip.lock");
    Ok(path)
}

pub(crate) fn legacy_config_file_path() -> Option<PathBuf> {
    ProjectDirs::from("", "", "fsel").map(|proj_dirs| {
        let mut path = proj_dirs.config_dir().to_path_buf();
        path.push("config.toml");
        path
    })
}
