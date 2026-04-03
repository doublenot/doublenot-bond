use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const SYSTEM_PROMPT_BASE: &str = r#"You are doublenot-bond, a coding agent running inside a repository.
Use the repository's .bond files as the source of truth for identity, tone, and working context.
Be direct, safe, and practical. When asked to do work, inspect the code, make focused changes, and verify them when possible."#;

#[derive(Debug, Clone, Default)]
pub struct PermissionConfig {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

impl PermissionConfig {
    pub fn check(&self, subject: &str) -> Option<bool> {
        for pattern in &self.deny {
            if glob_match(pattern, subject) {
                return Some(false);
            }
        }

        for pattern in &self.allow {
            if glob_match(pattern, subject) {
                return Some(true);
            }
        }

        None
    }

    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }
}

#[derive(Debug, Clone, Default)]
pub struct DirectoryRestrictions {
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

impl DirectoryRestrictions {
    pub fn is_empty(&self) -> bool {
        self.allow.is_empty() && self.deny.is_empty()
    }

    pub fn check_path(&self, path: &str) -> Result<()> {
        if self.is_empty() {
            return Ok(());
        }

        let resolved = resolve_path(path)?;

        for denied in &self.deny {
            let denied_resolved = resolve_path(denied)?;
            if path_is_under(&resolved, &denied_resolved) {
                bail!(
                    "Access denied: '{}' is under restricted directory '{}'",
                    path,
                    denied
                );
            }
        }

        if !self.allow.is_empty() {
            let allowed = self
                .allow
                .iter()
                .map(|dir| resolve_path(dir))
                .collect::<Result<Vec<_>>>()?
                .into_iter()
                .any(|dir| path_is_under(&resolved, &dir));

            if !allowed {
                bail!(
                    "Access denied: '{}' is not under any allowed directory",
                    path
                );
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Args {
    pub repo: Option<PathBuf>,
    pub prompt: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub api_key: Option<String>,
    pub permissions: PermissionConfig,
    pub dir_restrictions: DirectoryRestrictions,
    pub no_color: bool,
    pub help: bool,
    pub version: bool,
    pub bootstrap_only: bool,
    pub bond_runtime: bool,
    pub run_scheduled_issue: bool,
}

pub fn parse_args(args: Vec<String>) -> Result<Args> {
    let mut parsed = Args::default();
    let mut iter = args.into_iter();

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--help" | "-h" => parsed.help = true,
            "--version" | "-V" => parsed.version = true,
            "--no-color" => parsed.no_color = true,
            "--bootstrap-only" => parsed.bootstrap_only = true,
            "--bond-runtime" => parsed.bond_runtime = true,
            "--run-scheduled-issue" => parsed.run_scheduled_issue = true,
            "--repo" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--repo requires a path"))?;
                parsed.repo = Some(PathBuf::from(value));
            }
            "--prompt" | "-p" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--prompt requires a value"))?;
                parsed.prompt = Some(value);
            }
            "--provider" => {
                parsed.provider = Some(
                    iter.next()
                        .ok_or_else(|| anyhow!("--provider requires a value"))?,
                );
            }
            "--model" => {
                parsed.model = Some(
                    iter.next()
                        .ok_or_else(|| anyhow!("--model requires a value"))?,
                );
            }
            "--api-key" => {
                parsed.api_key = Some(
                    iter.next()
                        .ok_or_else(|| anyhow!("--api-key requires a value"))?,
                );
            }
            "--allow" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--allow requires a pattern"))?;
                parsed.permissions.allow.push(value);
            }
            "--deny" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--deny requires a pattern"))?;
                parsed.permissions.deny.push(value);
            }
            "--allow-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--allow-dir requires a path"))?;
                parsed.dir_restrictions.allow.push(value);
            }
            "--deny-dir" => {
                let value = iter
                    .next()
                    .ok_or_else(|| anyhow!("--deny-dir requires a path"))?;
                parsed.dir_restrictions.deny.push(value);
            }
            other if other.starts_with('-') => {
                bail!("Unknown flag: {other}. Run --help for usage.");
            }
            other => {
                if parsed.prompt.is_none() {
                    parsed.prompt = Some(other.to_string());
                } else {
                    bail!("Unexpected argument: {other}");
                }
            }
        }
    }

    Ok(parsed)
}

pub fn resolve_repo_root(repo: Option<&Path>) -> Result<PathBuf> {
    let root = if let Some(repo) = repo {
        repo.to_path_buf()
    } else {
        std::env::current_dir().context("failed to resolve current directory")?
    };

    if !root.exists() {
        bail!("Repository path does not exist: {}", root.display());
    }

    root.canonicalize()
        .with_context(|| format!("failed to canonicalize {}", root.display()))
}

pub fn default_model_for_provider(provider: &str) -> String {
    match provider {
        "anthropic" => "claude-sonnet-4-20250514",
        "google" => "gemini-2.5-pro",
        "ollama" => "qwen2.5-coder",
        _ => "gpt-4.1",
    }
    .to_string()
}

pub fn provider_api_key_env(provider: &str) -> Option<&'static str> {
    match provider {
        "anthropic" => Some("ANTHROPIC_API_KEY"),
        "openai" => Some("OPENAI_API_KEY"),
        "google" => Some("GOOGLE_API_KEY"),
        "openrouter" => Some("OPENROUTER_API_KEY"),
        "groq" => Some("GROQ_API_KEY"),
        "deepseek" => Some("DEEPSEEK_API_KEY"),
        "ollama" => None,
        _ => Some("API_KEY"),
    }
}

pub fn print_help() {
    println!("doublenot-bond v{VERSION}");
    println!();
    println!("Usage:");
    println!("  doublenot-bond [OPTIONS]");
    println!("  doublenot-bond --prompt \"inspect this repo\"");
    println!();
    println!("Options:");
    println!("  --repo <path>          Use a specific repository root");
    println!("  --prompt, -p <text>    Run a one-shot prompt");
    println!("  --provider <name>      Set the AI provider (defaults to .bond/config.yml)");
    println!("  --model <name>         Override the model (defaults to .bond/config.yml)");
    println!("  --api-key <key>        Provide an API key directly");
    println!("  --allow <pattern>      Allow matching shell/file operations");
    println!("  --deny <pattern>       Deny matching shell/file operations");
    println!("  --allow-dir <path>     Allow file access only under this directory");
    println!("  --deny-dir <path>      Deny file access under this directory");
    println!("  --bootstrap-only       Create .bond/ and exit");
    println!("  --run-scheduled-issue  Select or resume an issue-driven run for automation");
    println!("  --no-color             Disable ANSI colors");
    println!("  --help, -h             Show this help text");
    println!("  --version, -V          Show the version");
    println!();
    println!("Slash commands:");
    println!("  /status                Show runtime and model status");
    println!("  /setup <subcommand>    Manage .bond setup state and onboarding issue");
    println!("    /setup workflow      Create .github/workflows/bond.yml from .bond/config.yml");
    println!("  /issues list           List eligible GitHub intake issues");
    println!("  /issues next           Select the highest-priority eligible issue");
    println!(
        "  /issues resume         Resume current work, fall back to previous, then next issue"
    );
    println!("  /issues select <n>     Select a specific GitHub issue by number");
    println!("  /issues reopen <n>     Reopen a GitHub issue, optionally comment, and restore it locally");
    println!("  /issues reopen-current Reopen the last recorded issue, optionally comment, and restore it locally");
    println!("  /issues previous       Restore the most recent prior issue from local history");
    println!(
        "  /issues history [flt]  Show recent issue history, filtered by action, state, or label"
    );
    println!("  /issues park [msg]     Clear the current issue while preserving parked state");
    println!("  /issues sync           Refresh or clear the current issue from GitHub");
    println!("  /issues current        Show the persisted current issue");
    println!("  /issues prompt         Render the agent execution prompt for the issue");
    println!("  /issues start          Execute the current issue prompt through the agent");
    println!("  /issues comment <msg>  Post a comment to the current GitHub issue");
    println!("  /issues complete [msg] Comment optionally, close the issue, and clear selection");
    println!("  /issues clear          Clear the persisted current issue selection");
    println!("  /tree [path] [depth]   Print a repo tree view");
    println!("  /git [status|diff|log] Run git inspection commands");
    println!("  /test                  Run the repo test command");
    println!("  /lint                  Run the repo lint command");
}

pub fn print_version() {
    println!("doublenot-bond v{VERSION}");
}

fn resolve_path(path: &str) -> Result<String> {
    let path = Path::new(path);
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .context("failed to read current directory")?
            .join(path)
    };

    if let Ok(canonical) = absolute.canonicalize() {
        return Ok(canonical.to_string_lossy().to_string());
    }

    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            std::path::Component::CurDir => {}
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized.to_string_lossy().to_string())
}

fn path_is_under(path: &str, dir: &str) -> bool {
    let dir_with_sep = if dir.ends_with('/') {
        dir.to_string()
    } else {
        format!("{dir}/")
    };
    path == dir || path.starts_with(&dir_with_sep)
}

fn glob_match(pattern: &str, text: &str) -> bool {
    let parts: Vec<&str> = pattern.split('*').collect();

    if parts.len() == 1 {
        return pattern == text;
    }

    let starts_with_wildcard = pattern.starts_with('*');
    let ends_with_wildcard = pattern.ends_with('*');
    let mut position = 0usize;

    for (index, part) in parts.iter().enumerate() {
        if part.is_empty() {
            continue;
        }

        if index == 0 && !starts_with_wildcard {
            if !text[position..].starts_with(part) {
                return false;
            }
            position += part.len();
            continue;
        }

        if index == parts.len() - 1 && !ends_with_wildcard {
            return text[position..].ends_with(part);
        }

        match text[position..].find(part) {
            Some(found) => position += found + part.len(),
            None => return false,
        }
    }

    true
}

#[cfg(test)]
mod tests {
    use super::{glob_match, parse_args};

    #[test]
    fn parse_args_collects_permission_and_directory_flags() {
        let args = parse_args(vec![
            "--allow".to_string(),
            "git*".to_string(),
            "--deny".to_string(),
            "rm *".to_string(),
            "--allow-dir".to_string(),
            "src".to_string(),
            "--deny-dir".to_string(),
            ".git".to_string(),
        ])
        .expect("parse args");

        assert_eq!(args.permissions.allow, vec!["git*"]);
        assert_eq!(args.permissions.deny, vec!["rm *"]);
        assert_eq!(args.dir_restrictions.allow, vec!["src"]);
        assert_eq!(args.dir_restrictions.deny, vec![".git"]);
    }

    #[test]
    fn glob_match_supports_simple_wildcards() {
        assert!(glob_match("git*", "git status"));
        assert!(glob_match("*status", "git status"));
        assert!(glob_match("*cargo*test*", "cargo test --lib"));
        assert!(!glob_match("cargo*fmt", "cargo test"));
    }
}
