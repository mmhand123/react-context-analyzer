use std::path::PathBuf;

use assert_cmd::Command;

#[test]
fn cli_accepts_project_path_as_positional_argument() {
    let fixture_path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("basic_working_context")
        .join("input");

    let command_output = Command::cargo_bin("context-analyzer-cli")
        .expect("cli binary should build")
        .arg(&fixture_path)
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_json: serde_json::Value =
        serde_json::from_slice(&command_output).expect("stdout should contain valid json");

    assert_eq!(output_json["summary"]["file_count"], 1);
    assert_eq!(output_json["summary"]["context_count"], 1);
    assert_eq!(output_json["summary"]["provider_count"], 1);
    assert_eq!(output_json["summary"]["consumer_count"], 1);
    assert!(output_json.get("diagnostics").is_none());
}

#[test]
fn cli_pretty_flag_outputs_indented_json() {
    let fixture_path = workspace_root()
        .join("tests")
        .join("fixtures")
        .join("basic_working_context")
        .join("input");

    let command_output = Command::cargo_bin("context-analyzer-cli")
        .expect("cli binary should build")
        .arg(&fixture_path)
        .arg("--pretty")
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let output_text = String::from_utf8(command_output).expect("stdout should be utf8 text");
    assert!(output_text.contains("\n  \"summary\": {"));
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}
