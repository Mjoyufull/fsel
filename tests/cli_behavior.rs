//! Regression checks for top-level CLI behavior that must remain stable.

use std::path::PathBuf;
use std::process::Command;

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_fsel")
}

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
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
