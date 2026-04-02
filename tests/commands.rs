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
}

#[test]
fn prompt_test_runs_default_configured_workflow() {
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
        output.status.success(),
        "default configured /test should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("> cargo test"),
        "stdout should show configured test command: {stdout}"
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

#[test]
fn prompt_test_uses_configured_commands() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncommands:\n  test:\n    - program: sh\n      args:\n        - -c\n        - 'printf custom-test\\n'\n      description: custom test\n  lint: []\n",
    )
    .expect("write config");

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

    assert!(output.status.success(), "configured /test should exit 0");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("> custom test"));
    assert!(stdout.contains("custom-test"));
}

#[test]
fn prompt_lint_fails_when_no_commands_are_configured() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncommands:\n  test: []\n  lint: []\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/lint")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(!output.status.success(), "empty lint config should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No commands configured for this workflow"));
}

#[test]
fn prompt_issues_next_returns_highest_priority_eligible_issue() {
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

    let fake_gh = repo.join("fake-gh-issues.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"list\" ]; then\ncat <<'JSON'\n[{\"number\":8,\"title\":\"Task issue\",\"url\":\"https://github.com/acme/widgets/issues/8\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nfoo\\n## Expected Output\\nbar\\n## Constraints\\nbaz\\n## Edge Cases\\nqux\\n## Acceptance Criteria\\nready\"},{\"number\":3,\"title\":\"Debug issue\",\"url\":\"https://github.com/acme/widgets/issues/3\",\"labels\":[{\"name\":\"bond-debug\"}],\"body\":\"## Inputs\\nfoo\\n## Expected Output\\nbar\\n## Constraints\\nbaz\\n## Edge Cases\\nqux\\n## Acceptance Criteria\\nready\"},{\"number\":9,\"title\":\"Incomplete issue\",\"url\":\"https://github.com/acme/widgets/issues/9\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nfoo\"}]\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh issues script");

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
        .arg("/issues next")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues next should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Next issue: #3 [bond-debug] Debug issue"),
        "stdout should prefer debug issue: {stdout}"
    );
    assert!(stdout.contains("https://github.com/acme/widgets/issues/3"));
    assert!(
        !stdout.contains("Incomplete issue"),
        "incomplete prompt-contract issue should be filtered out: {stdout}"
    );

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue:"));
    assert!(config_text.contains("number: 3"));
    assert!(config_text.contains("title: Debug issue"));
}

#[test]
fn prompt_issues_list_reports_no_eligible_issues() {
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

    let fake_gh = repo.join("fake-gh-empty.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"list\" ]; then\nprintf '[]\\n'\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh empty script");

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
        .arg("/issues list")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues list should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("No eligible issues found."));
}

#[test]
fn prompt_issues_select_persists_requested_issue() {
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

    let fake_gh = repo.join("fake-gh-select.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ] && [ \"$3\" = \"14\" ]; then\ncat <<'JSON'\n{\"number\":14,\"title\":\"Manual issue\",\"url\":\"https://github.com/acme/widgets/issues/14\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nInspect src\\n## Expected Output\\nManual selection works\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle missing config\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh select script");

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
        .arg("/issues select 14")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues select should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Selected issue: #14 [bond-task] Manual issue"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("number: 14"));
    assert!(config_text.contains("title: Manual issue"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Selected"));
    assert!(journal_text.contains("Manual issue"));
}

#[test]
fn prompt_issues_select_rejects_ineligible_issue() {
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

    let fake_gh = repo.join("fake-gh-select-invalid.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ] && [ \"$3\" = \"22\" ]; then\ncat <<'JSON'\n{\"number\":22,\"title\":\"Wrong label\",\"url\":\"https://github.com/acme/widgets/issues/22\",\"labels\":[{\"name\":\"docs\"}],\"body\":\"## Inputs\\nInspect docs\\n## Expected Output\\nExplain behavior\\n## Constraints\\nNo code changes\\n## Edge Cases\\nNone\\n## Acceptance Criteria\\nLooks good\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh invalid select script");

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
        .arg("/issues select 22")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "issues select should reject ineligible issues"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not satisfy the configured intake workflow"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue: null"));
}

#[test]
fn prompt_issues_current_and_clear_use_persisted_selection() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let current_output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues current")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        current_output.status.success(),
        "issues current should succeed"
    );
    let current_stdout = String::from_utf8_lossy(&current_output.stdout);
    assert!(current_stdout.contains("Current issue: #12 [bond-task] Existing issue"));

    let clear_output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues clear")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(clear_output.status.success(), "issues clear should succeed");
    let clear_stdout = String::from_utf8_lossy(&clear_output.stdout);
    assert!(clear_stdout.contains("Cleared current issue selection."));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue: null"));
}

#[test]
fn prompt_issues_sync_refreshes_current_issue_metadata() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-sync-open.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ]; then\ncat <<'JSON'\n{\"number\":12,\"title\":\"Renamed issue\",\"url\":\"https://github.com/acme/widgets/issues/12\",\"state\":\"OPEN\",\"labels\":[{\"name\":\"bond-debug\"}],\"body\":\"## Inputs\\nLook at src\\n## Expected Output\\nWorking change\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle empty input\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh sync script");

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
        .arg("/issues sync")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues sync should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Synchronized current issue: #12 [bond-debug] Renamed issue"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("title: Renamed issue"));
    assert!(config_text.contains("label: bond-debug"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Synced"));
}

#[test]
fn prompt_issues_sync_clears_closed_issue_selection() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-sync-closed.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ]; then\ncat <<'JSON'\n{\"number\":12,\"title\":\"Existing issue\",\"url\":\"https://github.com/acme/widgets/issues/12\",\"state\":\"CLOSED\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nLook at src\\n## Expected Output\\nWorking change\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle empty input\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh closed sync script");

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
        .arg("/issues sync")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "issues sync should succeed for closed issue"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Current issue is closed on GitHub. Cleared selection."));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue: null"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text
        .contains("Cleared current issue selection after GitHub reported issue #12 as closed."));
}

#[test]
fn prompt_issues_reopen_restores_issue_as_current_selection() {
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

    let fake_gh = repo.join("fake-gh-reopen.sh");
    let log_path = repo.join("gh-reopen.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\n' \"$*\" >> \"$BOND_GH_LOG\"\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"reopen\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"comment\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ] && [ \"$3\" = \"18\" ]; then\ncat <<'JSON'\n{\"number\":18,\"title\":\"Reopened issue\",\"url\":\"https://github.com/acme/widgets/issues/18\",\"state\":\"OPEN\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nInspect src\\n## Expected Output\\nRestore work\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle stale state\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh reopen script");

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
        .arg("/issues reopen 18 restarting after regression")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &log_path)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues reopen should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reopened issue: #18 [bond-task] Reopened issue"));

    let gh_log = fs::read_to_string(log_path).expect("read gh reopen log");
    assert!(gh_log.contains("issue reopen 18 --repo acme/widgets"));
    assert!(
        gh_log.contains("issue comment 18 --repo acme/widgets --body restarting after regression")
    );
    assert!(gh_log
        .contains("issue view 18 --repo acme/widgets --json number,title,body,url,state,labels"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("number: 18"));
    assert!(config_text.contains("title: Reopened issue"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Reopened"));
    assert!(journal_text.contains("restarting after regression"));
}

#[test]
fn prompt_issues_reopen_rejects_ineligible_issue() {
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

    let fake_gh = repo.join("fake-gh-reopen-invalid.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"reopen\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ] && [ \"$3\" = \"25\" ]; then\ncat <<'JSON'\n{\"number\":25,\"title\":\"Wrong label\",\"url\":\"https://github.com/acme/widgets/issues/25\",\"state\":\"OPEN\",\"labels\":[{\"name\":\"docs\"}],\"body\":\"## Inputs\\nInspect docs\\n## Expected Output\\nExplain behavior\\n## Constraints\\nNo code changes\\n## Edge Cases\\nNone\\n## Acceptance Criteria\\nLooks good\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh invalid reopen script");

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
        .arg("/issues reopen 25")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "issues reopen should reject ineligible issues"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not satisfy the configured intake workflow"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue: null"));
}

#[test]
fn prompt_issues_reopen_current_uses_last_recorded_issue() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 44\n  title: Last issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\ncurrent_issue: null\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-reopen-current.sh");
    let log_path = repo.join("gh-reopen-current.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\n' \"$*\" >> \"$BOND_GH_LOG\"\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"reopen\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"comment\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ] && [ \"$3\" = \"44\" ]; then\ncat <<'JSON'\n{\"number\":44,\"title\":\"Last issue\",\"url\":\"https://github.com/acme/widgets/issues/44\",\"state\":\"OPEN\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nInspect src\\n## Expected Output\\nResume work\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle stale state\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh reopen-current script");

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
        .arg("/issues reopen-current retrying after triage")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &log_path)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "issues reopen-current should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Reopened issue: #44 [bond-task] Last issue"));

    let gh_log = fs::read_to_string(log_path).expect("read gh reopen-current log");
    assert!(gh_log.contains("issue reopen 44 --repo acme/widgets"));
    assert!(gh_log.contains("issue comment 44 --repo acme/widgets --body retrying after triage"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("last_issue:"));
    assert!(config_text.contains("current_issue:"));
    assert!(config_text.contains("number: 44"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Reopened"));
    assert!(journal_text.contains("retrying after triage"));
}

#[test]
fn prompt_issues_reopen_current_fails_without_last_issue() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue: null\ncurrent_issue: null\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues reopen-current")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "issues reopen-current should fail without last_issue"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No last issue recorded yet"));
}

#[test]
fn prompt_issues_previous_restores_prior_issue_from_history() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\ncurrent_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\nissue_history:\n  - number: 44\n    title: Current issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\n  - number: 18\n    title: Prior issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues previous")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues previous should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Restored previous issue: #18 [bond-debug] Prior issue"));
    assert!(stdout.contains("https://github.com/acme/widgets/issues/18"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    let config: Value = serde_yaml::from_str(&config_text).expect("parse config yaml");
    let current_issue = config
        .get("current_issue")
        .expect("current issue should exist after restore");
    assert_eq!(
        current_issue.get("number").and_then(Value::as_u64),
        Some(18)
    );
    assert_eq!(
        current_issue.get("last_action").and_then(Value::as_str),
        Some("restored")
    );
    assert!(
        current_issue
            .get("last_action_at")
            .and_then(Value::as_str)
            .is_some(),
        "restore should stamp a timestamp"
    );

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Restored"));
    assert!(journal_text.contains("Prior issue"));
}

#[test]
fn prompt_issues_previous_fails_without_prior_history() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue: null\ncurrent_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\nissue_history:\n  - number: 44\n    title: Current issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues previous")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        !output.status.success(),
        "issues previous should fail without a prior issue"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("No previous issue recorded yet"));
}

#[test]
fn prompt_issues_resume_prefers_current_issue() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\ncurrent_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\nissue_history:\n  - number: 44\n    title: Current issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues resume")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues resume should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resuming current issue: #44 [bond-task] Current issue"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    let config: Value = serde_yaml::from_str(&config_text).expect("parse config yaml");
    let current_issue = config
        .get("current_issue")
        .expect("current issue should exist");
    assert_eq!(
        current_issue.get("last_action").and_then(Value::as_str),
        Some("resumed")
    );
}

#[test]
fn prompt_issues_resume_falls_back_to_previous_issue() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 18\n  title: Prior issue\n  url: https://github.com/acme/widgets/issues/18\n  label: bond-debug\ncurrent_issue: null\nissue_history:\n  - number: 18\n    title: Prior issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues resume")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "issues resume should restore previous issue"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resumed previous issue: #18 [bond-debug] Prior issue"));
}

#[test]
fn prompt_issues_resume_falls_back_to_next_issue() {
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

    let fake_gh = repo.join("fake-gh-resume-next.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"list\" ]; then\ncat <<'JSON'\n[{\"number\":8,\"title\":\"Task issue\",\"url\":\"https://github.com/acme/widgets/issues/8\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nfoo\\n## Expected Output\\nbar\\n## Constraints\\nbaz\\n## Edge Cases\\nqux\\n## Acceptance Criteria\\nready\"}]\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh issues script");

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
        .arg("/issues resume")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "issues resume should select next issue when local state is empty"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Resumed next issue: #8 [bond-task] Task issue"));
    assert!(stdout.contains("https://github.com/acme/widgets/issues/8"));
}

#[test]
fn prompt_issues_history_lists_recent_recorded_issues() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 44\n  title: Last issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\n  last_action: selected\n  last_action_at: 2026-04-02T00:00:00Z\ncurrent_issue: null\nissue_history:\n  - number: 44\n    title: Last issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\n    last_action: selected\n    last_action_at: 2026-04-02T00:00:00Z\n  - number: 18\n    title: Reopened issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\n    last_action: reopened\n    last_action_at: 2026-04-01T12:00:00Z\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues history")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues history should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("#44 [bond-task] Last issue"));
    assert!(stdout.contains("https://github.com/acme/widgets/issues/44"));
    assert!(stdout.contains("action: selected"));
    assert!(stdout.contains("at: 2026-04-02T00:00:00Z"));
    assert!(stdout.contains("#18 [bond-debug] Reopened issue"));
    assert!(stdout.contains("https://github.com/acme/widgets/issues/18"));
    assert!(stdout.contains("action: reopened"));
    assert!(stdout.contains("at: 2026-04-01T12:00:00Z"));

    let first = stdout
        .find("#44 [bond-task] Last issue")
        .expect("first history entry");
    let second = stdout
        .find("#18 [bond-debug] Reopened issue")
        .expect("second history entry");
    assert!(
        first < second,
        "history should preserve newest-first ordering: {stdout}"
    );
}

#[test]
fn prompt_issues_history_filters_by_action() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue: null\ncurrent_issue: null\nissue_history:\n  - number: 44\n    title: Last issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\n    last_action: parked\n    last_action_at: 2026-04-02T00:00:00Z\n  - number: 18\n    title: Reopened issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\n    last_action: reopened\n    last_action_at: 2026-04-01T12:00:00Z\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues history reopened")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "filtered issues history should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History filters: reopened"));
    assert!(stdout.contains("#18 [bond-debug] Reopened issue"));
    assert!(stdout.contains("action: reopened"));
    assert!(!stdout.contains("#44 [bond-task] Last issue"));
}

#[test]
fn prompt_issues_history_filters_by_label_and_current() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");
    fs::write(
        config_dir.join("state.yml"),
        "configured: false\nautonomous_enabled: false\nsetup_issue: null\nlast_issue: null\ncurrent_issue:\n  number: 18\n  title: Active debug issue\n  url: https://github.com/acme/widgets/issues/18\n  label: bond-debug\n  last_action: resumed\n  last_action_at: 2026-04-02T00:00:00Z\nissue_history:\n  - number: 44\n    title: Parked task issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\n    last_action: parked\n    last_action_at: 2026-04-01T12:00:00Z\n  - number: 18\n    title: Active debug issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\n    last_action: resumed\n    last_action_at: 2026-04-02T00:00:00Z\n",
    )
    .expect("write state");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues history current label:bond-debug")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "history filtering by current and label should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("History filters: current, label:bond-debug"));
    assert!(stdout.contains("#18 [bond-debug] Active debug issue"));
    assert!(!stdout.contains("#44 [bond-task] Parked task issue"));
}

#[test]
fn prompt_issues_park_clears_current_issue_with_parked_action() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues park waiting on review")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues park should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parked current issue: #12 [bond-task] Existing issue"));

    let state_text = fs::read_to_string(state_file(repo)).expect("read state");
    let state: Value = serde_yaml::from_str(&state_text).expect("parse state yaml");
    assert!(state.get("current_issue").is_some());
    assert!(state.get("current_issue").is_some_and(Value::is_null));
    let last_issue = state.get("last_issue").expect("last issue should exist");
    assert_eq!(last_issue.get("number").and_then(Value::as_u64), Some(12));
    assert_eq!(
        last_issue.get("last_action").and_then(Value::as_str),
        Some("parked")
    );
    assert!(last_issue
        .get("last_action_at")
        .and_then(Value::as_str)
        .is_some());

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Parked"));
    assert!(journal_text.contains("waiting on review"));
}

#[test]
fn prompt_issues_previous_respects_configured_history_limit() {
    let temp = tempdir().expect("tempdir");
    let repo = temp.path();

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\nlast_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\ncurrent_issue:\n  number: 44\n  title: Current issue\n  url: https://github.com/acme/widgets/issues/44\n  label: bond-task\nissue_history:\n  - number: 44\n    title: Current issue\n    url: https://github.com/acme/widgets/issues/44\n    label: bond-task\n  - number: 18\n    title: Prior issue\n    url: https://github.com/acme/widgets/issues/18\n    label: bond-debug\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n  issue_history_limit: 1\ncommands:\n  test: []\n  lint: []\n",
    )
    .expect("write config");

    let output = bond_cmd()
        .arg("--repo")
        .arg(repo)
        .arg("--bond-runtime")
        .arg("--provider")
        .arg("ollama")
        .arg("--prompt")
        .arg("/issues previous")
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(
        output.status.success(),
        "issues previous should succeed with configured limit"
    );

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    let config: Value = serde_yaml::from_str(&config_text).expect("parse config yaml");
    let history = config
        .get("issue_history")
        .and_then(Value::as_sequence)
        .expect("issue history sequence");

    assert_eq!(
        history.len(),
        1,
        "configured history limit should be enforced"
    );
    assert_eq!(
        history[0].get("number").and_then(Value::as_u64),
        Some(18),
        "history should keep the restored issue as the newest retained entry"
    );
}

#[test]
fn prompt_issues_prompt_renders_execution_prompt_from_current_issue() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-view.sh");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"view\" ]; then\ncat <<'JSON'\n{\"number\":12,\"title\":\"Existing issue\",\"url\":\"https://github.com/acme/widgets/issues/12\",\"labels\":[{\"name\":\"bond-task\"}],\"body\":\"## Inputs\\nLook at src\\n## Expected Output\\nWorking change\\n## Constraints\\nKeep scope tight\\n## Edge Cases\\nHandle empty input\\n## Acceptance Criteria\\nTests pass\"}\nJSON\nexit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh view script");

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
        .arg("/issues prompt")
        .env("BOND_GH_BIN", &fake_gh)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues prompt should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Work on GitHub issue #12 [bond-task] Existing issue."));
    assert!(stdout.contains("## Inputs"));
    assert!(stdout.contains("Run relevant verification before concluding."));
}

#[test]
fn prompt_issues_comment_posts_to_current_issue() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-comment.sh");
    let log_path = repo.join("gh-comment.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\n' \"$*\" >> \"$BOND_GH_LOG\"\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"comment\" ]; then\n  exit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh comment script");

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
        .arg("/issues comment shipped fix")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &log_path)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues comment should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Commented on current issue."));
    let gh_log = fs::read_to_string(log_path).expect("read gh comment log");
    assert!(gh_log.contains("issue comment 12 --repo acme/widgets --body shipped fix"));
}

#[test]
fn prompt_issues_complete_closes_issue_and_clears_selection() {
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

    let config_dir = repo.join(".bond");
    fs::create_dir_all(&config_dir).expect("create .bond directory");
    fs::write(
        config_dir.join("config.yml"),
        "version: 1\nconfigured: false\nautonomous_enabled: false\nexecutable_path: .bond/bin/doublenot-bond\nsetup_issue: null\ncurrent_issue:\n  number: 12\n  title: Existing issue\n  url: https://github.com/acme/widgets/issues/12\n  label: bond-task\ncommands:\n  test: []\n  lint: []\nissues:\n  eligible_labels:\n    - bond-debug\n    - bond-task\n  priority_labels:\n    - bond-debug\n    - bond-task\n  require_prompt_contract: true\n",
    )
    .expect("write config");

    let fake_gh = repo.join("fake-gh-complete.sh");
    let log_path = repo.join("gh-complete.log");
    fs::write(
        &fake_gh,
        "#!/bin/sh\nprintf '%s\n' \"$*\" >> \"$BOND_GH_LOG\"\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"comment\" ]; then\n  exit 0\nfi\nif [ \"$1\" = \"issue\" ] && [ \"$2\" = \"close\" ]; then\n  exit 0\nfi\nprintf 'unexpected gh invocation: %s\\n' \"$*\" >&2\nexit 1\n",
    )
    .expect("write fake gh complete script");

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
        .arg("/issues complete done and verified")
        .env("BOND_GH_BIN", &fake_gh)
        .env("BOND_GH_LOG", &log_path)
        .stdin(Stdio::null())
        .output()
        .expect("run doublenot-bond");

    assert!(output.status.success(), "issues complete should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Completed current issue and cleared selection."));

    let gh_log = fs::read_to_string(log_path).expect("read gh complete log");
    assert!(gh_log.contains("issue comment 12 --repo acme/widgets --body done and verified"));
    assert!(gh_log.contains("issue close 12 --repo acme/widgets"));

    let config_text = fs::read_to_string(state_file(repo)).expect("read state");
    assert!(config_text.contains("current_issue: null"));

    let journal_text = fs::read_to_string(repo.join(".bond/JOURNAL.md")).expect("read journal");
    assert!(journal_text.contains("## Issue Completed"));
    assert!(journal_text.contains("done and verified"));
}
