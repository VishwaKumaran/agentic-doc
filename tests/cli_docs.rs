use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_docs_m2_scenario() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let auth_py = proj.path().join("auth.py");
    std::fs::write(&auth_py, "class AuthService:\n    pass\n").unwrap();

    let doc_file = ws.path().join("authentication.md");
    std::fs::write(&doc_file, "# Authentication\nDoc text\n").unwrap();

    // 1. setup
    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // 2. scan
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(ws.path())
        .arg("scan")
        .assert()
        .success();

    // 3. ecrire relations.json
    let rel_dir = ws.path().join(".agentic-doc");
    let rel_json = r#"{
  "version": 1,
  "relations": [
    {
      "source": "auth.py::AuthService",
      "target": "authentication.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}"#;
    std::fs::write(rel_dir.join("relations.json"), rel_json).unwrap();

    // 4. modifier auth.py
    std::fs::write(&auth_py, "class AuthService:\n    def login(): pass\n").unwrap();

    // 5. status -> changements
    let mut status_cmd = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd
        .current_dir(ws.path())
        .arg("status")
        .assert()
        .success();

    // 6. docs -> impact sur authentication.md
    let mut docs_cmd = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd
        .current_dir(ws.path())
        .arg("docs")
        .assert()
        .success()
        .stdout(predicate::str::contains("authentication.md"))
        .stdout(predicate::str::contains("AuthService was modified."))
        // Sortie compacte : le document porte sa confiance, chaque raison est suivie
        // directement de sa relation — plus de sections « Reason: » / « Relation: » /
        // « Confidence: ».
        .stdout(predicate::str::contains("Reason:").not())
        .stdout(predicate::str::contains("Confidence:").not())
        .stdout(predicate::str::contains(
            "auth.py::AuthService → authentication.md",
        ));

    // 7. docs --json
    let mut docs_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = docs_json_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("docs")
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "docs");
    assert_eq!(json_val["schema_version"], 2);
    assert_eq!(json_val["impacts"][0]["document"], "authentication.md");
}

#[test]
fn test_cli_docs_sans_snapshot_premier_run_succes() {
    // Étape 17 : sans snapshot, docs doit réussir (premier run).
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("a.py"), "def foo(): pass\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // Pas de scan : premier run.
    let mut docs_cmd = Command::cargo_bin("agentic-doc").unwrap();
    docs_cmd
        .current_dir(ws.path())
        .arg("docs")
        .assert()
        .success()
        .code(0);
}

#[test]
fn test_cli_docs_sans_snapshot_json_valide() {
    // Étape 17 : au premier run, --json doit produire un objet JSON valide.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("a.py"), "def foo(): pass\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut docs_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = docs_json_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("docs")
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "docs");
    // Pas d'impacts sans relations
    assert!(json_val["impacts"].as_array().unwrap().is_empty());
}

#[test]
fn test_cli_docs_coverage_sans_relations() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("a.py"), "def foo(): pass\n").unwrap();
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
        .arg("docs")
        .arg("--coverage")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Documentation coverage"))
        .stdout(predicate::str::contains("2 undocumented"))
        .stdout(predicate::str::contains("Unreferenced documents: 1"));
}

#[test]
fn test_cli_docs_coverage_with_all_flag() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("a.py"), "def foo(): pass\n").unwrap();
    std::fs::write(ws.path().join("doc.md"), "# Doc\n").unwrap();

    let rel_dir = ws.path().join(".agentic-doc");
    std::fs::create_dir_all(&rel_dir).unwrap();
    let rel_json = r#"{
  "version": 1,
  "relations": [
    {
      "source": "a.py::foo",
      "target": "doc.md",
      "origin": "explicit",
      "status": "validated",
      "confidence": "high"
    }
  ]
}"#;
    std::fs::write(rel_dir.join("relations.json"), rel_json).unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("docs")
        .arg("--coverage")
        .arg("--all")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("Documented directly:"));
}

#[test]
fn test_cli_docs_aucun_changement_ne_parle_pas_de_relations() {
    // cli.md §7 : sans aucun changement, le message ne doit pas évoquer les relations.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(
        proj.path().join("auth.py"),
        "class AuthService:\n    pass\n",
    )
    .unwrap();

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
        .success();

    // Une relation est déclarée, mais le projet n'a pas changé depuis le scan.
    let rel_dir = ws.path().join(".agentic-doc");
    std::fs::write(
        rel_dir.join("relations.json"),
        "{\n  \"version\": 1,\n  \"relations\": [\n    { \"source\": \"auth.py::AuthService\", \"target\": \"authentication.md\", \"origin\": \"explicit\", \"status\": \"validated\", \"confidence\": \"high\" }\n  ]\n}\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("docs")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("No documentation impact."))
        .stdout(predicate::str::contains(
            "No changes since the last snapshot.",
        ))
        .stdout(predicate::str::contains("no documentation relation was found").not());
}

#[test]
fn test_cli_docs_all_without_coverage_fails() {
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
        .arg("docs")
        .arg("--all")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains(
            "Error: --all requires --coverage.",
        ));
}

#[test]
fn test_cli_docs_warnings_for_orphan_relation_source_and_target_text() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    // Projet avec un fichier auth.py et une fonction login
    std::fs::write(
        proj.path().join("auth.py"),
        "def login():\n    return True\n",
    )
    .unwrap();
    // Document existant auth.md
    std::fs::write(ws.path().join("auth.md"), "# Auth Documentation\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    // Snapshot initial
    let mut scan_cmd = Command::cargo_bin("agentic-doc").unwrap();
    scan_cmd
        .current_dir(ws.path())
        .arg("scan")
        .assert()
        .success();

    // Modification du code pour produire un impact
    std::fs::write(
        proj.path().join("auth.py"),
        "def login():\n    return False\n",
    )
    .unwrap();

    // Déclaration de relations : 1 valide, 1 source orpheline, 1 cible orpheline
    let rel_dir = ws.path().join(".agentic-doc");
    std::fs::write(
        rel_dir.join("relations.json"),
        r#"{
  "version": 1,
  "relations": [
    { "source": "auth.py::login", "target": "auth.md", "origin": "explicit", "status": "validated", "confidence": "high" },
    { "source": "src/legacy.py::OldAuth", "target": "auth.md", "origin": "explicit", "status": "validated", "confidence": "high" },
    { "source": "auth.py::login", "target": "removed-doc.md", "origin": "explicit", "status": "validated", "confidence": "high" }
  ]
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("docs")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "Potentially impacted documentation",
        ))
        .stdout(predicate::str::contains("auth.md"))
        .stdout(predicate::str::contains("Warnings:"))
        .stdout(predicate::str::contains(
            "relation source not found: src/legacy.py::OldAuth",
        ))
        .stdout(predicate::str::contains(
            "relation target not found: removed-doc.md",
        ));
}

#[test]
fn test_cli_docs_warnings_for_orphan_relation_source_and_target_json() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    std::fs::write(
        proj.path().join("auth.py"),
        "def login():\n    return True\n",
    )
    .unwrap();
    std::fs::write(ws.path().join("auth.md"), "# Auth Documentation\n").unwrap();

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
        .success();

    std::fs::write(
        proj.path().join("auth.py"),
        "def login():\n    return False\n",
    )
    .unwrap();

    let rel_dir = ws.path().join(".agentic-doc");
    std::fs::write(
        rel_dir.join("relations.json"),
        r#"{
  "version": 1,
  "relations": [
    { "source": "auth.py::login", "target": "auth.md", "origin": "explicit", "status": "validated", "confidence": "high" },
    { "source": "src/legacy.py::OldAuth", "target": "auth.md", "origin": "explicit", "status": "validated", "confidence": "high" },
    { "source": "auth.py::login", "target": "removed-doc.md", "origin": "explicit", "status": "validated", "confidence": "high" }
  ]
}
"#,
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = cmd
        .current_dir(ws.path())
        .arg("docs")
        .arg("--json")
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let val: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(val["command"], "docs");
    assert_eq!(val["schema_version"], 2);

    let warnings = val["warnings"].as_array().expect("warnings array in json");
    let warnings_str: Vec<&str> = warnings.iter().map(|w| w.as_str().unwrap()).collect();

    assert_eq!(
        warnings_str,
        vec![
            "relation source not found: src/legacy.py::OldAuth",
            "relation target not found: removed-doc.md",
        ]
    );
}
