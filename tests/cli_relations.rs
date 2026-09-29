use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_relations_no_subcommand_fails_code_2() {
    let ws = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_cli_relations_invalid_confidence_fails_code_2() {
    let ws = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("add")
        .arg("src/auth.py::login")
        .arg("doc.md")
        .arg("--confidence")
        .arg("invalid")
        .assert()
        .failure()
        .code(2);
}

#[test]
fn test_cli_relations_list_empty_returns_no_relations_declared() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("list")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("No relations declared."));
}

#[test]
fn test_cli_relations_add_unknown_source_fails_code_1() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("main.py"), "def run(): pass\n").unwrap();
    std::fs::write(ws.path().join("doc.md"), "# Doc\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("add")
        .arg("main.py::unknown")
        .arg("doc.md")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: relation source not found: main.py::unknown",
        ));

    assert!(!ws.path().join(".agentic-doc/relations.json").exists());
}

#[test]
fn test_cli_relations_add_and_list_success_and_json() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("main.py"), "def run(): pass\n").unwrap();
    std::fs::write(ws.path().join("doc.md"), "# Doc\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // 1. Add
    let mut add_cmd = Command::cargo_bin("agentic-doc").unwrap();
    add_cmd
        .current_dir(ws.path())
        .arg("relations")
        .arg("add")
        .arg("main.py::run")
        .arg("doc.md")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "Relation added: main.py::run → doc.md",
        ));

    assert!(ws.path().join(".agentic-doc/relations.json").exists());

    // 2. Add replayed -> already declared
    let mut add_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    add_cmd2
        .current_dir(ws.path())
        .arg("relations")
        .arg("add")
        .arg("main.py::run")
        .arg("doc.md")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "Relation already declared: main.py::run → doc.md",
        ));

    // 3. List --json
    let mut list_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = list_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("relations")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "relations");
    assert_eq!(json_val["schema_version"], 1);
    assert_eq!(json_val["count"], 1);
    assert_eq!(json_val["relations"][0]["resolution"], "ok");
}

#[test]
fn test_cli_relations_remove_absent_pair_code_0() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("remove")
        .arg("main.py::run")
        .arg("doc.md")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("Nothing to remove."));
}

#[test]
fn test_cli_relations_dry_run_does_not_modify_disk() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("main.py"), "def run(): pass\n").unwrap();
    std::fs::write(ws.path().join("doc.md"), "# Doc\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("add")
        .arg("main.py::run")
        .arg("doc.md")
        .arg("--dry-run")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Dry run: nothing written."));

    assert!(!ws.path().join(".agentic-doc/relations.json").exists());
}

#[test]
fn test_cli_relations_check_returns_0_with_orphans() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let rel_json = r#"{
  "version": 1,
  "relations": [
    {
      "source": "legacy.py::Old",
      "target": "doc.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}"#;
    std::fs::write(ws.path().join(".agentic-doc/relations.json"), rel_json).unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("relations")
        .arg("check")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("1 source not found"));
}
