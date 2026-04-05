use serde_yaml::Value;
use std::fs;
use std::path::Path;
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

fn state_file(repo: &Path) -> std::path::PathBuf {
    repo.join(".bond/state.yml")
}

fn write_state_file(repo: &Path, configured: bool, autonomous_enabled: bool) {
    fs::create_dir_all(repo.join(".bond")).expect("create .bond directory");
    fs::write(
        state_file(repo),
        format!(
            "configured: {configured}\nautonomous_enabled: {autonomous_enabled}\nsetup_issue: null\nlast_issue: null\ncurrent_issue: null\nissue_history: []\n"
        ),
    )
    .expect("write state");
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

#[test]
fn prompt_setup_issue_creates_and_records_github_issue() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let init = Command::new("git")
        .arg("init")
        .current_dir(repo)
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init should succeed");

    let remote = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/acme/widgets.git",
        ])
        .current_dir(repo)
        .output()
        .expect("git remote add origin");
    assert!(
        remote.status.success(),
        "git remote add origin should succeed"
    );

    let fake_gh = repo.join("fake-gh.sh");
    let log_path = repo.join("gh.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\n' \"$*\" >> \"$BOND_GH_LOG\"\nif [ \"$1\" = \"label\" ] && [ \"$2\" = \"create\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"create\" ]; then\n  printf 'https://github.com/acme/widgets/issues/42\\n'\n  exit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&fake_gh)
            .expect("fake gh metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_gh, permissions).expect("chmod fake gh");
    }

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup issue")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &log_path)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "setup issue should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Created setup issue: https://github.com/acme/widgets/issues/42"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("number: 42"));
    assert!(config_text.contains("state: open"));
    assert!(config_text.contains("url: https://github.com/acme/widgets/issues/42"));

    let gh_log = fs::read_to_string(log_path).expect("read gh log");
    assert!(gh_log.contains("label create bond-setup"));
    assert!(gh_log.contains("issue create --repo acme/widgets"));
}

#[test]
fn prompt_setup_issue_fails_without_github_remote() {
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
        .arg("/setup issue")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "setup issue without GitHub remote should fail"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("GitHub origin remote") || stderr.contains("remote.origin.url"));
}

#[test]
fn prompt_setup_workflow_creates_bond_workflow_file() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup workflow")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "setup workflow should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Bond workflow installed:"),
        "stdout: {stdout}"
    );

    let workflow_text = fs::read_to_string(repo.join(".github/workflows/bond.yml"))
        .expect("read generated workflow");
    serde_yaml::from_str::<Value>(&workflow_text).expect("parse generated workflow");
    assert!(!workflow_text.contains("# Model reasoning:"));
    assert!(workflow_text.contains("cron: '0 * * * *'"));
    assert!(workflow_text.contains("concurrency:"));
    assert!(workflow_text.contains("group: bond-${{ github.ref }}"));
    assert!(workflow_text.contains("cancel-in-progress: false"));
    assert!(workflow_text.contains("timeout-minutes: 30"));
    assert!(workflow_text.contains("ref: ${{ github.ref_name }}"));
    assert!(workflow_text.contains("cargo build --locked --bin doublenot-bond"));
    assert!(workflow_text.contains("mkdir -p .bond/bin"));
    assert!(workflow_text.contains("cp target/debug/doublenot-bond .bond/bin/doublenot-bond"));
    assert!(workflow_text.contains("git config user.name \"doublenot-bond[bot]\""));
    assert!(workflow_text
        .contains("git config user.email \"doublenot-bond[bbot]@users.noreply.github.com\""));
    assert!(workflow_text.contains("ANTHROPIC_API_KEY: ${{ secrets.ANTHROPIC_API_KEY }}"));
    assert!(workflow_text.contains("GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}"));
    assert!(!workflow_text.contains("BOND_PROVIDER:"));
    assert!(!workflow_text.contains("BOND_MODEL:"));
    assert!(workflow_text.contains("./.bond/bin/doublenot-bond --repo . --run-scheduled-issue"));
    assert!(workflow_text.contains("Resume existing issue branch"));
    assert!(workflow_text.contains("Verify repo changes before commit"));
    assert!(workflow_text.contains("Verification required but no commands configured."));
    assert!(workflow_text.contains("exit 1"));
    let verify_index = workflow_text
        .find("Verify repo changes before commit")
        .expect("verify step present");
    let git_add_index = workflow_text.find("git add -A").expect("git add present");
    assert!(
        verify_index < git_add_index,
        "verification should happen before staging"
    );
    assert!(workflow_text.contains("refs/remotes/origin/$ISSUE_BRANCH"));
    assert!(workflow_text.contains("git checkout -B \"$ISSUE_BRANCH\" \"origin/$ISSUE_BRANCH\""));
    assert!(workflow_text.contains("git status --short"));
    assert!(workflow_text.contains("git add -A"));
    assert!(workflow_text.contains("git commit -m \"bond: work on #$ISSUE_NUMBER\""));
    assert!(workflow_text.contains("git push --set-upstream origin \"$ISSUE_BRANCH\""));
    assert!(workflow_text
        .contains("gh pr list --head \"$ISSUE_BRANCH\" --base \"${{ github.ref_name }}\""));
    assert!(workflow_text
        .contains("gh pr create --base \"${{ github.ref_name }}\" --head \"$ISSUE_BRANCH\""));
    assert!(workflow_text.contains(
        "printf 'Closes #%s\\n\\nAutomated changes from doublenot-bond.\\n' \"$ISSUE_NUMBER\""
    ));
    assert!(!workflow_text.contains("--provider \"$BOND_PROVIDER\""));
    assert!(!workflow_text.contains("--model \"$BOND_MODEL\""));
}

#[test]
fn prompt_setup_workflow_preserves_existing_file_without_refresh() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let workflow_dir = repo.join(".github/workflows");
    fs::create_dir_all(&workflow_dir).expect("create workflow dir");
    fs::write(workflow_dir.join("bond.yml"), "name: custom\n").expect("write existing workflow");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup workflow")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "setup workflow should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Bond workflow already exists:"),
        "stdout: {stdout}"
    );

    let workflow_text =
        fs::read_to_string(workflow_dir.join("bond.yml")).expect("read existing workflow");
    assert_eq!(workflow_text, "name: custom\n");
}

#[test]
fn prompt_setup_workflow_refresh_overwrites_existing_file() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '15 * * * *'\n  provider: google\n  model: gemini-2.5-pro\n  model_reasoning: Use Gemini for scheduled monorepo work because it handles broad cross-package analysis well.\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
    )
    .expect("write config");

    let workflow_dir = repo.join(".github/workflows");
    fs::create_dir_all(&workflow_dir).expect("create workflow dir");
    fs::write(workflow_dir.join("bond.yml"), "name: stale\n").expect("write existing workflow");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup workflow refresh")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "workflow refresh should exit 0");

    let workflow_text =
        fs::read_to_string(workflow_dir.join("bond.yml")).expect("read refreshed workflow");
    serde_yaml::from_str::<Value>(&workflow_text).expect("parse refreshed workflow");
    assert!(workflow_text.contains("# Model reasoning:"));
    assert!(workflow_text.contains("Use Gemini for scheduled monorepo work because it handles broad cross-package analysis well."));
    assert!(workflow_text.contains("cron: '15 * * * *'"));
    assert!(workflow_text.contains("timeout-minutes: 30"));
    assert!(workflow_text.contains("ref: ${{ github.ref_name }}"));
    assert!(workflow_text.contains("cp target/debug/doublenot-bond .bond/bin/doublenot-bond"));
    assert!(workflow_text.contains("git config user.name \"doublenot-bond[bot]\""));
    assert!(workflow_text
        .contains("git config user.email \"doublenot-bond[bbot]@users.noreply.github.com\""));
    assert!(workflow_text.contains("GOOGLE_API_KEY: ${{ secrets.GOOGLE_API_KEY }}"));
    assert!(workflow_text.contains("Resume existing issue branch"));
    assert!(workflow_text.contains("Verify repo changes before commit"));
    assert!(workflow_text.contains("Verification required but no commands configured."));
    assert!(workflow_text.contains("exit 1"));
    assert!(workflow_text.contains("git commit -m \"bond: work on #$ISSUE_NUMBER\""));
    assert!(workflow_text.contains("git push --set-upstream origin \"$ISSUE_BRANCH\""));
    assert!(workflow_text
        .contains("gh pr create --base \"${{ github.ref_name }}\" --head \"$ISSUE_BRANCH\""));
    assert!(!workflow_text.contains("BOND_PROVIDER:"));
    assert!(!workflow_text.contains("BOND_MODEL:"));
}

#[test]
fn scheduled_run_without_eligible_issue_skips_setup_warning() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let init = Command::new("git")
        .arg("init")
        .current_dir(repo)
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init should succeed");

    let remote = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/acme/widgets.git",
        ])
        .current_dir(repo)
        .output()
        .expect("git remote add origin");
    assert!(
        remote.status.success(),
        "git remote add origin should succeed"
    );

    write_state_file(repo, true, true);

    let fake_gh = repo.join("fake-gh.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"list\" ]; then\n  printf '[]\\n'\n  exit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&fake_gh)
            .expect("fake gh metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_gh, permissions).expect("chmod fake gh");
    }

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--run-scheduled-issue")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "scheduled run should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stdout.contains("No eligible scheduled issue found."));
    assert!(!stderr.contains("Bond setup is not complete yet."));
}

#[test]
fn scheduled_run_skips_cleanly_when_autonomous_execution_is_disabled() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let init = Command::new("git")
        .arg("init")
        .current_dir(repo)
        .output()
        .expect("git init");
    assert!(init.status.success(), "git init should succeed");

    let remote = Command::new("git")
        .args([
            "remote",
            "add",
            "origin",
            "https://github.com/acme/widgets.git",
        ])
        .current_dir(repo)
        .output()
        .expect("git remote add origin");
    assert!(
        remote.status.success(),
        "git remote add origin should succeed"
    );

    write_state_file(repo, true, false);

    let fake_gh = repo.join("fake-gh.sh");
    let gh_log = repo.join("gh.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\\n' \"$*\" >> \"$BOND_GH_LOG\"\nexit 1\n",
    )
    .expect("write fake gh script");

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(&fake_gh)
            .expect("fake gh metadata")
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&fake_gh, permissions).expect("chmod fake gh");
    }

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--run-scheduled-issue")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &gh_log)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "scheduled run should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(
            "Scheduled automation is disabled. Use /setup complete to enable --run-scheduled-issue."
        ),
        "stdout: {stdout}"
    );
    assert!(
        !gh_log.exists(),
        "scheduled run should not invoke gh while autonomous mode is disabled"
    );
}

#[test]
fn prompt_setup_complete_enables_autonomous_execution() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup complete")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "setup complete should exit 0");

    let state_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(state_text.contains("configured: true"));
    assert!(state_text.contains("autonomous_enabled: true"));
}

#[test]
fn prompt_setup_reset_disables_autonomous_execution() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup complete")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond setup complete");
    assert!(output.status.success(), "setup complete should exit 0");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup reset")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond setup reset");

    assert!(output.status.success(), "setup reset should exit 0");

    let state_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(state_text.contains("configured: false"));
    assert!(state_text.contains("autonomous_enabled: false"));
}

#[test]
fn prompt_status_uses_config_provider_and_model_when_flags_are_omitted() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '0 * * * *'\n  provider: openai\n  model: gpt-5.4\n  model_reasoning: Use the configured OpenAI defaults when CLI flags are omitted.\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
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
        .arg("--prompt")
        .arg("/status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("provider: openai"));
    assert!(stdout.contains("provider_source: .bond/config.yml"));
    assert!(stdout.contains("model: gpt-5.4"));
    assert!(stdout.contains("model_source: .bond/config.yml"));
    assert!(stdout.contains("automation_provider_matches_runtime: true"));
}

#[test]
fn prompt_status_reports_cli_sources_when_flags_are_explicit() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '0 * * * *'\n  provider: openai\n  model: gpt-5.4\n  model_reasoning: Use config defaults unless the operator passes explicit flags.\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
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
        .arg("--model")
        .arg("claude-sonnet-4-20250514")
        .arg("--prompt")
        .arg("/status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("provider: anthropic"));
    assert!(stdout.contains("provider_source: --provider flag"));
    assert!(stdout.contains("model: claude-sonnet-4-20250514"));
    assert!(stdout.contains("model_source: --model flag"));
}

#[test]
fn prompt_setup_status_uses_default_automation_when_config_omits_it() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let bond_dir = repo.join(".bond");
    fs::create_dir_all(&bond_dir).expect("create .bond directory");
    fs::write(
        bond_dir.join("config.yml"),
        "version: 1\nexecutable_path: .bond/bin/doublenot-bond\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 10\n",
    )
    .expect("write legacy-style config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/setup status")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "setup status should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("provider: ollama"));
    assert!(stdout.contains("provider_source: --provider flag"));
    assert!(stdout.contains("model_source: .bond/config.yml"));
    assert!(stdout.contains("automation_schedule_cron: 0 * * * *"));
    assert!(stdout.contains("automation_provider: anthropic"));
    assert!(stdout.contains("automation_model_reasoning: <none>"));
}

#[test]
fn prompt_tree_respects_deny_dir_flag() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--deny-dir")
        .arg(".bond")
        .arg("--prompt")
        .arg("/tree .bond 1")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "tree should be blocked by deny-dir"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Access denied"),
        "stderr should mention denied path: {stderr}"
    );
}
