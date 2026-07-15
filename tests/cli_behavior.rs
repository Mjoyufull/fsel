//! Regression checks for top-level CLI behavior that must remain stable.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_fsel")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}

fn isolated_runtime_dir(label: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();
    let dir =
        std::env::temp_dir().join(format!("fsel-cli-{label}-{}-{unique}", std::process::id()));
    fs::create_dir_all(&dir).expect("isolated runtime directory should be created");
    dir
}

fn isolated_command(runtime_dir: &Path) -> Command {
    let mut command = Command::new(binary());
    command
        .env("HOME", runtime_dir)
        .env("XDG_CACHE_HOME", runtime_dir.join("cache"))
        .env("XDG_CONFIG_HOME", runtime_dir.join("config"))
        .env("XDG_DATA_HOME", runtime_dir.join("data"));
    command
}

#[test]
fn version_flag_exits_successfully() {
    let output = Command::new(binary())
        .arg("--version")
        .output()
        .expect("test binary should run");

    assert!(
        output.status.success(),
        "expected --version to succeed, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        env!("CARGO_PKG_VERSION")
    );
}

#[test]
fn short_help_exits_successfully() {
    let output = Command::new(binary())
        .arg("-h")
        .output()
        .expect("test binary should run");

    assert!(
        output.status.success(),
        "expected -h to succeed, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "short help should include Usage:, got: {stdout}"
    );
    assert!(
        stdout.contains("Core Modes"),
        "short help should include Core Modes, got: {stdout}"
    );
}

#[test]
fn tag_requires_cclip_mode() {
    let output = Command::new(binary())
        .args(["--tag", "list"])
        .output()
        .expect("test binary should run");

    assert!(
        !output.status.success(),
        "expected --tag without --cclip to fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: --tag requires --cclip mode"),
        "missing expected cclip validation error: {stderr}"
    );
}

#[test]
fn legacy_config_fixture_still_loads() {
    let output = Command::new(binary())
        .args([
            "--config",
            fixture_path("legacy-config.toml")
                .to_str()
                .expect("fixture path should be valid UTF-8"),
            "--version",
        ])
        .output()
        .expect("test binary should run");

    assert!(
        output.status.success(),
        "legacy config fixture should still load, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn missing_explicit_config_path_fails() {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("missing-config.toml");

    let output = Command::new(binary())
        .args([
            "--config",
            path.to_str().expect("fixture path should be valid UTF-8"),
        ])
        .output()
        .expect("test binary should run");

    assert!(
        !output.status.success(),
        "missing explicit config path should fail"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error loading configuration:"),
        "missing config error should be reported, got: {stderr}"
    );
    assert!(
        stderr.contains("Config file not found"),
        "missing config path should be included, got: {stderr}"
    );
}

#[test]
fn list_hidden_initializes_an_empty_store() {
    let runtime_dir = isolated_runtime_dir("hidden-list");
    let output = isolated_command(&runtime_dir)
        .arg("--list-hidden")
        .output()
        .expect("test binary should run");

    assert!(
        output.status.success(),
        "expected --list-hidden to succeed, stderr was: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "No manually hidden entries."
    );

    fs::remove_dir_all(runtime_dir).expect("isolated runtime directory should be removed");
}
