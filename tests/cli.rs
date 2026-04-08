use std::process::{Command, Stdio};

fn bond_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_doublenot-bond"));
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("GOOGLE_API_KEY");
    cmd.env_remove("API_KEY");
    cmd.env("HOME", "/nonexistent-bond-test-home");
    cmd
}

#[test]
fn help_flag_prints_usage() {
    let output = bond_cmd()
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Usage:"),
        "help should include usage: {stdout}"
    );
    assert!(stdout.contains("--bootstrap-only"));
}

#[test]
fn version_flag_prints_version() {
    let output = bond_cmd()
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.starts_with("doublenot-bond v"));
}

#[test]
fn prompt_mode_without_api_key_shows_error() {
    let temp = tempfile::tempdir().expect("tempdir");

    let output = bond_cmd()
        .arg("--repo")
        .arg(temp.path())
        .arg("--prompt")
        .arg("hello")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("API key") || stderr.contains("--api-key"));
}

#[test]
fn help_flag_shows_permission_and_directory_flags() {
    let output = bond_cmd()
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--allow <pattern>"));
    assert!(stdout.contains("--deny-dir <path>"));
}

#[test]
fn help_flag_lists_issue_workflow_commands() {
    let output = bond_cmd()
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("/issues select <n>"));
    assert!(stdout.contains("/issues resume"));
    assert!(stdout.contains("/issues reopen-current"));
    assert!(stdout.contains("/issues previous"));
    assert!(stdout.contains("/issues history"));
    assert!(stdout.contains("/issues park"));
    assert!(stdout.contains("/issues sync"));
    assert!(stdout.contains("/issues complete [msg]"));
}

#[test]
fn help_flag_documents_workflow_schedule_command() {
    let output = bond_cmd()
        .arg("--help")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("/setup workflow schedule"),
        "help should document /setup workflow schedule: {stdout}"
    );
}
