use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

const BOND_DIR: &str = ".bond";
const BOND_BIN_DIR: &str = "bin";
const GITHUB_DIR: &str = ".github";
const ISSUE_TEMPLATE_DIR: &str = "ISSUE_TEMPLATE";
const IDENTITY_FILE: &str = "IDENTITY.md";
const PERSONALITY_FILE: &str = "PERSONALITY.md";
const JOURNAL_FILE: &str = "JOURNAL.md";
const CONFIG_FILE: &str = "config.yml";
const STATE_FILE: &str = "state.yml";
const ISSUE_TEMPLATE_CONFIG_FILE: &str = "config.yml";
const SETUP_ISSUE_TEMPLATE_FILE: &str = "bond-setup.md";
const TASK_ISSUE_TEMPLATE_FILE: &str = "bond-task.md";
const DEBUG_ISSUE_TEMPLATE_FILE: &str = "bond-debug.md";

#[derive(Debug, Clone)]
pub struct BondPaths {
    pub repo_root: PathBuf,
    pub bond_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub issue_template_dir: PathBuf,
    pub identity_file: PathBuf,
    pub personality_file: PathBuf,
    pub journal_file: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    pub issue_template_config_file: PathBuf,
    pub setup_issue_template_file: PathBuf,
    pub task_issue_template_file: PathBuf,
    pub debug_issue_template_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupIssue {
    pub number: Option<u64>,
    pub state: Option<String>,
    pub url: Option<String>,
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentIssue {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub label: String,
    #[serde(default)]
    pub last_action: Option<String>,
    #[serde(default)]
    pub last_action_at: Option<String>,
}

impl CurrentIssue {
    pub fn apply_action(&mut self, action: &str) {
        self.last_action = Some(action.to_string());
        self.last_action_at = Some(now_timestamp());
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoCommand {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowCommands {
    #[serde(default)]
    pub test: Vec<RepoCommand>,
    #[serde(default)]
    pub lint: Vec<RepoCommand>,
}

impl Default for WorkflowCommands {
    fn default() -> Self {
        default_workflow_commands()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueWorkflow {
    #[serde(default)]
    pub eligible_labels: Vec<String>,
    #[serde(default)]
    pub priority_labels: Vec<String>,
    #[serde(default = "default_require_prompt_contract")]
    pub require_prompt_contract: bool,
    #[serde(default = "default_issue_history_limit")]
    pub issue_history_limit: usize,
}

impl Default for IssueWorkflow {
    fn default() -> Self {
        default_issue_workflow()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondSettings {
    pub version: u32,
    pub executable_path: String,
    #[serde(default)]
    pub commands: WorkflowCommands,
    #[serde(default)]
    pub issues: IssueWorkflow,
}

impl Default for BondSettings {
    fn default() -> Self {
        Self {
            version: 1,
            executable_path: default_executable_path(),
            commands: default_workflow_commands(),
            issues: default_issue_workflow(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BondState {
    pub configured: bool,
    pub autonomous_enabled: bool,
    pub setup_issue: Option<SetupIssue>,
    #[serde(default)]
    pub last_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub current_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub issue_history: Vec<CurrentIssue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondConfig {
    pub version: u32,
    pub configured: bool,
    pub autonomous_enabled: bool,
    pub executable_path: String,
    pub setup_issue: Option<SetupIssue>,
    #[serde(default)]
    pub last_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub current_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub issue_history: Vec<CurrentIssue>,
    #[serde(default)]
    pub commands: WorkflowCommands,
    #[serde(default)]
    pub issues: IssueWorkflow,
}

impl Default for BondConfig {
    fn default() -> Self {
        Self::from_parts(BondSettings::default(), BondState::default())
    }
}

impl BondConfig {
    fn from_parts(settings: BondSettings, state: BondState) -> Self {
        Self {
            version: settings.version,
            configured: state.configured,
            autonomous_enabled: state.autonomous_enabled,
            executable_path: settings.executable_path,
            setup_issue: state.setup_issue,
            last_issue: state.last_issue,
            current_issue: state.current_issue,
            issue_history: state.issue_history,
            commands: settings.commands,
            issues: settings.issues,
        }
    }
}

#[derive(Debug, Clone)]
pub struct BondRuntimeContext {
    pub paths: BondPaths,
    pub identity: String,
    pub personality: String,
    pub journal: String,
    pub config: BondConfig,
}

impl BondPaths {
    pub fn new(repo_root: PathBuf) -> Result<Self> {
        let repo_root = repo_root
            .canonicalize()
            .with_context(|| format!("failed to canonicalize {}", repo_root.display()))?;
        let bond_dir = repo_root.join(BOND_DIR);
        let bin_dir = bond_dir.join(BOND_BIN_DIR);
        let github_dir = repo_root.join(GITHUB_DIR);
        let issue_template_dir = github_dir.join(ISSUE_TEMPLATE_DIR);
        Ok(Self {
            repo_root,
            bond_dir: bond_dir.clone(),
            bin_dir,
            issue_template_dir: issue_template_dir.clone(),
            identity_file: bond_dir.join(IDENTITY_FILE),
            personality_file: bond_dir.join(PERSONALITY_FILE),
            journal_file: bond_dir.join(JOURNAL_FILE),
            config_file: bond_dir.join(CONFIG_FILE),
            state_file: bond_dir.join(STATE_FILE),
            issue_template_config_file: issue_template_dir.join(ISSUE_TEMPLATE_CONFIG_FILE),
            setup_issue_template_file: issue_template_dir.join(SETUP_ISSUE_TEMPLATE_FILE),
            task_issue_template_file: issue_template_dir.join(TASK_ISSUE_TEMPLATE_FILE),
            debug_issue_template_file: issue_template_dir.join(DEBUG_ISSUE_TEMPLATE_FILE),
        })
    }

    pub fn ensure_bond_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.bin_dir)
            .with_context(|| format!("failed to create {}", self.bin_dir.display()))
    }

    pub fn bootstrap_bond_files(&self) -> Result<bool> {
        self.ensure_bond_dir()?;

        let mut created_any = false;
        created_any |= write_if_missing(
            &self.identity_file,
            &default_identity_contents(&self.repo_root),
        )?;
        created_any |= write_if_missing(
            &self.personality_file,
            &default_personality_contents(&self.repo_root),
        )?;
        created_any |= write_if_missing(&self.journal_file, &default_journal_contents())?;

        if !self.config_file.exists() {
            self.save_bond_settings(&BondSettings::default())?;
            created_any = true;
        }

        if !self.state_file.exists() {
            let state = self.load_legacy_bond_state().unwrap_or_default();
            self.save_bond_state(&state)?;
            created_any = true;
        }

        created_any |= self.bootstrap_github_files()?;

        Ok(created_any)
    }

    pub fn bootstrap_github_files(&self) -> Result<bool> {
        fs::create_dir_all(&self.issue_template_dir)
            .with_context(|| format!("failed to create {}", self.issue_template_dir.display()))?;

        let mut created_any = false;
        created_any |= write_if_missing(
            &self.issue_template_config_file,
            &default_issue_template_config_contents(),
        )?;
        created_any |= write_if_missing(
            &self.setup_issue_template_file,
            &default_setup_issue_template_contents(&self.repo_root),
        )?;
        created_any |= write_if_missing(
            &self.task_issue_template_file,
            &default_task_issue_template_contents(),
        )?;
        created_any |= write_if_missing(
            &self.debug_issue_template_file,
            &default_debug_issue_template_contents(),
        )?;

        Ok(created_any)
    }

    pub fn load_bond_config(&self) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let state = self.load_bond_state()?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn load_bond_settings(&self) -> Result<BondSettings> {
        let text = fs::read_to_string(&self.config_file)
            .with_context(|| format!("failed to read {}", self.config_file.display()))?;
        let settings = serde_yaml::from_str(&text)
            .with_context(|| format!("failed to parse {}", self.config_file.display()))?;
        Ok(settings)
    }

    pub fn load_bond_state(&self) -> Result<BondState> {
        if self.state_file.exists() {
            let text = fs::read_to_string(&self.state_file)
                .with_context(|| format!("failed to read {}", self.state_file.display()))?;
            let state = serde_yaml::from_str(&text)
                .with_context(|| format!("failed to parse {}", self.state_file.display()))?;
            Ok(state)
        } else {
            self.load_legacy_bond_state()
        }
    }

    fn load_legacy_bond_state(&self) -> Result<BondState> {
        if !self.config_file.exists() {
            return Ok(BondState::default());
        }

        let text = fs::read_to_string(&self.config_file)
            .with_context(|| format!("failed to read {}", self.config_file.display()))?;
        let config: BondConfig = serde_yaml::from_str(&text).with_context(|| {
            format!(
                "failed to parse legacy state from {}",
                self.config_file.display()
            )
        })?;
        Ok(BondState {
            configured: config.configured,
            autonomous_enabled: config.autonomous_enabled,
            setup_issue: config.setup_issue,
            last_issue: config.last_issue,
            current_issue: config.current_issue,
            issue_history: config.issue_history,
        })
    }

    pub fn executable_target_path(&self, config: &BondConfig) -> PathBuf {
        let configured = PathBuf::from(&config.executable_path);
        if configured.is_absolute() {
            configured
        } else {
            self.repo_root.join(configured)
        }
    }

    pub fn ensure_runtime_executable(
        &self,
        config: &BondConfig,
        current_executable: &Path,
    ) -> Result<bool> {
        let target = self.executable_target_path(config);
        if current_executable == target {
            return Ok(false);
        }

        let should_copy = if !target.exists() {
            true
        } else {
            let source_meta = fs::metadata(current_executable).with_context(|| {
                format!(
                    "failed to read metadata for {}",
                    current_executable.display()
                )
            })?;
            let target_meta = fs::metadata(&target)
                .with_context(|| format!("failed to read metadata for {}", target.display()))?;
            source_meta.len() != target_meta.len()
                || source_meta.modified().ok() > target_meta.modified().ok()
        };

        if !should_copy {
            return Ok(false);
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }

        fs::copy(current_executable, &target).with_context(|| {
            format!(
                "failed to copy executable from {} to {}",
                current_executable.display(),
                target.display()
            )
        })?;

        set_executable_permissions(&target)?;
        Ok(true)
    }

    pub fn save_bond_settings(&self, settings: &BondSettings) -> Result<()> {
        let text = serde_yaml::to_string(settings).context("failed to serialize bond settings")?;
        fs::write(&self.config_file, text)
            .with_context(|| format!("failed to write {}", self.config_file.display()))
    }

    pub fn save_bond_state(&self, state: &BondState) -> Result<()> {
        let text = serde_yaml::to_string(state).context("failed to serialize bond state")?;
        fs::write(&self.state_file, text)
            .with_context(|| format!("failed to write {}", self.state_file.display()))
    }

    pub fn set_configured(&self, configured: bool) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let mut state = self.load_bond_state()?;
        state.configured = configured;
        if !configured {
            state.autonomous_enabled = false;
        }
        self.save_bond_state(&state)?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn set_autonomous_enabled(&self, enabled: bool) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let mut state = self.load_bond_state()?;
        state.autonomous_enabled = enabled && state.configured;
        self.save_bond_state(&state)?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn set_setup_issue(&self, setup_issue: Option<SetupIssue>) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let mut state = self.load_bond_state()?;
        state.setup_issue = setup_issue;
        self.save_bond_state(&state)?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn set_current_issue(
        &self,
        current_issue: Option<CurrentIssue>,
        action: Option<&str>,
    ) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let mut state = self.load_bond_state()?;
        let history_limit = settings.issues.issue_history_limit;
        if let Some(mut issue) = current_issue {
            if let Some(action) = action {
                issue.apply_action(action);
            }
            state.last_issue = Some(issue.clone());
            remember_issue(&mut state.issue_history, &issue, history_limit);
            state.current_issue = Some(issue);
        } else if let Some(mut issue) = state.current_issue.clone() {
            if let Some(action) = action {
                issue.apply_action(action);
            }
            state.last_issue = Some(issue);
            if let Some(issue) = state.last_issue.as_ref() {
                remember_issue(&mut state.issue_history, issue, history_limit);
            }
            state.current_issue = None;
        } else {
            state.current_issue = None;
        }
        self.save_bond_state(&state)?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn append_journal_entry(&self, title: &str, body: &str) -> Result<()> {
        let existing = fs::read_to_string(&self.journal_file).unwrap_or_default();
        let mut updated = String::new();
        updated.push_str("# Journal\n\n");
        updated.push_str(&format!("## {}\n\n{}\n\n", title, body.trim()));

        let existing_body = existing
            .strip_prefix("# Journal\n\n")
            .unwrap_or(existing.as_str());
        updated.push_str(existing_body);

        fs::write(&self.journal_file, updated)
            .with_context(|| format!("failed to write {}", self.journal_file.display()))
    }

    pub fn load_runtime_context(&self) -> Result<BondRuntimeContext> {
        Ok(BondRuntimeContext {
            paths: self.clone(),
            identity: fs::read_to_string(&self.identity_file)
                .with_context(|| format!("failed to read {}", self.identity_file.display()))?,
            personality: fs::read_to_string(&self.personality_file)
                .with_context(|| format!("failed to read {}", self.personality_file.display()))?,
            journal: fs::read_to_string(&self.journal_file)
                .with_context(|| format!("failed to read {}", self.journal_file.display()))?,
            config: self.load_bond_config()?,
        })
    }
}

impl BondRuntimeContext {
    pub fn refresh_config(&mut self) -> Result<()> {
        self.config = self.paths.load_bond_config()?;
        Ok(())
    }
}

fn remember_issue(history: &mut Vec<CurrentIssue>, issue: &CurrentIssue, limit: usize) {
    history.retain(|entry| entry.number != issue.number);
    history.insert(0, issue.clone());
    history.truncate(limit);
}

fn now_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn write_if_missing(path: &Path, contents: &str) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(true)
}

fn default_identity_contents(repo_root: &Path) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this repository");

    format!(
        "# Identity\n\n## Repository\n- Name: {repo_name}\n- Purpose: Describe what this repository is for.\n\n## Allowed Work\n- Source code\n- Tests\n- Documentation\n\n## Guardrails\n- Confirm what the agent should never modify without human approval.\n- Record any repo-specific rules here.\n"
    )
}

fn default_personality_contents(repo_root: &Path) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this repository");

    format!(
        "# Personality\n\nThe bond for {repo_name} should communicate clearly, keep changes focused, and verify work before claiming success.\n\n## Tone\n- Direct\n- Practical\n- Calm\n\n## Collaboration\n- Prefer small, auditable changes\n- Explain blockers plainly\n- Respect the repository's existing style\n"
    )
}

fn default_journal_contents() -> String {
    "# Journal\n\n## Bond Initialized\n\nThe .bond runtime context was created for this repository.\n".to_string()
}

fn default_issue_template_config_contents() -> String {
    "blank_issues_enabled: false\ncontact_links: []\n".to_string()
}

fn default_workflow_commands() -> WorkflowCommands {
    WorkflowCommands {
        test: vec![RepoCommand {
            program: "cargo".to_string(),
            args: vec!["test".to_string()],
            description: "cargo test".to_string(),
        }],
        lint: vec![
            RepoCommand {
                program: "cargo".to_string(),
                args: vec!["fmt".to_string(), "--".to_string(), "--check".to_string()],
                description: "cargo fmt -- --check".to_string(),
            },
            RepoCommand {
                program: "cargo".to_string(),
                args: vec![
                    "clippy".to_string(),
                    "--all-targets".to_string(),
                    "--".to_string(),
                    "-D".to_string(),
                    "warnings".to_string(),
                ],
                description: "cargo clippy --all-targets -- -D warnings".to_string(),
            },
        ],
    }
}

fn default_issue_workflow() -> IssueWorkflow {
    IssueWorkflow {
        eligible_labels: vec!["bond-debug".to_string(), "bond-task".to_string()],
        priority_labels: vec!["bond-debug".to_string(), "bond-task".to_string()],
        require_prompt_contract: true,
        issue_history_limit: default_issue_history_limit(),
    }
}

fn default_require_prompt_contract() -> bool {
    true
}

fn default_issue_history_limit() -> usize {
    10
}

fn default_setup_issue_template_contents(repo_root: &Path) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this repository");

    format!(
        "---\nname: Bond Setup\nabout: Configure the .bond runtime for this repository\ntitle: \"Bond setup: configure {repo_name}\"\nlabels: bond-setup\nassignees: ''\n---\n\n## Inputs\n\n- Review [.bond/IDENTITY.md](../.bond/IDENTITY.md) and replace placeholders with repository-specific intent and guardrails.\n- Review [.bond/PERSONALITY.md](../.bond/PERSONALITY.md) and set the collaboration style this bond should follow.\n- Confirm whether autonomous work should remain disabled until onboarding is reviewed.\n\n## Expected Output\n\n- `.bond` files reflect the repository's real scope and constraints.\n- Prompt-contract issue templates are available under `.github/ISSUE_TEMPLATE/`.\n- The repository is ready for `/setup complete` after human review.\n\n## Constraints\n\n- Do not enable autonomous execution until the repository owner has reviewed the `.bond` files.\n- Keep repository-specific rules in `.bond`, not in ad hoc chat context.\n- Preserve existing repo conventions and contribution rules.\n\n## Edge Cases\n\n- If the repository is not hosted on GitHub, document that blocker before marking setup complete.\n- If existing issue templates already exist, reconcile them instead of deleting them blindly.\n- If `.bond` conflicts with project policy, record the alternative onboarding path here.\n\n## Acceptance Criteria\n\n- [ ] `.bond/IDENTITY.md` is customized for this repository.\n- [ ] `.bond/PERSONALITY.md` is customized for this repository.\n- [ ] `.bond/JOURNAL.md` has an initial repository-specific entry.\n- [ ] The repository owner agrees the bond can begin work.\n- [ ] `doublenot-bond --prompt \"/setup complete\"` is the next step after review.\n"
    )
}

fn default_task_issue_template_contents() -> String {
    "---\nname: Bond Task\nabout: Give the bond a task using the prompt-contract structure\ntitle: \"Task: \"\nlabels: bond-task\nassignees: ''\n---\n\n## Inputs\n\nDescribe the codebase context, relevant files, and what the bond should look at first.\n\n## Expected Output\n\nDescribe the desired code, docs, or behavior change.\n\n## Constraints\n\nList any architectural rules, scope boundaries, or forbidden approaches.\n\n## Edge Cases\n\nList failure modes, tricky cases, or compatibility concerns.\n\n## Acceptance Criteria\n\nList the concrete checks that determine when this task is complete.\n".to_string()
}

fn default_debug_issue_template_contents() -> String {
    "---\nname: Bond Debug\nabout: Ask the bond to diagnose and fix a bug using a debugging contract\ntitle: \"Debug: \"\nlabels: bond-task, bug\nassignees: ''\n---\n\n## Inputs\n\nDescribe the bug, failing behavior, reproduction steps, and relevant logs.\n\n## Expected Output\n\nDescribe the fix, explanation, and any tests or instrumentation you expect.\n\n## Constraints\n\nList limits on risky changes, migrations, or files the bond should avoid.\n\n## Edge Cases\n\nCall out intermittent failures, environment differences, or known false leads.\n\n## Acceptance Criteria\n\nList the exact reproduction that should stop failing and the checks that should pass afterward.\n".to_string()
}

fn default_executable_path() -> String {
    let file_name = if cfg!(windows) {
        "doublenot-bond.exe"
    } else {
        "doublenot-bond"
    };
    format!(".bond/bin/{file_name}")
}

fn set_executable_permissions(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)
            .with_context(|| format!("failed to read metadata for {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("failed to set permissions for {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}
