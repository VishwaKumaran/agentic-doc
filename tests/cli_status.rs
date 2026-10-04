use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_cli_status_unconfigured_fails() {
    let ws = tempdir().unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "Error: agentic-doc is not configured in this directory.",
        ));
}

#[test]
fn test_cli_status_no_snapshot_premier_run_succes() {
    // Étape 17 : sans snapshot, status doit réussir et afficher l'en-tête du premier run.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("app.py"), "def start(): pass\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains(
            "First analysis — no snapshot recorded yet.",
        ))
        .stdout(predicate::str::contains(
            "All elements are reported as added.",
        ));
}

#[test]
fn test_cli_status_no_snapshot_premier_run_json_previous_snapshot_null() {
    // Étape 17 : au premier run, previous_snapshot vaut null en JSON.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("app.py"), "def start(): pass\n").unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("status")
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "status");
    assert_eq!(json_val["schema_version"], 1);
    assert_eq!(json_val["previous_snapshot"], serde_json::Value::Null);
    assert_eq!(json_val["current_snapshot"], serde_json::Value::Null);
    // Tous les changements sont du type file_added ou element_added
    for change in json_val["changes"].as_array().unwrap() {
        let typ = change["type"].as_str().unwrap();
        assert!(
            typ == "file_added" || typ == "element_added",
            "changement inattendu au premier run : {typ}"
        );
    }
}

#[test]
fn test_cli_status_m1_scenario() {
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();

    let auth_py = proj.path().join("auth.py");
    std::fs::write(&auth_py, "class AuthService:\n    pass\n").unwrap();

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

    // 3. status sans changement
    let mut status_cmd1 = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd1
        .current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "No changes since the last snapshot.",
        ));

    // 4. modification de auth.py
    std::fs::write(
        &auth_py,
        "class AuthService:\n    pass\n\ndef refresh_token():\n    pass\n",
    )
    .unwrap();

    // 5. status avec changement
    let mut status_cmd2 = Command::cargo_bin("agentic-doc").unwrap();
    status_cmd2
        .current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("auth.py::refresh_token  added"));

    // 6. status avec --json
    let mut status_json_cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = status_json_cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("status")
        .assert()
        .success();

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "status");
    assert_eq!(json_val["current_snapshot"], serde_json::Value::Null);
    // Après scan, previous_snapshot doit être non-null
    assert!(json_val["previous_snapshot"].is_string());
}

#[test]
fn test_cli_status_zero_files_emits_warning_on_stderr() {
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

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("Warning: 0 source file analyzed."))
        .stderr(predicate::str::contains(
            "Hint: the project may use a language that is not supported yet",
        ));
}

#[test]
fn test_cli_status_zero_files_json_stdout_intact() {
    // Étape 18 : avec --json et 0 fichier, stdout reste un JSON valide.
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

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stderr(predicate::str::contains("Warning: 0 source file analyzed."));

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(json_val["command"], "status");
    assert!(json_val["changes"].as_array().unwrap().is_empty());
}

#[test]
fn test_cli_status_premier_run_liste_seulement_les_fichiers() {
    // Règle d'affichage (cli.md §6) : un fichier ajouté en bloc est affiché seul.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(
        proj.path().join("base.py"),
        "class BaseTable:\n    __tablename__ = \"base\"\n\n    def query(self):\n        return None\n",
    )
    .unwrap();

    let mut setup_cmd = Command::cargo_bin("agentic-doc").unwrap();
    setup_cmd
        .current_dir(ws.path())
        .arg("setup")
        .arg(proj.path())
        .assert()
        .success();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .code(0)
        .stdout(predicate::str::contains("  base.py\n"))
        .stdout(predicate::str::contains("base.py::").not())
        .stdout(predicate::str::contains(
            "1 file added, 0 unchanged files, 3 changes (2 inside added or removed files)",
        ));
}

#[test]
fn test_cli_status_fichier_ajoute_ne_liste_pas_ses_elements() {
    // Règle d'affichage (cli.md §6) : idem entre deux snapshots.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("keep.py"), "class Keep:\n    pass\n").unwrap();

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

    std::fs::write(proj.path().join("other.py"), "class Other:\n    pass\n").unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("  other.py\n"))
        .stdout(predicate::str::contains("other.py::").not())
        // Les fichiers non modifiés ne sont plus énumérés : seul leur compte reste.
        .stdout(predicate::str::contains("Unchanged:").not())
        .stdout(predicate::str::contains(
            "1 changed file, 1 unchanged file, 2 changes (1 inside added or removed files)",
        ));
}

#[test]
fn test_cli_status_fichier_supprime_ne_liste_pas_ses_elements() {
    // Règle d'affichage (cli.md §6) : symétrique pour un fichier supprimé en bloc.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    let doomed = proj.path().join("doomed.py");
    std::fs::write(
        &doomed,
        "class Doomed:\n    def run(self):\n        return None\n",
    )
    .unwrap();
    std::fs::write(proj.path().join("keep.py"), "class Keep:\n    pass\n").unwrap();

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

    std::fs::remove_file(&doomed).unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    cmd.current_dir(ws.path())
        .arg("status")
        .assert()
        .success()
        .stdout(predicate::str::contains("  doomed.py\n"))
        .stdout(predicate::str::contains("doomed.py::").not())
        .stdout(predicate::str::contains(
            "1 changed file, 1 unchanged file, 3 changes (2 inside added or removed files)",
        ));
}

#[test]
fn test_cli_status_json_reste_complet_pour_un_fichier_ajoute() {
    // Le contrat JSON n'applique PAS la règle d'affichage : les éléments restent listés.
    let ws = tempdir().unwrap();
    let proj = tempdir().unwrap();
    std::fs::write(proj.path().join("keep.py"), "class Keep:\n    pass\n").unwrap();

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

    std::fs::write(proj.path().join("other.py"), "class Other:\n    pass\n").unwrap();

    let mut cmd = Command::cargo_bin("agentic-doc").unwrap();
    let assert_out = cmd
        .current_dir(ws.path())
        .arg("--json")
        .arg("status")
        .assert()
        .success()
        .code(0);

    let stdout = String::from_utf8(assert_out.get_output().stdout.clone()).unwrap();
    let json_val: serde_json::Value = serde_json::from_str(&stdout).unwrap();

    let changes = json_val["changes"].as_array().unwrap();
    assert!(
        changes
            .iter()
            .any(|c| c["type"] == "element_added" && c["element"] == "other.py::Other"),
        "le JSON doit rester complet pour un fichier ajouté : {changes:?}"
    );
    // Le résumé JSON compte tous les changements, comme le texte.
    assert_eq!(json_val["summary"]["changes"], 2);
}
