use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_setup_success() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration created."));

    assert!(ws.path().join(".agentic-doc/config.json").exists());
}

#[test]
fn test_cli_setup_already_configured_and_force() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // Re-run without force -> fail (exit 1)
    let mut cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    cmd2.current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: already configured. Use --force to overwrite.",
        ));

    // Re-run with force -> success
    let mut cmd3 = Command::cargo_bin("agentic-doc").unwrap();
    cmd3.current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .arg("--force")
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration overwritten."));
}

#[test]
fn test_cli_setup_json() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "setup");
    assert_eq!(json_val["schema_version"], 1);
    assert_eq!(json_val["overwritten"], false);
}
