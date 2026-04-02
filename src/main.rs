mod agent;
mod bond;
mod cli;
mod commands;
mod prompt;
mod repl;

use anyhow::{Context, Result};
use cli::parse_args;
use commands::{dispatch_command, ReplDirective};
use std::env;
use std::io::{self, IsTerminal, Read};
use std::process::{Command, Stdio};

#[tokio::main]
async fn main() {
    if let Err(error) = run().await {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let args = parse_args(raw_args.clone())?;

    if args.help {
        cli::print_help();
        return Ok(());
    }

    if args.version {
        cli::print_version();
        return Ok(());
    }

    let repo_root = cli::resolve_repo_root(args.repo.as_deref())?;
    let bond_paths = bond::BondPaths::new(repo_root)?;
    let created = bond_paths.bootstrap_bond_files()?;
    let bond_config = bond_paths.load_bond_config()?;
    let current_executable = env::current_exe().context("failed to resolve current executable")?;
    let executable_installed =
        bond_paths.ensure_runtime_executable(&bond_config, &current_executable)?;

    if created {
        eprintln!("Initialized {}", bond_paths.bond_dir.display());
    }
    if executable_installed {
        let target = bond_paths.executable_target_path(&bond_config);
        let display_path = target
            .strip_prefix(&bond_paths.repo_root)
            .unwrap_or(target.as_path());
        eprintln!("Installed runtime executable to {}", display_path.display());
    }

    if args.bootstrap_only {
        println!("Bootstrap complete: {}", bond_paths.bond_dir.display());
        return Ok(());
    }

    let runtime_executable = bond_paths.executable_target_path(&bond_config);
    if !args.bond_runtime && should_respawn_into_runtime(&current_executable, &runtime_executable)?
    {
        return respawn_into_runtime(&bond_paths, &runtime_executable, &raw_args);
    }

    let mut runtime = bond_paths.load_runtime_context()?;
    if !runtime.config.configured {
        eprintln!(
            "Bond setup is not complete yet. Review .bond files and use /setup complete when ready."
        );
    }

    let agent_config = agent::BondAgentConfig::from_args(&args, &runtime)?;

    if let Some(prompt_text) = args.prompt.clone() {
        if prompt_text.trim_start().starts_with('/') {
            match dispatch_command(prompt_text.trim(), &mut runtime, &agent_config)? {
                ReplDirective::Continue | ReplDirective::Exit => {}
                ReplDirective::Prompt(issue_prompt) => {
                    let mut agent = agent_config.build_agent()?;
                    prompt::run_prompt(&mut agent, &issue_prompt).await?;
                }
            }
            return Ok(());
        }

        let mut agent = agent_config.build_agent()?;
        prompt::run_prompt(&mut agent, &prompt_text).await?;
        return Ok(());
    }

    if !io::stdin().is_terminal() {
        let mut buffer = String::new();
        io::stdin()
            .read_to_string(&mut buffer)
            .context("failed to read stdin")?;
        if buffer.trim().is_empty() {
            anyhow::bail!("No input on stdin.");
        }

        let trimmed = buffer.trim();
        if !trimmed.contains('\n') && trimmed.starts_with('/') {
            match dispatch_command(trimmed, &mut runtime, &agent_config)? {
                ReplDirective::Continue | ReplDirective::Exit => {}
                ReplDirective::Prompt(issue_prompt) => {
                    let mut agent = agent_config.build_agent()?;
                    prompt::run_prompt(&mut agent, &issue_prompt).await?;
                }
            }
            return Ok(());
        }

        let mut agent = agent_config.build_agent()?;
        prompt::run_prompt(&mut agent, &buffer).await?;
        return Ok(());
    }

    repl::run_repl(&mut runtime, &agent_config).await
}

fn should_respawn_into_runtime(
    current_executable: &std::path::Path,
    runtime_executable: &std::path::Path,
) -> Result<bool> {
    let current = current_executable
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", current_executable.display()))?;
    let runtime = runtime_executable
        .canonicalize()
        .with_context(|| format!("failed to canonicalize {}", runtime_executable.display()))?;
    Ok(current != runtime)
}

fn respawn_into_runtime(
    bond_paths: &bond::BondPaths,
    runtime_executable: &std::path::Path,
    raw_args: &[String],
) -> Result<()> {
    eprintln!(
        "Switching to repo-local runtime executable: {}",
        runtime_executable.display()
    );

    let status = Command::new(runtime_executable)
        .args(raw_args)
        .arg("--bond-runtime")
        .current_dir(&bond_paths.repo_root)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to launch {}", runtime_executable.display()))?;

    std::process::exit(status.code().unwrap_or(1));
}
