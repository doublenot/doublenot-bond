use crate::bond::BondRuntimeContext;
use crate::cli::{self, Args, DirectoryRestrictions, PermissionConfig};
use anyhow::{bail, Result};
use std::sync::Arc;
use yoagent::agent::Agent;
use yoagent::provider::{AnthropicProvider, GoogleProvider, ModelConfig, OpenAiCompatProvider};
use yoagent::tools::bash::BashTool;
use yoagent::tools::edit::EditFileTool;
use yoagent::tools::file::{ReadFileTool, WriteFileTool};
use yoagent::tools::list::ListFilesTool;
use yoagent::tools::search::SearchTool;
use yoagent::types::{AgentTool, ToolContext, ToolError, ToolResult};

pub struct BondAgentConfig {
    pub repo_root: std::path::PathBuf,
    pub model: String,
    pub provider: String,
    pub api_key: String,
    pub system_prompt: String,
    pub permissions: PermissionConfig,
    pub dir_restrictions: DirectoryRestrictions,
}

impl BondAgentConfig {
    pub fn from_args(args: &Args, runtime: &BondRuntimeContext) -> Result<Self> {
        let provider = args.provider.clone();
        let model = args
            .model
            .clone()
            .unwrap_or_else(|| cli::default_model_for_provider(&provider));

        let api_key = args
            .api_key
            .clone()
            .or_else(|| {
                cli::provider_api_key_env(&provider).and_then(|env| std::env::var(env).ok())
            })
            .unwrap_or_default();

        Ok(Self {
            repo_root: runtime.paths.repo_root.clone(),
            model,
            provider,
            api_key,
            system_prompt: build_system_prompt(runtime),
            permissions: args.permissions.clone(),
            dir_restrictions: absolutize_restrictions(
                &args.dir_restrictions,
                &runtime.paths.repo_root,
            ),
        })
    }

    pub fn build_agent(&self) -> Result<Agent> {
        self.validate_api_key()?;

        let model_config = create_model_config(&self.provider, &self.model);
        let agent = match self.provider.as_str() {
            "anthropic" => Agent::new(AnthropicProvider).with_model_config(model_config),
            "google" => Agent::new(GoogleProvider).with_model_config(model_config),
            _ => Agent::new(OpenAiCompatProvider).with_model_config(model_config),
        };

        Ok(agent
            .with_system_prompt(&self.system_prompt)
            .with_model(&self.model)
            .with_api_key(&self.api_key)
            .with_tools(build_tools(self)))
    }

    fn validate_api_key(&self) -> Result<()> {
        if self.provider != "ollama" && self.api_key.is_empty() {
            let env_hint = cli::provider_api_key_env(&self.provider).unwrap_or("API_KEY");
            bail!(
                "Missing API key for provider '{}'. Set {} or pass --api-key.",
                self.provider,
                env_hint
            );
        }

        Ok(())
    }
}

fn build_system_prompt(runtime: &BondRuntimeContext) -> String {
    format!(
        "{}\n\n# Bond Identity\n\n{}\n\n# Bond Personality\n\n{}\n\n# Bond Journal\n\n{}",
        cli::SYSTEM_PROMPT_BASE,
        runtime.identity.trim(),
        runtime.personality.trim(),
        runtime.journal.trim()
    )
}

fn build_tools(config: &BondAgentConfig) -> Vec<Box<dyn AgentTool>> {
    let bash = maybe_permission_gate(
        Box::new(BashTool::new().with_cwd(config.repo_root.display().to_string())),
        config.permissions.clone(),
        gate_bash,
    );
    let read_file = maybe_guard(
        Box::new(ReadFileTool::default()),
        config.dir_restrictions.clone(),
    );
    let write_file = maybe_guard(
        maybe_permission_gate(
            Box::new(WriteFileTool::new()),
            config.permissions.clone(),
            gate_path,
        ),
        config.dir_restrictions.clone(),
    );
    let edit_file = maybe_guard(
        maybe_permission_gate(
            Box::new(EditFileTool::new()),
            config.permissions.clone(),
            gate_path,
        ),
        config.dir_restrictions.clone(),
    );
    let list_files = maybe_guard(
        Box::new(ListFilesTool::default()),
        config.dir_restrictions.clone(),
    );
    let search = maybe_guard(
        Box::new(SearchTool::new().with_root(config.repo_root.display().to_string())),
        config.dir_restrictions.clone(),
    );

    vec![bash, read_file, write_file, edit_file, list_files, search]
}

fn absolutize_restrictions(
    restrictions: &DirectoryRestrictions,
    repo_root: &std::path::Path,
) -> DirectoryRestrictions {
    DirectoryRestrictions {
        allow: restrictions
            .allow
            .iter()
            .map(|path| absolutize_path(path, repo_root))
            .collect(),
        deny: restrictions
            .deny
            .iter()
            .map(|path| absolutize_path(path, repo_root))
            .collect(),
    }
}

fn absolutize_path(path: &str, repo_root: &std::path::Path) -> String {
    let candidate = std::path::Path::new(path);
    let absolute = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        repo_root.join(candidate)
    };

    absolute.to_string_lossy().to_string()
}

struct GuardedTool {
    inner: Box<dyn AgentTool>,
    restrictions: DirectoryRestrictions,
}

#[async_trait::async_trait]
impl AgentTool for GuardedTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        for key in ["path", "directory"] {
            if let Some(path) = params.get(key).and_then(|value| value.as_str()) {
                self.restrictions
                    .check_path(path)
                    .map_err(|error| ToolError::Failed(error.to_string()))?;
            }
        }

        self.inner.execute(params, ctx).await
    }
}

fn maybe_guard(
    tool: Box<dyn AgentTool>,
    restrictions: DirectoryRestrictions,
) -> Box<dyn AgentTool> {
    if restrictions.is_empty() {
        tool
    } else {
        Box::new(GuardedTool {
            inner: tool,
            restrictions,
        })
    }
}

type PermissionGate =
    dyn Fn(&PermissionConfig, &serde_json::Value) -> Result<(), ToolError> + Send + Sync;

struct PermissionTool {
    inner: Box<dyn AgentTool>,
    permissions: PermissionConfig,
    gate: Arc<PermissionGate>,
}

#[async_trait::async_trait]
impl AgentTool for PermissionTool {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn label(&self) -> &str {
        self.inner.label()
    }

    fn description(&self) -> &str {
        self.inner.description()
    }

    fn parameters_schema(&self) -> serde_json::Value {
        self.inner.parameters_schema()
    }

    async fn execute(
        &self,
        params: serde_json::Value,
        ctx: ToolContext,
    ) -> Result<ToolResult, ToolError> {
        (self.gate)(&self.permissions, &params)?;
        self.inner.execute(params, ctx).await
    }
}

fn maybe_permission_gate(
    tool: Box<dyn AgentTool>,
    permissions: PermissionConfig,
    gate: impl Fn(&PermissionConfig, &serde_json::Value) -> Result<(), ToolError>
        + Send
        + Sync
        + 'static,
) -> Box<dyn AgentTool> {
    if permissions.is_empty() {
        tool
    } else {
        Box::new(PermissionTool {
            inner: tool,
            permissions,
            gate: Arc::new(gate),
        })
    }
}

fn gate_bash(permissions: &PermissionConfig, params: &serde_json::Value) -> Result<(), ToolError> {
    let command = params
        .get("command")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ToolError::InvalidArgs("missing 'command' parameter".into()))?;

    match permissions.check(command) {
        Some(false) => Err(ToolError::Failed(format!(
            "Command denied by permission rule: {command}"
        ))),
        _ => Ok(()),
    }
}

fn gate_path(permissions: &PermissionConfig, params: &serde_json::Value) -> Result<(), ToolError> {
    let path = params
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| ToolError::InvalidArgs("missing 'path' parameter".into()))?;

    match permissions.check(path) {
        Some(false) => Err(ToolError::Failed(format!(
            "File operation denied by permission rule: {path}"
        ))),
        _ => Ok(()),
    }
}

fn create_model_config(provider: &str, model: &str) -> ModelConfig {
    match provider {
        "anthropic" => ModelConfig::anthropic(model, model),
        "google" => ModelConfig::google(model, model),
        "ollama" => ModelConfig::local("http://localhost:11434/v1", model),
        "openai" => ModelConfig::openai(model, model),
        _ => ModelConfig::local("http://localhost:8080/v1", model),
    }
}
