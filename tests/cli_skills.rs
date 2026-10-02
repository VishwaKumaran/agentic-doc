//! Tests d'intégration de la commande `skills` : un alias vers `npx skills add`.
//!
//! Aucun accès réseau : un faux `npx` est placé en tête de `PATH`. Il journalise
//! les arguments reçus et renvoie un code de sortie configurable.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::tempdir;

/// Crée un faux `npx` exécutable dans `<dir>/bin` et renvoie ce répertoire.
fn fake_npx(dir: &Path) -> PathBuf {
    let bin = dir.join("bin");
    fs::create_dir_all(&bin).unwrap();
    let script = bin.join("npx");
    fs::write(
        &script,
        "#!/bin/sh\nprintf '%s\\n' \"$@\" > \"$NPX_LOG\"\nexit \"${NPX_EXIT:-0}\"\n",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).unwrap();
    }
    bin
}

/// `PATH` avec `bin` en tête, pour que `Command::new("npx")` trouve le faux.
fn path_with(bin: &Path) -> String {
    let existing = std::env::var("PATH").unwrap_or_default();
    format!("{}:{}", bin.display(), existing)
}

#[test]
fn forwards_args_to_npx_skills_add() {
    let tmp = tempdir().unwrap();
    let bin = fake_npx(tmp.path());
    let log = tmp.path().join("argv.txt");

    Command::cargo_bin("agentic-doc")
        .unwrap()
        .env("PATH", path_with(&bin))
        .env("NPX_LOG", &log)
        .args(["skills", "install", "-a", "claude-code", "--copy"])
        .assert()
        .success();

    let logged = fs::read_to_string(&log).unwrap();
    assert_eq!(
        logged,
        "-y\nskills\nadd\nVishwaKumaran/agentic-doc\n-a\nclaude-code\n--copy\n"
    );
}

#[test]
fn add_is_an_alias_of_install() {
    let tmp = tempdir().unwrap();
    let bin = fake_npx(tmp.path());
    let log = tmp.path().join("argv.txt");

    Command::cargo_bin("agentic-doc")
        .unwrap()
        .env("PATH", path_with(&bin))
        .env("NPX_LOG", &log)
        .args(["skills", "add", "--all"])
        .assert()
        .success();

    let logged = fs::read_to_string(&log).unwrap();
    assert_eq!(
        logged,
        "-y\nskills\nadd\nVishwaKumaran/agentic-doc\n--all\n"
    );
}

#[test]
fn global_json_is_reemitted_to_npx() {
    let tmp = tempdir().unwrap();
    let bin = fake_npx(tmp.path());
    let log = tmp.path().join("argv.txt");

    Command::cargo_bin("agentic-doc")
        .unwrap()
        .env("PATH", path_with(&bin))
        .env("NPX_LOG", &log)
        .args(["--json", "skills", "install"])
        .assert()
        .success();

    let logged = fs::read_to_string(&log).unwrap();
    assert!(logged.contains("--json\n"), "argv was: {logged}");
}

#[test]
fn propagates_child_exit_code() {
    let tmp = tempdir().unwrap();
    let bin = fake_npx(tmp.path());
    let log = tmp.path().join("argv.txt");

    Command::cargo_bin("agentic-doc")
        .unwrap()
        .env("PATH", path_with(&bin))
        .env("NPX_LOG", &log)
        .env("NPX_EXIT", "3")
        .args(["skills", "install"])
        .assert()
        .code(3);
}

#[test]
fn missing_npx_fails_with_explicit_message() {
    let tmp = tempdir().unwrap();
    let empty = tmp.path().join("empty");
    fs::create_dir_all(&empty).unwrap();

    Command::cargo_bin("agentic-doc")
        .unwrap()
        .env("PATH", empty)
        .args(["skills", "install"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("npx not found"));
}
