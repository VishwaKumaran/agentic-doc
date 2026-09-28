use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_scan_unconfigured_fails() {
    let ws = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("scan")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: agentic-doc is not configured in this directory.",
        ));
}

#[test]
fn test_cli_scan_success_and_json() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    std::fs::write(proj.path().join("main.py"), "def run(): pass\n").unwrap();

    // Setup
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // Scan
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(ws.path())
        .arg("scan")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project scanned."));

    assert!(ws.path().join(".agentic-doc/snapshots").exists());

    // Scan JSON
    let mut scan_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = scan_json_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("scan")
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "scan");
    assert_eq!(json_val["schema_version"], 1);
    assert_eq!(json_val["files"], 1);
    assert_eq!(json_val["elements"], 1);
}

#[test]
fn test_cli_scan_zero_files_emits_warning_on_stderr() {
    // Étape 18 : projet sans fichier Python → avertissement sur stderr, code 0.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    // Aucun fichier .py
    std::fs::write(proj.path().join("README.md"), "# Projet\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(ws.path())
        .arg("scan")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("Warning: 0 source file analyzed."))
        .stderr(predicate::str::contains(
            "Hint: the project may use a language that is not supported yet",
        ));
}

#[test]
fn test_cli_scan_zero_files_json_stdout_intact() {
    // Étape 18 : avec --json, stdout reste un JSON valide même quand 0 fichier.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("README.md"), "# Projet\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut scan_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = scan_json_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("scan")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("Warning: 0 source file analyzed."));

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "scan");
    assert_eq!(json_val["files"], 0);
}
