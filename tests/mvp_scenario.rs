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

    // 7. docs -> vérifier authentication.md + raison + relation
    let mut docs_cmd = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd
        .current_dir(&sample_docs_dir)
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication.md"))
        .stdout(predicate::str::contains("AuthService was modified."));

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
