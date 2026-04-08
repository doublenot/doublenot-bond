use crate::cli;
use anyhow::{Context, Result};
use serde::{Deserialize, Deserializer, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use yoagent::types::ThinkingLevel;

const BOND_DIR: &str = ".bond";
const BOND_BIN_DIR: &str = "bin";
const GITHUB_DIR: &str = ".github";
const WORKFLOWS_DIR: &str = "workflows";
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
const BOND_WORKFLOW_FILE: &str = "bond.yml";

#[derive(Debug, Clone)]
pub struct BondPaths {
    pub repo_root: PathBuf,
    pub bond_dir: PathBuf,
    pub bin_dir: PathBuf,
    pub github_workflows_dir: PathBuf,
    pub issue_template_dir: PathBuf,
    pub identity_file: PathBuf,
    pub personality_file: PathBuf,
    pub journal_file: PathBuf,
    pub config_file: PathBuf,
    pub state_file: PathBuf,
    pub bond_workflow_file: PathBuf,
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
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub test: Vec<RepoCommand>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub lint: Vec<RepoCommand>,
}

impl Default for WorkflowCommands {
    fn default() -> Self {
        default_workflow_commands()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationSettings {
    #[serde(default = "default_schedule_cron")]
    pub schedule_cron: String,
    #[serde(default = "default_automation_provider")]
    pub provider: String,
    #[serde(default = "default_automation_model")]
    pub model: String,
    #[serde(default = "default_automation_multiple_issues")]
    pub multiple_issues: bool,
    #[serde(
        alias = "model_reasoning",
        default = "default_automation_thinking_effort"
    )]
    pub thinking_effort: ThinkingLevel,
}

impl Default for AutomationSettings {
    fn default() -> Self {
        default_automation_settings()
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
    pub automation: AutomationSettings,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub commands: WorkflowCommands,
    #[serde(default)]
    pub issues: IssueWorkflow,
}

impl Default for BondSettings {
    fn default() -> Self {
        Self {
            version: 1,
            executable_path: default_executable_path(),
            automation: default_automation_settings(),
            commands: default_workflow_commands(),
            issues: default_issue_workflow(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduledTargetKind {
    Issue,
    PrFeedback,
    MergeWait,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledTarget {
    pub kind: ScheduledTargetKind,
    #[serde(default)]
    pub issue_number: Option<u64>,
    #[serde(default)]
    pub issue_title: Option<String>,
    #[serde(default)]
    pub issue_url: Option<String>,
    #[serde(default)]
    pub branch_name: Option<String>,
    #[serde(default)]
    pub pr_number: Option<u64>,
    #[serde(default)]
    pub pr_title: Option<String>,
    #[serde(default)]
    pub pr_url: Option<String>,
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
    #[serde(default)]
    pub scheduled_target: Option<ScheduledTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BondConfig {
    pub version: u32,
    pub configured: bool,
    pub autonomous_enabled: bool,
    pub executable_path: String,
    #[serde(default)]
    pub automation: AutomationSettings,
    pub setup_issue: Option<SetupIssue>,
    #[serde(default)]
    pub last_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub current_issue: Option<CurrentIssue>,
    #[serde(default)]
    pub issue_history: Vec<CurrentIssue>,
    #[serde(default)]
    pub scheduled_target: Option<ScheduledTarget>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
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
            automation: settings.automation,
            setup_issue: state.setup_issue,
            last_issue: state.last_issue,
            current_issue: state.current_issue,
            issue_history: state.issue_history,
            scheduled_target: state.scheduled_target,
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
        let github_workflows_dir = github_dir.join(WORKFLOWS_DIR);
        let issue_template_dir = github_dir.join(ISSUE_TEMPLATE_DIR);
        Ok(Self {
            repo_root,
            bond_dir: bond_dir.clone(),
            bin_dir,
            github_workflows_dir: github_workflows_dir.clone(),
            issue_template_dir: issue_template_dir.clone(),
            identity_file: bond_dir.join(IDENTITY_FILE),
            personality_file: bond_dir.join(PERSONALITY_FILE),
            journal_file: bond_dir.join(JOURNAL_FILE),
            config_file: bond_dir.join(CONFIG_FILE),
            state_file: bond_dir.join(STATE_FILE),
            bond_workflow_file: github_workflows_dir.join(BOND_WORKFLOW_FILE),
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
            created_any |= write_if_missing(
                &self.config_file,
                &default_bootstrap_bond_settings_contents(&BondSettings::default()),
            )?;
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

    pub fn install_bond_workflow(&self, force: bool) -> Result<bool> {
        fs::create_dir_all(&self.github_workflows_dir)
            .with_context(|| format!("failed to create {}", self.github_workflows_dir.display()))?;

        let settings = self.load_bond_settings()?;
        let contents = default_bond_workflow_contents(&settings);

        if force {
            fs::write(&self.bond_workflow_file, contents).with_context(|| {
                format!("failed to write {}", self.bond_workflow_file.display())
            })?;
            return Ok(true);
        }

        write_if_missing(&self.bond_workflow_file, &contents)
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
            scheduled_target: config.scheduled_target,
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

    pub fn set_scheduled_target(
        &self,
        scheduled_target: Option<ScheduledTarget>,
    ) -> Result<BondConfig> {
        let settings = self.load_bond_settings()?;
        let mut state = self.load_bond_state()?;
        state.scheduled_target = scheduled_target;
        self.save_bond_state(&state)?;
        Ok(BondConfig::from_parts(settings, state))
    }

    pub fn set_schedule_cron(&self, cron: &str) -> Result<()> {
        let text = fs::read_to_string(&self.config_file)
            .with_context(|| format!("failed to read {}", self.config_file.display()))?;
        let mut updated_lines: Vec<String> = Vec::new();
        let mut found = false;
        for line in text.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("schedule_cron:") {
                let indent = &line[..line.len() - trimmed.len()];
                updated_lines.push(format!(
                    "{indent}schedule_cron: {}",
                    yaml_single_quoted(cron)
                ));
                found = true;
            } else {
                updated_lines.push(line.to_string());
            }
        }
        if !found {
            anyhow::bail!(
                "schedule_cron field not found in {}",
                self.config_file.display()
            );
        }
        let mut updated = updated_lines.join("\n");
        if text.ends_with('\n') {
            updated.push('\n');
        }
        fs::write(&self.config_file, updated)
            .with_context(|| format!("failed to write {}", self.config_file.display()))
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
        "# Identity\n\n## Repository\n- Name: {repo_name}\n- Purpose: Describe what this repository is for.\n\n## Allowed Work\n- Source code\n- Tests\n- Documentation\n- Repo-local runtime bootstrap and issue-intake workflow files\n\n## Guardrails\n- Treat `.bond/IDENTITY.md`, `.bond/PERSONALITY.md`, and `.bond/config.yml` as operator-owned files. Update them only through deliberate setup or direct human instruction.\n- Respect the configured issue intake rules. Do not bypass label requirements or prompt-contract expectations to create your own work queue.\n- Regenerate `.github/workflows/bond.yml` intentionally through the setup flow instead of drifting it with casual manual edits.\n- If work is blocked on human judgment, credentials, policy, or manual configuration, open a GitHub issue labeled `needs-human` with the exact request. Those issues are for humans only and are outside the bond's intake queue.\n"
    )
}

fn default_personality_contents(repo_root: &Path) -> String {
    let repo_name = repo_root
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("this repository");

    format!(
        "# Personality\n\nThe bond for {repo_name} should operate like a careful repository mechanic: clear about scope, disciplined about state, and willing to stop and escalate when automation should not guess.\n\n## Tone\n- Direct\n- Practical\n- Calm\n- Operator-aware\n\n## Collaboration\n- Prefer small, auditable changes\n- Explain blockers plainly\n- Respect the repository's existing style\n- Treat `.bond` history, issue state, and journal entries as part of the working record\n- When blocked by missing human input, open a `needs-human` issue with a specific request instead of improvising around the gap\n"
    )
}

fn default_journal_contents() -> String {
    "# Journal\n\n## Bond Initialized\n\nThe .bond runtime context was created for this repository.\n".to_string()
}

fn default_issue_template_config_contents() -> String {
    "blank_issues_enabled: false\ncontact_links: []\n".to_string()
}

fn default_automation_settings() -> AutomationSettings {
    AutomationSettings {
        schedule_cron: default_schedule_cron(),
        provider: default_automation_provider(),
        model: default_automation_model(),
        multiple_issues: default_automation_multiple_issues(),
        thinking_effort: default_automation_thinking_effort(),
    }
}

fn default_schedule_cron() -> String {
    "0 * * * *".to_string()
}

fn default_automation_provider() -> String {
    "anthropic".to_string()
}

fn default_automation_model() -> String {
    cli::default_model_for_provider(&default_automation_provider())
}

fn default_automation_multiple_issues() -> bool {
    false
}

fn default_automation_thinking_effort() -> ThinkingLevel {
    ThinkingLevel::Medium
}

fn default_bootstrap_bond_settings_contents(settings: &BondSettings) -> String {
    format!(
        "version: {}\nexecutable_path: {}\nautomation:\n  schedule_cron: {}\n  provider: {}\n  model: {}\n  multiple_issues: {}\n  thinking_effort: {}\ncommands:\n  # Add repo-specific test commands before using /test or scheduled verification.\n  test: []\n  # Add repo-specific lint commands before using /lint or scheduled verification.\n  lint: []\nissues:\n  eligible_labels:\n{}  priority_labels:\n{}  require_prompt_contract: {}\n  issue_history_limit: {}\n",
        settings.version,
        yaml_single_quoted(&settings.executable_path),
        yaml_single_quoted(&settings.automation.schedule_cron),
        yaml_single_quoted(&settings.automation.provider),
        yaml_single_quoted(&settings.automation.model),
        settings.automation.multiple_issues,
        yaml_single_quoted(thinking_level_str(settings.automation.thinking_effort)),
        render_yaml_string_list(&settings.issues.eligible_labels, 4),
        render_yaml_string_list(&settings.issues.priority_labels, 4),
        settings.issues.require_prompt_contract,
        settings.issues.issue_history_limit,
    )
}

fn default_workflow_commands() -> WorkflowCommands {
    WorkflowCommands {
        test: Vec::new(),
        lint: Vec::new(),
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

fn default_bond_workflow_contents(settings: &BondSettings) -> String {
    let provider = settings.automation.provider.trim();
    let thinking_effort_comment =
        workflow_thinking_effort_comment(settings.automation.thinking_effort);
    let api_key_env = cli::provider_api_key_env(provider)
        .map(|env| {
            format!(
                "          {env}: ${{{{ secrets.{env} }}}}
"
            )
        })
        .unwrap_or_default();
    let verification_commands = workflow_verification_commands(&settings.commands);
    let workflow_write_auth_steps = concat!(
        r#"      - name: Inspect scheduled publish state
    "#,
        r#"        id: scheduled_publish_state
    "#,
        r#"        shell: bash
    "#,
        r#"        run: |
    "#,
        r#"          set -euo pipefail
    "#,
        r#"
    "#,
        r#"          read_scheduled_target() {
    "#,
        r#"            if [[ ! -f .bond/state.yml ]]; then
    "#,
        r#"              return 0
    "#,
        r#"            fi
    "#,
        r#"
    "#,
        r#"            local target_block
    "#,
        r#"            target_block="$(sed -n '/^scheduled_target:$/,/^[^ ]/p' .bond/state.yml)"
    "#,
        r#"            if [[ -z "$target_block" ]]; then
    "#,
        r#"              return 0
    "#,
        r#"            fi
    "#,
        r#"
    "#,
        r#"            SCHEDULED_TARGET_KIND="$(printf '%s\n' "$target_block" | sed -n 's/^  kind: //p' | head -n 1)"
    "#,
        r#"          }
    "#,
        r#"
    "#,
        r#"          read_scheduled_target || true
    "#,
        r#"
    "#,
        r#"          if [[ "${SCHEDULED_TARGET_KIND:-}" == "merge_wait" ]]; then
    "#,
        r#"            echo "needs_publish=false" >> "$GITHUB_OUTPUT"
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          if [[ -z "$(git status --short)" ]]; then
    "#,
        r#"            echo "needs_publish=false" >> "$GITHUB_OUTPUT"
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          echo "needs_publish=true" >> "$GITHUB_OUTPUT"
    "#,
        r#"
    "#,
        r#"      - name: Validate bond write credentials
    "#,
        r#"        if: steps.scheduled_publish_state.outputs.needs_publish == 'true'
    "#,
        r#"        shell: bash
    "#,
        r#"        env:
    "#,
        r#"          BOND_GITHUB_APP_ID: ${{ secrets.BOND_GITHUB_APP_ID }}
    "#,
        r#"          BOND_GITHUB_APP_PRIVATE_KEY: ${{ secrets.BOND_GITHUB_APP_PRIVATE_KEY }}
    "#,
        r#"        run: |
    "#,
        r#"          set -euo pipefail
    "#,
        r#"
    "#,
        r#"          if [[ -z "${BOND_GITHUB_APP_ID:-}" ]]; then
    "#,
        r#"            echo "Missing required secret BOND_GITHUB_APP_ID for scheduled bond writes." >&2
    "#,
        r#"            exit 1
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          if [[ -z "${BOND_GITHUB_APP_PRIVATE_KEY:-}" ]]; then
    "#,
        r#"            echo "Missing required secret BOND_GITHUB_APP_PRIVATE_KEY for scheduled bond writes." >&2
    "#,
        r#"            exit 1
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"      - name: Mint bond write token
    "#,
        r#"        if: steps.scheduled_publish_state.outputs.needs_publish == 'true'
    "#,
        r#"        id: bond_write_token
    "#,
        r#"        uses: actions/create-github-app-token@v2
    "#,
        r#"        with:
    "#,
        r#"          app-id: ${{ secrets.BOND_GITHUB_APP_ID }}
    "#,
        r#"          private-key: ${{ secrets.BOND_GITHUB_APP_PRIVATE_KEY }}
    "#,
        r#"          owner: ${{ github.repository_owner }}
    "#,
        r#"
    "#,
        r#"      - name: Commit, push, and open PR
    "#,
        r#"        if: steps.scheduled_publish_state.outputs.needs_publish == 'true'
    "#,
        r#"        shell: bash
    "#,
        r#"        env:
    "#,
        r#"          GH_TOKEN: ${{ steps.bond_write_token.outputs.token }}
    "#,
        r#"          BOND_WRITE_TOKEN: ${{ steps.bond_write_token.outputs.token }}
    "#,
        r#"        run: |
    "#,
        r#"          set -euo pipefail
    "#,
        r#"
    "#,
        r#"          read_scheduled_target() {
    "#,
        r#"            if [[ ! -f .bond/state.yml ]]; then
    "#,
        r#"              return 0
    "#,
        r#"            fi
    "#,
        r#"
    "#,
        r#"            local target_block
    "#,
        r#"            target_block="$(sed -n '/^scheduled_target:$/,/^[^ ]/p' .bond/state.yml)"
    "#,
        r#"            if [[ -z "$target_block" ]]; then
    "#,
        r#"              return 0
    "#,
        r#"            fi
    "#,
        r#"
    "#,
        r#"            SCHEDULED_TARGET_KIND="$(printf '%s\n' "$target_block" | sed -n 's/^  kind: //p' | head -n 1)"
    "#,
        r#"            ISSUE_NUMBER="$(printf '%s\n' "$target_block" | sed -n 's/^  issue_number: //p' | head -n 1)"
    "#,
        r#"            ISSUE_TITLE="$(printf '%s\n' "$target_block" | sed -n 's/^  issue_title: //p' | head -n 1)"
    "#,
        r#"            ISSUE_BRANCH="$(printf '%s\n' "$target_block" | sed -n 's/^  branch_name: //p' | head -n 1)"
    "#,
        r#"
    "#,
        r#"            ISSUE_TITLE="${ISSUE_TITLE#\"}"
    "#,
        r#"            ISSUE_TITLE="${ISSUE_TITLE%\"}"
    "#,
        r#"            ISSUE_BRANCH="${ISSUE_BRANCH#\"}"
    "#,
        r#"            ISSUE_BRANCH="${ISSUE_BRANCH%\"}"
    "#,
        r#"          }
    "#,
        r#"
    "#,
        r#"          read_scheduled_target || true
    "#,
        r#"
    "#,
        r#"          if [[ "${SCHEDULED_TARGET_KIND:-}" == "merge_wait" ]]; then
    "#,
        r#"            echo "Scheduled run is waiting for merge or approval. Skipping commit."
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          if [[ -z "$(git status --short)" ]]; then
    "#,
        r#"            echo "No changes to commit."
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          if [[ -z "${ISSUE_NUMBER:-}" || -z "${ISSUE_BRANCH:-}" ]]; then
    "#,
        r#"            echo "Changed files but no persisted scheduled target metadata was found."
    "#,
        r#"            exit 1
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          if [[ "$(git branch --show-current)" != "$ISSUE_BRANCH" ]]; then
    "#,
        r#"            git checkout -b "$ISSUE_BRANCH"
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          git add -A
    "#,
        r#"
    "#,
        r#"          if git diff --cached --quiet; then
    "#,
        r#"            echo "No staged changes to commit."
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          git commit -m "bond: work on #$ISSUE_NUMBER"
    "#,
        r#"          git push "https://x-access-token:${BOND_WRITE_TOKEN}@github.com/${{ github.repository }}.git" "$ISSUE_BRANCH"
    "#,
        r#"
    "#,
        r#"          existing_pr="$(gh pr list --head "$ISSUE_BRANCH" --base "${{ github.ref_name }}" --json number --jq '.[0].number')"
    "#,
        r#"          if [[ -n "$existing_pr" && "$existing_pr" != "null" ]]; then
    "#,
        r#"            echo "PR #$existing_pr already exists for $ISSUE_BRANCH."
    "#,
        r#"            exit 0
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          pr_title="bond: resolve #$ISSUE_NUMBER"
    "#,
        r#"          if [[ -n "${ISSUE_TITLE:-}" ]]; then
    "#,
        r#"            pr_title="$pr_title $ISSUE_TITLE"
    "#,
        r#"          fi
    "#,
        r#"
    "#,
        r#"          pr_body=$(printf 'Closes #%s\n\nAutomated changes from doublenot-bond.\n' "$ISSUE_NUMBER")
    "#,
        r#"
    "#,
        r#"          gh pr create --base "${{ github.ref_name }}" --head "$ISSUE_BRANCH" --title "$pr_title" --body "$pr_body"
    "#,
        )
        .replace("\n    ", "\n");

    format!(
        r##"# Generated by doublenot-bond from .bond/config.yml.
# Refresh with: doublenot-bond --prompt "/setup workflow refresh"
{thinking_effort_comment}name: bond

on:
  workflow_dispatch:
  schedule:
    - cron: '{cron}'

env:
  FORCE_JAVASCRIPT_ACTIONS_TO_NODE24: true

permissions:
  contents: write
  issues: write
  pull-requests: write

concurrency:
  group: bond-${{{{ github.ref }}}}
  cancel-in-progress: false

jobs:
  bond:
    runs-on: ubuntu-latest
    timeout-minutes: 30

    steps:
      - uses: actions/checkout@v5
        with:
          fetch-depth: 0
          ref: ${{{{ github.ref_name }}}}

      - name: Install build dependencies
        run: sudo apt-get update && sudo apt-get install -y pkg-config libssl-dev

      - name: Install Rust toolchain
        uses: dtolnay/rust-toolchain@stable

      - name: Build repo-local bond runtime
        shell: bash
        run: |
          set -euo pipefail
          cargo build --locked --bin doublenot-bond
          mkdir -p .bond/bin
          cp target/debug/doublenot-bond .bond/bin/doublenot-bond
          chmod +x .bond/bin/doublenot-bond

      - name: Configure git identity
        run: |
          git config user.name "doublenot-bond[bot]"
          git config user.email "doublenot-bond[bot]@users.noreply.github.com"

      - name: Run scheduled bond issue workflow
        env:
          GH_TOKEN: ${{{{ secrets.GITHUB_TOKEN }}}}
{api_key_env}        run: ./.bond/bin/doublenot-bond --repo . --run-scheduled-issue

      - name: Verify repo changes before commit
        shell: bash
        run: |
          set -euo pipefail

          read_scheduled_target() {{
            if [[ ! -f .bond/state.yml ]]; then
              return 0
            fi

            local target_block
            target_block="$(sed -n '/^scheduled_target:$/,/^[^ ]/p' .bond/state.yml)"
            if [[ -z "$target_block" ]]; then
              return 0
            fi

            SCHEDULED_TARGET_KIND="$(printf '%s\n' "$target_block" | sed -n 's/^  kind: //p' | head -n 1)"
          }}

          read_scheduled_target || true

          if [[ "${{SCHEDULED_TARGET_KIND:-}}" == "merge_wait" ]]; then
            echo "Scheduled run is waiting for merge or approval. Skipping verification."
            exit 0
          fi

          if [[ -z "$(git status --short)" ]]; then
            echo "No changes to verify."
            exit 0
          fi

{verification_commands}
{workflow_write_auth_steps}
"##,
        cron = settings.automation.schedule_cron,
        thinking_effort_comment = thinking_effort_comment,
        api_key_env = api_key_env,
        verification_commands = verification_commands,
        workflow_write_auth_steps = workflow_write_auth_steps,
    )
}

fn workflow_verification_commands(commands: &WorkflowCommands) -> String {
    let mut lines = Vec::new();

    lines.extend(render_workflow_command_group("lint", &commands.lint));
    lines.extend(render_workflow_command_group("test", &commands.test));

    if lines.is_empty() {
        return "          echo \"Verification required but no commands configured. Add commands.test or commands.lint to .bond/config.yml.\"\n          exit 1\n".to_string();
    }

    lines.join("\n") + "\n"
}

fn render_workflow_command_group(group_name: &str, commands: &[RepoCommand]) -> Vec<String> {
    if commands.is_empty() {
        return Vec::new();
    }

    let mut lines = vec![format!(
        "          echo \"Running configured {group_name} commands.\""
    )];
    for command in commands {
        let rendered_command = std::iter::once(command.program.as_str())
            .chain(command.args.iter().map(String::as_str))
            .map(shell_quote)
            .collect::<Vec<_>>()
            .join(" ");
        lines.push(format!(
            "          printf '> %s\\n' {}",
            shell_quote(&command.description)
        ));
        lines.push(format!("          {rendered_command}"));
    }

    lines
}

fn shell_quote(value: &str) -> String {
    let escaped = value.replace('\'', "'\"'\"'");
    format!("'{escaped}'")
}

fn deserialize_null_default<'de, D, T>(deserializer: D) -> std::result::Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Ok(Option::<T>::deserialize(deserializer)?.unwrap_or_default())
}

fn yaml_single_quoted(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn render_yaml_string_list(values: &[String], indent: usize) -> String {
    let indentation = " ".repeat(indent);
    values
        .iter()
        .map(|value| format!("{indentation}- {}\n", yaml_single_quoted(value)))
        .collect()
}

fn thinking_level_str(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
    }
}

fn workflow_thinking_effort_comment(level: ThinkingLevel) -> String {
    format!("# Thinking effort: {}\n", thinking_level_str(level))
}

pub fn issue_branch_name(issue_number: u64, issue_title: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for ch in issue_title.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch);
            last_was_dash = false;
        } else if !last_was_dash && !slug.is_empty() {
            slug.push('-');
            last_was_dash = true;
        }

        if slug.len() >= 48 {
            break;
        }
    }

    while slug.ends_with('-') {
        slug.pop();
    }

    if slug.is_empty() {
        slug.push_str("issue");
    }

    format!("bond/issue-{issue_number}-{slug}")
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
    "---\nname: Bond Task\nabout: Give the bond a task using the prompt-contract structure\ntitle: \"Task: \"\nlabels: bond-task\nassignees: ''\n---\n\n## Inputs\n\nDescribe the codebase context, relevant files, and what the bond should look at first.\n\n## Expected Output\n\nDescribe the desired code, docs, or behavior change.\n\n## Constraints\n\nList any architectural rules, scope boundaries, or forbidden approaches.\n\nIf this task depends on earlier issue work that is not resolved yet, add a line like `Depends on: #123`.\n\n## Edge Cases\n\nList failure modes, tricky cases, or compatibility concerns.\n\n## Acceptance Criteria\n\nList the concrete checks that determine when this task is complete.\n".to_string()
}

fn default_debug_issue_template_contents() -> String {
    "---\nname: Bond Debug\nabout: Ask the bond to diagnose and fix a bug using a debugging contract\ntitle: \"Debug: \"\nlabels: bond-task, bug\nassignees: ''\n---\n\n## Inputs\n\nDescribe the bug, failing behavior, reproduction steps, and relevant logs.\n\n## Expected Output\n\nDescribe the fix, explanation, and any tests or instrumentation you expect.\n\n## Constraints\n\nList limits on risky changes, migrations, or files the bond should avoid.\n\nIf this debug work depends on earlier issue work that is not resolved yet, add a line like `Depends on: #123`.\n\n## Edge Cases\n\nCall out intermittent failures, environment differences, or known false leads.\n\n## Acceptance Criteria\n\nList the exact reproduction that should stop failing and the checks that should pass afterward.\n".to_string()
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn set_schedule_cron_updates_config_field() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path();

        let paths = BondPaths::new(repo.to_path_buf()).expect("BondPaths::new");
        paths.bootstrap_bond_files().expect("bootstrap");

        paths
            .set_schedule_cron("0 */6 * * *")
            .expect("set_schedule_cron");

        let config_text = fs::read_to_string(&paths.config_file).expect("read config");
        assert!(
            config_text.contains("schedule_cron: '0 */6 * * *'"),
            "config should have updated cron: {config_text}"
        );
    }

    #[test]
    fn set_schedule_cron_preserves_other_config_fields() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path();

        let paths = BondPaths::new(repo.to_path_buf()).expect("BondPaths::new");
        paths.bootstrap_bond_files().expect("bootstrap");

        paths
            .set_schedule_cron("30 4 * * 1")
            .expect("set_schedule_cron");

        let settings = paths.load_bond_settings().expect("load settings");
        assert_eq!(settings.automation.schedule_cron, "30 4 * * 1");
        assert!(!settings.automation.provider.is_empty());
        assert!(!settings.automation.model.is_empty());
    }

    #[test]
    fn set_schedule_cron_handles_quoted_existing_value() {
        let temp = tempdir().expect("tempdir");
        let repo = temp.path();
        let config_path = repo.join(".bond/config.yml");
        fs::create_dir_all(repo.join(".bond")).expect("create .bond");
        fs::write(
            &config_path,
            "version: 1\nexecutable_path: .bond/bin/doublenot-bond\nautomation:\n  schedule_cron: '0 * * * *'\n  provider: anthropic\n  model: claude-sonnet\n",
        )
        .expect("write config");

        let paths = BondPaths::new(repo.to_path_buf()).expect("BondPaths::new");
        paths
            .set_schedule_cron("0 */4 * * *")
            .expect("set_schedule_cron");

        let config_text = fs::read_to_string(&config_path).expect("read config");
        assert!(
            config_text.contains("schedule_cron: '0 */4 * * *'"),
            "config should have updated cron: {config_text}"
        );
        assert!(config_text.contains("provider: anthropic"));
        assert!(config_text.contains("model: claude-sonnet"));
    }
}
