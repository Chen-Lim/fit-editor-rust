use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

/// Path to the test FIT file from the SDK fixtures.
const TEST_FIT: &str = "tests/fixtures/Activity.fit";

fn test_fit_path() -> &'static Path {
    Path::new(TEST_FIT)
}

// ─── Encode round-trip ──────────────────────────────────────────────

#[test]
fn export_json_then_encode_produces_valid_fit() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("export.json");
    let fit_path = dir.path().join("roundtrip.fit");

    // Export to JSON
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "export",
            test_fit_path().to_str().unwrap(),
            "--format",
            "json",
            "--output",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Encode back to FIT
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "encode",
            json_path.to_str().unwrap(),
            "-o",
            fit_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Validate the round-tripped file
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["validate", fit_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn encode_roundtrip_preserves_message_count() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("export.json");
    let fit_path = dir.path().join("roundtrip.fit");

    // Export
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "export",
            test_fit_path().to_str().unwrap(),
            "--format",
            "json",
            "--output",
            json_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Encode
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "encode",
            json_path.to_str().unwrap(),
            "-o",
            fit_path.to_str().unwrap(),
        ])
        .assert()
        .success();

    // Check message counts match
    let orig = Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["info", test_fit_path().to_str().unwrap()])
        .output()
        .unwrap();
    let round = Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["info", fit_path.to_str().unwrap()])
        .output()
        .unwrap();

    let orig_out = String::from_utf8_lossy(&orig.stdout);
    let round_out = String::from_utf8_lossy(&round.stdout);

    // Both should have record messages (the majority)
    assert!(
        orig_out.contains("record"),
        "original should contain record messages"
    );
    assert!(
        round_out.contains("record"),
        "roundtrip should contain record messages"
    );

    // Original has 3601 records (minus developer metadata = 3608 total)
    // Roundtrip should have the same record count
    assert!(
        round_out.contains("3601"),
        "roundtrip should preserve 3601 record messages"
    );
}

// ─── Edit commands ──────────────────────────────────────────────────

#[test]
fn edit_set_modifies_field_value() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("edited.fit");

    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "edit",
            test_fit_path().to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
            "--set",
            "session.total_elapsed_time=7200",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Applied 1 field edit(s)"));

    // Validate
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["validate", out_path.to_str().unwrap()])
        .assert()
        .success();

    // Verify the value changed
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["dump", out_path.to_str().unwrap(), "--message", "session"])
        .assert()
        .success()
        .stdout(predicate::str::contains("total_elapsed_time = 7200"));
}

#[test]
fn edit_remove_message_removes_specified_type() {
    let dir = tempfile::tempdir().unwrap();
    let out_path = dir.path().join("nolap.fit");

    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "edit",
            test_fit_path().to_str().unwrap(),
            "-o",
            out_path.to_str().unwrap(),
            "--remove-message",
            "lap",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Removed 1 message(s)"));

    // Validate
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["validate", out_path.to_str().unwrap()])
        .assert()
        .success();

    // Verify lap is gone — use " lap " (space-padded) to avoid substring matches
    let info_out = Command::cargo_bin("fit-editor")
        .unwrap()
        .args(["info", out_path.to_str().unwrap()])
        .output()
        .unwrap();
    let info_str = String::from_utf8_lossy(&info_out.stdout);
    // The "lap" message count line would appear as "  lap                  1"
    assert!(
        !info_str.contains(" lap "),
        "lap message should be removed, got: {info_str}"
    );
}

// ─── Hexdump ────────────────────────────────────────────────────────

#[test]
fn hexdump_shows_file_header() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "hexdump",
            test_fit_path().to_str().unwrap(),
            "--bytes",
            "16",
        ])
        .assert()
        .success()
        .stdout(
            predicate::str::contains(".FIT") // signature
                .and(predicate::str::contains("0e")), // header size byte
        );
}

#[test]
fn hexdump_annotate_shows_message_boundaries() {
    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "hexdump",
            test_fit_path().to_str().unwrap(),
            "--bytes",
            "64",
            "--annotate",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Annotations:"));
}

// ─── Encode error handling ──────────────────────────────────────────

#[test]
fn encode_rejects_missing_messages_array() {
    let dir = tempfile::tempdir().unwrap();
    let json_path = dir.path().join("bad.json");
    let fit_path = dir.path().join("out.fit");

    std::fs::write(&json_path, r#"{"not_messages": []}"#).unwrap();

    Command::cargo_bin("fit-editor")
        .unwrap()
        .args([
            "encode",
            json_path.to_str().unwrap(),
            "-o",
            fit_path.to_str().unwrap(),
        ])
        .assert()
        .failure();
}
