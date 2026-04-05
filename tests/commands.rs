use serde_yaml::Value;
use std::fs;
use std::process::{Command, Stdio};
use tempfile::tempdir;

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
fn prompt_tree_command_runs_without_api_key() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/tree . 1")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "tree command should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("."));
    assert!(
        stdout.contains(".bond/"),
        "tree output should include .bond/: {stdout}"
    );
}

#[test]
fn prompt_git_status_command_runs_without_api_key() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let init = Command::new("git")
        .arg("init")
        .current_dir(repo)
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init should succeed");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/git status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "git status command should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("> git status"));
    assert!(
        stdout.contains("## "),
        "git status output should show branch summary: {stdout}"
    );
}

#[test]
fn prompt_status_reports_active_and_parked_issue_summary() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
    )
    .expect("write config");
    fs::write(
        bond_dir.join("state.yml"),
        "configured: false\nautonomous_enabled: false\nsetup_issue: null\nlast_issue:\n  number: 21\n  title: Parked issue\n  url: https://github.com/acme/widgets/issues/21\n  label: bond-task\n  last_action: parked\n  last_action_at: 2026-04-02T01:00:00Z\ncurrent_issue:\n  number: 34\n  title: Active issue\n  url: https://github.com/acme/widgets/issues/34\n  label: bond-debug\n  last_action: resumed\n  last_action_at: 2026-04-02T02:00:00Z\nissue_history:\n  - number: 21\n    title: Parked issue\n    url: https://github.com/acme/widgets/issues/21\n    label: bond-task\n    last_action: parked\n    last_action_at: 2026-04-02T01:00:00Z\n  - number: 34\n    title: Active issue\n    url: https://github.com/acme/widgets/issues/34\n    label: bond-debug\n    last_action: resumed\n    last_action_at: 2026-04-02T02:00:00Z\n",
    )
    .expect("write state");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("current_issue: #34 [bond-debug] Active issue"));
    assert!(stdout.contains("issue_posture: active=1, parked=1"));
    assert!(stdout.contains("latest_parked: #21 [bond-task] Parked issue"));
    assert!(stdout.contains("automation_model_reasoning: <none>"));
    assert!(stdout.contains("automation_provider_matches_runtime: false"));
    assert!(stdout.contains("automation_model_looks_valid_for_provider: true"));
}

#[test]
fn prompt_status_reports_automation_provider_and_model_validation() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '0 * * * *'\n  provider: openai\n  model: claude-sonnet-4-20250514\n  model_reasoning: Need to compare the config against the runtime provider and model family.\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
    )
    .expect("write config");
    fs::write(
        bond_dir.join("state.yml"),
        "configured: false\nautonomous_enabled: false\nsetup_issue: null\nlast_issue: null\ncurrent_issue: null\nissue_history: []\n",
    )
    .expect("write state");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("anthropic")
        .arg("--prompt")
        .arg("/status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("automation_provider: openai"));
    assert!(stdout.contains("automation_model: claude-sonnet-4-20250514"));
    assert!(stdout.contains("automation_model_reasoning: Need to compare the config against the runtime provider and model family."));
    assert!(stdout.contains("automation_provider_matches_runtime: false"));
    assert!(stdout.contains("automation_model_looks_valid_for_provider: false"));
    assert!(stdout.contains("automation_recommended_model: gpt-4.1"));
    assert!(stdout.contains("automation_provider_warning:"));
    assert!(stdout.contains("automation_model_warning:"));
}

#[test]
fn prompt_status_accepts_matching_automation_provider_and_model() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '0 * * * *'\n  provider: anthropic\n  model: claude-sonnet-4-20250514\n  model_reasoning: Use the default Claude model for the scheduled repository workflow.\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
    )
    .expect("write config");
    fs::write(
        bond_dir.join("state.yml"),
        "configured: false\nautonomous_enabled: false\nsetup_issue: null\nlast_issue: null\ncurrent_issue: null\nissue_history: []\n",
    )
    .expect("write state");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("anthropic")
        .arg("--prompt")
        .arg("/status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("automation_provider_matches_runtime: true"));
    assert!(stdout.contains("automation_model_looks_valid_for_provider: true"));
    assert!(stdout.contains("automation_model_reasoning: Use the default Claude model for the scheduled repository workflow."));
    assert!(stdout.contains("automation_recommended_model: claude-sonnet-4-20250514"));
    assert!(!stdout.contains("automation_provider_warning:"));
    assert!(!stdout.contains("automation_model_warning:"));
}

#[test]
fn prompt_setup_workflow_schedule_updates_config_for_human_readable_interval() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup workflow schedule 'every 6 hours'")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "setup workflow schedule should succeed"
    );

    let config_text = fs::read_to_string(repo.join(".bond/config.yml")).expect("read config");
    let config: Value = serde_yaml::from_str(&config_text).expect("parse config");
    assert_eq!(
        config["automation"]["schedule_cron"].as_str(),
        Some("0 */6 * * *")
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Updated .bond/config.yml automation.schedule_cron to: 0 */6 * * *"));
    assert!(stdout.contains(
        "Run /setup workflow refresh to apply the new schedule to .github/workflows/bond.yml."
    ));
}

#[test]
fn prompt_test_fails_when_default_commands_are_unconfigured() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    fs::create_dir_all(repo.join("src")).expect("create src directory");
    fs::write(
        repo.join("Cargo.toml"),
        "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    )
    .expect("write Cargo.toml");
    fs::write(repo.join("src/lib.rs"), "pub fn ok() -> bool { true }\n").expect("write src/lib.rs");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/test")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "test command should fail when defaults are unconfigured"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No commands configured for this workflow in .bond/config.yml."),
        "unexpected stderr: {stderr}"
    );
}
