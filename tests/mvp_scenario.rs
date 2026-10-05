use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::tempdir;

#[test]
fn test_mvp_scenario() {
    let temp = tempdir().unwrap();
    let sample_project_dir = temp.path().join("sample-project");
    let sample_docs_dir = temp.path().join("sample-docs");

    fs::create_dir_all(sample_project_dir.join("src")).unwrap();
    fs::create_dir_all(&sample_docs_dir).unwrap();

    fs::copy(
        "tests/fixtures/sample-project/src/auth.py",
        sample_project_dir.join("src/auth.py"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-project/src/users.py",
        sample_project_dir.join("src/users.py"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-project/src/config.py",
        sample_project_dir.join("src/config.py"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-project/src/database.py",
        sample_project_dir.join("src/database.py"),
    )
    .unwrap();

    fs::copy(
        "tests/fixtures/sample-docs/authentication.md",
        sample_docs_dir.join("authentication.md"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-docs/users.md",
        sample_docs_dir.join("users.md"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-docs/architecture.md",
        sample_docs_dir.join("architecture.md"),
    )
    .unwrap();

    // 1. setup ../sample-project
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(&sample_docs_dir)
        .arg("setup")
        .arg("../sample-project")
        .assert()
        .success()
        .stdout(predicate::str::contains("Configuration created."));

    assert!(sample_docs_dir.join(".agentic-doc/config.json").exists());

    // 2. scan
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(&sample_docs_dir)
        .arg("scan")
        .assert()
        .success()
        .stdout(predicate::str::contains("Project scanned."));

    // 3. status -> aucun changement
    let mut status_cmd1 = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd1
        .current_dir(&sample_docs_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No changes since the last snapshot.",
        ));

    // 4. modifier src/auth.py (ajouter refresh_token())
    let auth_path = sample_project_dir.join("src/auth.py");
    fs::write(
        &auth_path,
        "class AuthService:\n    def authenticate(self, username, password):\n        return True\n\ndef refresh_token():\n    pass\n",
    )
    .unwrap();

    // 5. status -> changements attendus
    let mut status_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd2
        .current_dir(&sample_docs_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("src/auth.py::refresh_token"));

    // 6. ecrire relations.json
    let rel_json = r#"{
  "version": 1,
  "relations": [
    {
      "source": "src/auth.py",
      "target": "authentication.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}"#;
    fs::write(
        sample_docs_dir.join(".agentic-doc/relations.json"),
        rel_json,
    )
    .unwrap();

    // 7. docs -> vérifier authentication.md + raison + relation
    let mut docs_cmd = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication.md"))
        .stdout(predicate::str::contains("src/auth.py was modified."));

    // 8. scan -> enregistrer le nouvel état
    let mut scan_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd2
        .current_dir(&sample_docs_dir)
        .arg("scan")
        .assert()
        .success();

    // 9. status -> aucun changement
    let mut status_cmd3 = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd3
        .current_dir(&sample_docs_dir)
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No changes since the last snapshot.",
        ));
}

#[test]
fn test_mvp_premier_run_setup_status_sans_scan() {
    // Étape 17 : setup → status sans scan doit lister tous les éléments comme Added.
    let temp = tempdir().unwrap();
    let sample_project_dir = temp.path().join("sample-project");
    let sample_docs_dir = temp.path().join("sample-docs");

    fs::create_dir_all(sample_project_dir.join("src")).unwrap();
    fs::create_dir_all(&sample_docs_dir).unwrap();

    fs::copy(
        "tests/fixtures/sample-project/src/auth.py",
        sample_project_dir.join("src/auth.py"),
    )
    .unwrap();

    // 1. setup sans scan
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(&sample_docs_dir)
        .arg("setup")
        .arg("../sample-project")
        .assert()
        .success();

    // 2. status sans scan → premier run : tous les éléments sont Added
    let mut status_cmd = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd
        .current_dir(&sample_docs_dir)
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "First analysis — no snapshot recorded yet.",
        ))
        .stdout(predicate::str::contains(
            "All elements are reported as added.",
        ))
        // src/auth.py doit apparaître dans la sortie
        .stdout(predicate::str::contains("src/auth.py"));
}

#[test]
fn test_m4_bootstrap_scenario() {
    // Jalon M4 : setup → docs --coverage → écriture relations.json → docs --coverage → scan
    let temp = tempdir().unwrap();
    let sample_project_dir = temp.path().join("sample-project");
    let sample_docs_dir = temp.path().join("sample-docs");

    fs::create_dir_all(sample_project_dir.join("src")).unwrap();
    fs::create_dir_all(&sample_docs_dir).unwrap();

    fs::copy(
        "tests/fixtures/sample-project/src/auth.py",
        sample_project_dir.join("src/auth.py"),
    )
    .unwrap();
    fs::copy(
        "tests/fixtures/sample-docs/authentication.md",
        sample_docs_dir.join("authentication.md"),
    )
    .unwrap();

    // 1. setup
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(&sample_docs_dir)
        .arg("setup")
        .arg("../sample-project")
        .assert()
        .success();

    // 2. docs --coverage sans aucun scan préalable
    let mut cov_cmd1 = Command::cargo_bin("agentic-doc").unwrap();
    cov_cmd1
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .arg("--coverage")
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage: 0%"))
        .stdout(predicate::str::contains("3 undocumented"));

    // 3. ecrire relations.json pour couvrir l'élément
    let rel_json = r#"{
  "version": 1,
  "relations": [
    {
      "source": "src/auth.py::AuthService",
      "target": "authentication.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}"#;
    fs::write(
        sample_docs_dir.join(".agentic-doc/relations.json"),
        rel_json,
    )
    .unwrap();

    // 4. docs --coverage met à jour le taux de couverture (1 sur 3 direct = 33%)
    let mut cov_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    cov_cmd2
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .arg("--coverage")
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage: 33%"))
        .stdout(predicate::str::contains("1 documented directly"));

    // 5. scan enregistre le premier état de référence
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(&sample_docs_dir)
        .arg("scan")
        .assert()
        .success();
}

#[test]
fn test_m5_scenario() {
    // Jalon M5 : bootstrap complet sans écriture manuelle de relations.json
    // setup → docs --coverage (0%) → relations add → docs --coverage (33%) → relations check → docs → scan → edit code → docs (impact) → relations list --json
    let temp = tempdir().unwrap();
    let sample_project_dir = temp.path().join("sample-project");
    let sample_docs_dir = temp.path().join("sample-docs");

    fs::create_dir_all(sample_project_dir.join("src")).unwrap();
    fs::create_dir_all(&sample_docs_dir).unwrap();

    let auth_py_path = sample_project_dir.join("src/auth.py");
    fs::copy("tests/fixtures/sample-project/src/auth.py", &auth_py_path).unwrap();
    fs::copy(
        "tests/fixtures/sample-docs/authentication.md",
        sample_docs_dir.join("authentication.md"),
    )
    .unwrap();

    // 1. setup
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(&sample_docs_dir)
        .arg("setup")
        .arg("../sample-project")
        .assert()
        .success();

    // 2. docs --coverage (initialement 0%)
    let mut cov_cmd1 = Command::cargo_bin("agentic-doc").unwrap();
    cov_cmd1
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .arg("--coverage")
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage: 0%"));

    // 3. relations add (sans modifier relations.json à la main)
    let mut add_cmd = Command::cargo_bin("agentic-doc").unwrap();
    add_cmd
        .current_dir(&sample_docs_dir)
        .arg("relations")
        .arg("add")
        .arg("src/auth.py::AuthService")
        .arg("authentication.md")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Relation added: src/auth.py::AuthService → authentication.md",
        ));

    // 4. docs --coverage met à jour le taux (33%)
    let mut cov_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    cov_cmd2
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .arg("--coverage")
        .assert()
        .success()
        .stdout(predicate::str::contains("coverage: 33%"))
        .stdout(predicate::str::contains("1 documented directly"));

    // 5. relations check (aucun problème, code 0)
    let mut check_cmd = Command::cargo_bin("agentic-doc").unwrap();
    check_cmd
        .current_dir(&sample_docs_dir)
        .arg("relations")
        .arg("check")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("1 relations"))
        .stdout(predicate::str::contains("0 source not found"))
        .stdout(predicate::str::contains("0 target not found"))
        .stdout(predicate::str::contains("0 duplicates"));

    // 6. docs (au premier run sans snapshot, AuthService est rapporté comme Added, donc l'impact est détecté sur authentication.md)
    let mut docs_cmd1 = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd1
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication.md"))
        .stdout(predicate::str::contains("AuthService was added."));

    // 7. scan (premier snapshot de référence)
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(&sample_docs_dir)
        .arg("scan")
        .assert()
        .success();

    // 8. Modification du code
    fs::write(
        &auth_py_path,
        "class AuthService:\n    def authenticate():\n        # modified\n        pass\n",
    )
    .unwrap();

    // 9. docs (impact détecté)
    let mut docs_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd2
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication.md"))
        .stdout(predicate::str::contains("AuthService was modified."));

    // 10. relations list --json
    let mut list_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = list_json_cmd
        .current_dir(&sample_docs_dir)
        .arg("--json")
        .arg("relations")
        .arg("list")
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "relations");
    assert_eq!(json_val["count"], 1);
    assert_eq!(
        json_val["relations"][0]["source"],
        "src/auth.py::AuthService"
    );
    assert_eq!(json_val["relations"][0]["target"], "authentication.md");
    assert_eq!(json_val["relations"][0]["resolution"], "ok");
}
