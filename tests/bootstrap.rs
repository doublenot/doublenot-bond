use serde_yaml::Value;
use std::process::{Command, Stdio};
use tempfile::tempdir;

fn bond_cmd() -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_doublenot-bond"));
    cmd.env_remove("ANTHROPIC_API_KEY");
    cmd.env_remove("OPENAI_API_KEY");
    cmd.env_remove("GOOGLE_API_KEY");
    cmd.env_remove("API_KEY");
    cmd
}

#[test]
fn bootstrap_only_creates_bond_tree() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bootstrap-only")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "bootstrap should exit 0");
    assert!(repo.join(".bond").is_dir(), ".bond should exist");
    assert!(repo.join(".bond/bin").is_dir(), ".bond/bin should exist");
    assert!(repo.join(".bond/IDENTITY.md").is_file());
    assert!(repo.join(".bond/PERSONALITY.md").is_file());
    assert!(repo.join(".bond/JOURNAL.md").is_file());
    assert!(repo.join(".bond/config.yml").is_file());
    assert!(repo.join(".bond/state.yml").is_file());
    assert!(repo.join(".bond/bin/doublenot-bond").is_file());
    assert!(repo.join(".github/ISSUE_TEMPLATE").is_dir());
    assert!(repo.join(".github/ISSUE_TEMPLATE/config.yml").is_file());
    assert!(repo.join(".github/ISSUE_TEMPLATE/bond-setup.md").is_file());
    assert!(repo.join(".github/ISSUE_TEMPLATE/bond-task.md").is_file());
    assert!(repo.join(".github/ISSUE_TEMPLATE/bond-debug.md").is_file());
}

#[test]
fn bootstrap_only_writes_runtime_executable() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bootstrap-only")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "bootstrap should exit 0");

    let executable = repo.join(".bond/bin/doublenot-bond");
    assert!(executable.is_file(), "repo-local executable should exist");
    let metadata = std::fs::metadata(executable).expect("metadata for repo-local executable");
    assert!(
        metadata.len() > 0,
        "repo-local executable should not be empty"
    );
}

#[test]
fn bootstrap_only_writes_default_config_and_state() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bootstrap-only")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "bootstrap should exit 0");

    let config_text =
        std::fs::read_to_string(repo.join(".bond/config.yml")).expect("read .bond/config.yml");
    let config: Value = serde_yaml::from_str(&config_text).expect("parse config yaml");

    let state_text =
        std::fs::read_to_string(repo.join(".bond/state.yml")).expect("read .bond/state.yml");
    let state: Value = serde_yaml::from_str(&state_text).expect("parse state yaml");

    assert_eq!(
        config.get("executable_path").and_then(Value::as_str),
        Some(".bond/bin/doublenot-bond")
    );
    assert_eq!(
        config
            .get("automation")
            .and_then(|automation| automation.get("schedule_cron"))
            .and_then(Value::as_str),
        Some("0 * * * *")
    );
    assert_eq!(
        config
            .get("automation")
            .and_then(|automation| automation.get("provider"))
            .and_then(Value::as_str),
        Some("anthropic")
    );
    assert_eq!(
        config
            .get("automation")
            .and_then(|automation| automation.get("model"))
            .and_then(Value::as_str),
        Some("claude-sonnet-4-20250514")
    );
    assert_eq!(
        config
            .get("automation")
            .and_then(|automation| automation.get("multiple_issues"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        config
            .get("automation")
            .and_then(|automation| automation.get("thinking_effort"))
            .and_then(Value::as_str),
        Some("medium")
    );
    assert!(config.get("configured").is_none());
    assert!(config.get("last_issue").is_none());
    assert!(config.get("current_issue").is_none());
    assert!(config.get("issue_history").is_none());

    assert_eq!(
        state.get("configured").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        state.get("autonomous_enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert!(state.get("last_issue").is_some());
    assert!(state.get("current_issue").is_some());
    assert!(state.get("issue_history").is_some());
    assert_eq!(
        state
            .get("issue_history")
            .and_then(Value::as_sequence)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        config
            .get("commands")
            .and_then(|commands| commands.get("test"))
            .and_then(Value::as_sequence)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        config
            .get("commands")
            .and_then(|commands| commands.get("lint"))
            .and_then(Value::as_sequence)
            .map(Vec::len),
        Some(0)
    );
    assert!(
        config_text.contains("Add repo-specific test commands"),
        "config should explain how to fill in test commands"
    );
    assert!(
        config_text.contains("Add repo-specific lint commands"),
        "config should explain how to fill in lint commands"
    );
    assert!(
        !config_text.contains("cargo test"),
        "bootstrap config should not contain Rust-specific test defaults"
    );
    assert!(
        !config_text.contains("cargo clippy"),
        "bootstrap config should not contain Rust-specific lint defaults"
    );
    assert_eq!(
        config
            .get("issues")
            .and_then(|issues| issues.get("eligible_labels"))
            .and_then(Value::as_sequence)
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        config
            .get("issues")
            .and_then(|issues| issues.get("issue_history_limit"))
            .and_then(Value::as_u64),
        Some(10)
    );
}

#[test]
fn bootstrap_only_writes_prompt_contract_issue_templates() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bootstrap-only")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "bootstrap should exit 0");

    let task_template = std::fs::read_to_string(repo.join(".github/ISSUE_TEMPLATE/bond-task.md"))
        .expect("read bond-task template");
    assert!(task_template.contains("## Inputs"));
    assert!(task_template.contains("## Acceptance Criteria"));
    assert!(task_template.contains("Depends on: #123"));

    let debug_template = std::fs::read_to_string(repo.join(".github/ISSUE_TEMPLATE/bond-debug.md"))
        .expect("read bond-debug template");
    assert!(debug_template.contains("## Edge Cases"));
    assert!(debug_template.contains("Depends on: #123"));
}

#[test]
fn bootstrap_only_writes_identity_and_personality_templates() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bootstrap-only")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "bootstrap should exit 0");

    let identity = std::fs::read_to_string(repo.join(".bond/IDENTITY.md")).expect("read identity");
    assert!(identity.contains("- Name: "));
    assert!(identity.contains("Describe what this repository is for"));
    assert!(identity.contains("needs-human"));
    assert!(identity.contains("outside the bond's intake queue"));

    let personality =
        std::fs::read_to_string(repo.join(".bond/PERSONALITY.md")).expect("read personality");
    assert!(personality.contains("careful repository mechanic"));
    assert!(personality.contains("Operator-aware"));
    assert!(personality.contains("needs-human"));
}
