use assert_cmd::Command;
use predicates::prelude::*;

const TEST_FIT: &str = "../fit-sdk-rust/tests/fixtures/test_data/Activity.fit";

#[test]
fn completion_zsh_emits_compdef() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["completion", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef fit-editor"));
}

#[test]
fn completion_bash_emits_function() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["completion", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_fit-editor()"));
}

#[test]
fn no_color_strips_ansi_from_info() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["--no-color", "info", TEST_FIT])
        .assert()
        .success()
        .stdout(predicate::str::contains("\x1b[").not())
        .stdout(predicate::str::contains("File Information"));
}

#[test]
fn info_renders_message_counts_table() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["--no-color", "info", TEST_FIT])
        .assert()
        .success()
        .stdout(predicate::str::contains("Message Counts"))
        .stdout(predicate::str::contains("record"));
}

#[test]
fn summary_renders_table() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["--no-color", "summary", TEST_FIT])
        .assert()
        .success()
        .stdout(predicate::str::contains("Activity Summary"))
        .stdout(predicate::str::contains("Sport"));
}

#[test]
fn batch_non_tty_runs_to_completion() {
    let dir = tempfile::tempdir().unwrap();
    let fit_copy = dir.path().join("a.fit");
    std::fs::copy(TEST_FIT, &fit_copy).unwrap();
    let pattern = dir.path().join("*.fit").to_string_lossy().into_owned();

    let bin = assert_cmd::cargo::cargo_bin("fit-editor");

    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "batch",
            &pattern,
            "--",
            bin.to_str().unwrap(),
            "validate",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("succeeded"));
}
