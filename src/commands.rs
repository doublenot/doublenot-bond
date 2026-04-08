use crate::agent::BondAgentConfig;
use crate::bond::{
    issue_branch_name, BondRuntimeContext, CurrentIssue, IssueWorkflow, RepoCommand,
    ScheduledTarget, ScheduledTargetKind, SetupIssue,
};
use crate::cli;
use crate::prompt;
use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use std::env;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use yoagent::types::ThinkingLevel;

pub enum ReplDirective {
    Continue,
    Exit,
    Prompt(String),
}

pub enum ScheduledExecution {
    Prompt(String),
    Wait(String),
    None,
}

enum ScheduledPullRequestDisposition<'a> {
    Actionable(&'a GitHubPullRequest),
    MergeWait(&'a GitHubPullRequest),
    None,
}

pub fn dispatch_command(
    input: &str,
    runtime: &mut BondRuntimeContext,
    config: &BondAgentConfig,
) -> Result<ReplDirective> {
    let mut parts = input.split_whitespace();
    let command = parts.next().unwrap_or(input);

    match command {
        "/help" => {
            cli::print_help();
            Ok(ReplDirective::Continue)
        }
        "/status" => {
            println!("repo: {}", runtime.paths.repo_root.display());
            println!("provider: {}", config.provider);
            println!("provider_source: {}", config.provider_source);
            println!("model: {}", config.model);
            println!("model_source: {}", config.model_source);
            println!("configured: {}", runtime.config.configured);
            println!("autonomous_enabled: {}", runtime.config.autonomous_enabled);
            println!("runtime_executable: {}", runtime.config.executable_path);
            println!(
                "automation_schedule_cron: {}",
                runtime.config.automation.schedule_cron
            );
            println!(
                "automation_provider: {}",
                runtime.config.automation.provider
            );
            println!("automation_model: {}", runtime.config.automation.model);
            println!(
                "automation_thinking_effort: {}",
                display_thinking_effort(runtime.config.automation.thinking_effort)
            );
            println!(
                "automation_multiple_issues: {}",
                runtime.config.automation.multiple_issues
            );
            println!(
                "workflow_file: {}",
                runtime.paths.bond_workflow_file.display()
            );
            println!(
                "workflow_installed: {}",
                runtime.paths.bond_workflow_file.exists()
            );
            print_automation_status_validation(runtime, config);
            if let Some(issue) = &runtime.config.setup_issue {
                println!(
                    "setup_issue: number={:?}, state={:?}, url={:?}",
                    issue.number, issue.state, issue.url
                );
            } else {
                println!("setup_issue: none");
            }
            if let Some(issue) = &runtime.config.current_issue {
                println!(
                    "current_issue: #{} [{}] {}",
                    issue.number, issue.label, issue.title
                );
                print_issue_metadata(issue);
            } else {
                println!("current_issue: none");
            }
            if let Some(issue) = &runtime.config.last_issue {
                println!(
                    "last_issue: #{} [{}] {}",
                    issue.number, issue.label, issue.title
                );
                print_issue_metadata(issue);
            } else {
                println!("last_issue: none");
            }
            println!(
                "issue_history: {}/{}",
                runtime.config.issue_history.len(),
                runtime.config.issues.issue_history_limit
            );
            print_issue_posture(runtime);
            println!(
                "permissions: allow={}, deny={}",
                config.permissions.allow.len(),
                config.permissions.deny.len()
            );
            println!(
                "directory_restrictions: allow={}, deny={}",
                config.dir_restrictions.allow.len(),
                config.dir_restrictions.deny.len()
            );
            Ok(ReplDirective::Continue)
        }
        "/setup" => handle_setup(parts.collect(), runtime, config),
        "/quit" | "/exit" => Ok(ReplDirective::Exit),
        "/git" => handle_git(parts.collect(), runtime, config),
        "/issues" => handle_issues(parts.collect(), runtime),
        "/test" => handle_test(runtime, config),
        "/lint" => handle_lint(runtime, config),
        "/tree" => handle_tree(parts.collect(), runtime, config),
        _ => {
            println!("Unknown command: {command}");
            Ok(ReplDirective::Continue)
        }
    }
}

fn print_automation_status_validation(runtime: &BondRuntimeContext, config: &BondAgentConfig) {
    let configured_provider = runtime.config.automation.provider.trim();
    let configured_model = runtime.config.automation.model.trim();
    let provider_matches_runtime = configured_provider == config.provider;
    let model_looks_valid =
        model_looks_compatible_with_provider(configured_provider, configured_model);
    let recommended_model = cli::default_model_for_provider(configured_provider);

    println!(
        "automation_provider_matches_runtime: {}",
        provider_matches_runtime
    );
    println!(
        "automation_model_looks_valid_for_provider: {}",
        model_looks_valid
    );
    println!("automation_recommended_model: {recommended_model}");

    if !provider_matches_runtime {
        println!(
            "automation_provider_warning: runtime provider '{}' differs from automation provider '{}'",
            config.provider, configured_provider
        );
    }

    if !model_looks_valid {
        println!(
            "automation_model_warning: model '{}' does not look like a normal match for provider '{}'",
            configured_model, configured_provider
        );
    }
}

fn model_looks_compatible_with_provider(provider: &str, model: &str) -> bool {
    if model.is_empty() {
        return false;
    }

    let provider = provider.to_ascii_lowercase();
    let model = model.to_ascii_lowercase();

    match provider.as_str() {
        "anthropic" => model.contains("claude"),
        "google" => model.contains("gemini"),
        "openai" => {
            model.starts_with("gpt")
                || model.starts_with("o1")
                || model.starts_with("o3")
                || model.starts_with("o4")
                || model.starts_with("chatgpt")
                || model.starts_with("codex")
        }
        "ollama" => true,
        "deepseek" => model.contains("deepseek"),
        "openrouter" | "groq" => true,
        _ => true,
    }
}

fn display_thinking_effort(level: ThinkingLevel) -> &'static str {
    match level {
        ThinkingLevel::Off => "off",
        ThinkingLevel::Minimal => "minimal",
        ThinkingLevel::Low => "low",
        ThinkingLevel::Medium => "medium",
        ThinkingLevel::High => "high",
    }
}

fn handle_setup(
    args: Vec<&str>,
    runtime: &mut BondRuntimeContext,
    config: &BondAgentConfig,
) -> Result<ReplDirective> {
    let subcommand = args.first().copied().unwrap_or("status");

    match subcommand {
        "status" => {
            println!("configured: {}", runtime.config.configured);
            println!("autonomous_enabled: {}", runtime.config.autonomous_enabled);
            println!("provider: {}", config.provider);
            println!("provider_source: {}", config.provider_source);
            println!("model: {}", config.model);
            println!("model_source: {}", config.model_source);
            if let Some(issue) = &runtime.config.setup_issue {
                println!("setup_issue_number: {:?}", issue.number);
                println!("setup_issue_state: {:?}", issue.state);
                println!("setup_issue_url: {:?}", issue.url);
            } else {
                println!("setup_issue: none");
            }
            println!("identity_file: {}", runtime.paths.identity_file.display());
            println!(
                "personality_file: {}",
                runtime.paths.personality_file.display()
            );
            println!("journal_file: {}", runtime.paths.journal_file.display());
            println!("config_file: {}", runtime.paths.config_file.display());
            println!("state_file: {}", runtime.paths.state_file.display());
            println!(
                "automation_schedule_cron: {}",
                runtime.config.automation.schedule_cron
            );
            println!(
                "automation_provider: {}",
                runtime.config.automation.provider
            );
            println!("automation_model: {}", runtime.config.automation.model);
            println!(
                "automation_thinking_effort: {}",
                display_thinking_effort(runtime.config.automation.thinking_effort)
            );
            println!(
                "automation_multiple_issues: {}",
                runtime.config.automation.multiple_issues
            );
            println!(
                "workflow_file: {}",
                runtime.paths.bond_workflow_file.display()
            );
            println!(
                "workflow_installed: {}",
                runtime.paths.bond_workflow_file.exists()
            );
            println!("test_commands: {}", runtime.config.commands.test.len());
            println!("lint_commands: {}", runtime.config.commands.lint.len());
            println!(
                "issue_labels: {}",
                runtime.config.issues.eligible_labels.join(", ")
            );
        }
        "issue" => {
            if let Some(issue) = &runtime.config.setup_issue {
                if matches!(issue.state.as_deref(), Some("open")) {
                    println!(
                        "Setup issue already recorded: {}",
                        issue.url.as_deref().unwrap_or("<missing url>")
                    );
                    return Ok(ReplDirective::Continue);
                }
            }

            let repo = detect_github_repo(&runtime.paths.repo_root)?;
            let title = format!(
                "Bond setup: configure {}",
                runtime
                    .paths
                    .repo_root
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("repository")
            );
            let body = build_setup_issue_body(runtime, &repo);
            let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());

            let _ = run_command_capture(
                &gh_bin,
                &[
                    "label",
                    "create",
                    "bond-setup",
                    "--color",
                    "0E8A16",
                    "--description",
                    "Bond onboarding and repository configuration",
                ],
                &runtime.paths.repo_root,
            );

            let url = run_command_capture(
                &gh_bin,
                &[
                    "issue",
                    "create",
                    "--repo",
                    &repo,
                    "--title",
                    &title,
                    "--body",
                    &body,
                    "--label",
                    "bond-setup",
                ],
                &runtime.paths.repo_root,
            )?;
            let issue = SetupIssue {
                number: parse_issue_number(&url),
                state: Some("open".to_string()),
                url: Some(url.clone()),
                title: Some(title),
            };

            runtime.paths.set_setup_issue(Some(issue))?;
            runtime.refresh_config()?;
            println!("Created setup issue: {url}");
        }
        "complete" => {
            runtime.paths.set_configured(true)?;
            runtime.paths.set_autonomous_enabled(true)?;
            runtime.refresh_config()?;
            println!("Bond setup marked complete.");
        }
        "workflow" => {
            let action = args.get(1).copied();

            if action == Some("schedule") {
                let description = collect_schedule_description(args.get(2..).unwrap_or_default());
                if description.is_empty() {
                    bail!("Usage: /setup workflow schedule <human-readable schedule description>");
                }
                let cron = tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current()
                        .block_on(convert_schedule_to_cron(config, &description))
                })?;
                runtime.paths.set_schedule_cron(&cron)?;
                runtime.refresh_config()?;
                println!("Updated schedule_cron to: {cron}");
                println!(
                    "Run: /setup workflow refresh to reinstall the workflow with the new schedule."
                );
            } else {
                let refresh = action == Some("refresh");
                let wrote_workflow = runtime.paths.install_bond_workflow(refresh)?;

                if wrote_workflow {
                    runtime.paths.append_journal_entry(
                        if refresh {
                            "Bond Workflow Refreshed"
                        } else {
                            "Bond Workflow Installed"
                        },
                        &format!(
                            "Workflow file: {}\n\nSchedule: {}\nProvider: {}\nModel: {}",
                            runtime.paths.bond_workflow_file.display(),
                            runtime.config.automation.schedule_cron,
                            runtime.config.automation.provider,
                            runtime.config.automation.model
                        ),
                    )?;
                    println!(
                        "Bond workflow {}: {}",
                        if refresh { "refreshed" } else { "installed" },
                        runtime.paths.bond_workflow_file.display()
                    );
                } else {
                    println!(
                        "Bond workflow already exists: {}",
                        runtime.paths.bond_workflow_file.display()
                    );
                    println!("Use: /setup workflow refresh");
                }
            }
        }
        "reset" => {
            runtime.paths.set_configured(false)?;
            runtime.refresh_config()?;
            println!("Bond setup reset. Autonomous execution is disabled.");
        }
        other => {
            println!("Unknown /setup subcommand: {other}");
            println!(
                "Use: /setup status | /setup issue | /setup workflow [schedule <description>] [refresh] | /setup complete | /setup reset"
            );
        }
    }

    Ok(ReplDirective::Continue)
}

fn collect_schedule_description(args: &[&str]) -> String {
    let joined = args.join(" ");
    let trimmed = joined.trim();
    if (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        || (trimmed.starts_with('"') && trimmed.ends_with('"'))
    {
        trimmed[1..trimmed.len() - 1].to_string()
    } else {
        trimmed.to_string()
    }
}

async fn convert_schedule_to_cron(config: &BondAgentConfig, description: &str) -> Result<String> {
    let system_prompt = "You are a cron expression expert. When given a human-readable schedule \
        description, respond with ONLY a valid 5-field cron expression (minute hour day-of-month \
        month day-of-week). No explanation, no markdown formatting, no code blocks, just the cron \
        expression itself.";
    let mut agent = config.build_minimal_agent(system_prompt)?;
    let prompt = format!("Convert this schedule to a cron expression: {description}");
    let raw = prompt::capture_prompt(&mut agent, &prompt).await?;
    let cron = clean_cron_response(&raw);
    if cron.is_empty() {
        bail!("AI returned an empty cron expression for: {description}");
    }
    Ok(cron)
}

fn clean_cron_response(raw: &str) -> String {
    let text = raw.trim();
    let text = text
        .trim_start_matches("```cron")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    text.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .to_string()
}

pub fn prepare_scheduled_issue_prompt(
    runtime: &mut BondRuntimeContext,
) -> Result<ScheduledExecution> {
    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let pull_requests = load_open_bond_pull_requests(runtime, &repo)?;

    match select_scheduled_pull_request(&pull_requests, runtime.config.automation.multiple_issues) {
        ScheduledPullRequestDisposition::Actionable(pull_request) => {
            let pull_request = load_pull_request_by_number(runtime, &repo, pull_request.number)?;
            let issue_number = pull_request.issue_number()?;
            let issue = load_issue_by_number(runtime, &repo, issue_number)?;
            let target = ScheduledTarget {
                kind: ScheduledTargetKind::PrFeedback,
                issue_number: Some(issue.number),
                issue_title: Some(issue.title.clone()),
                issue_url: Some(issue.url.clone()),
                branch_name: Some(pull_request.head_ref_name.clone()),
                pr_number: Some(pull_request.number),
                pr_title: Some(pull_request.title.clone()),
                pr_url: Some(pull_request.url.clone()),
            };
            checkout_scheduled_target_branch(runtime, &target)?;
            runtime.paths.set_scheduled_target(Some(target))?;
            runtime.refresh_config()?;
            runtime.paths.append_journal_entry(
                "Scheduled PR Feedback Started",
                &format!(
                    "Addressing requested changes on PR #{} {}.\n\nPR URL: {}\nBranch: {}\nRelated issue: #{} [{}] {}\n\n{}",
                    pull_request.number,
                    pull_request.title,
                    pull_request.url,
                    pull_request.head_ref_name,
                    issue.number,
                    issue.primary_label(),
                    issue.title,
                    issue.url
                ),
            )?;
            return Ok(ScheduledExecution::Prompt(
                build_pr_feedback_execution_prompt(&pull_request, &issue),
            ));
        }
        ScheduledPullRequestDisposition::MergeWait(pull_request) => {
            let issue_number = pull_request.issue_number()?;
            let issue = load_issue_by_number(runtime, &repo, issue_number)?;
            let target = ScheduledTarget {
                kind: ScheduledTargetKind::MergeWait,
                issue_number: Some(issue.number),
                issue_title: Some(issue.title.clone()),
                issue_url: Some(issue.url.clone()),
                branch_name: Some(pull_request.head_ref_name.clone()),
                pr_number: Some(pull_request.number),
                pr_title: Some(pull_request.title.clone()),
                pr_url: Some(pull_request.url.clone()),
            };
            runtime.paths.set_scheduled_target(Some(target))?;
            runtime.refresh_config()?;
            let message = format!(
                "Scheduled automation is waiting for merge or approval on PR #{} for issue #{}.",
                pull_request.number, issue.number
            );
            runtime.paths.append_journal_entry(
                "Scheduled PR Merge Wait",
                &format!(
                    "Paused scheduled work behind open PR #{} {}.\n\nPR URL: {}\nBranch: {}\nRelated issue: #{} [{}] {}\n\n{}",
                    pull_request.number,
                    pull_request.title,
                    pull_request.url,
                    pull_request.head_ref_name,
                    issue.number,
                    issue.primary_label(),
                    issue.title,
                    issue.url
                ),
            )?;
            return Ok(ScheduledExecution::Wait(message));
        }
        ScheduledPullRequestDisposition::None => {}
    }

    if runtime.config.current_issue.is_some() {
        let issue = current_issue_detail(runtime)?;
        if matches!(issue.state.as_deref(), Some("CLOSED" | "closed")) {
            runtime.paths.set_current_issue(None, Some("cleared"))?;
            runtime.refresh_config()?;
        } else if issue_matches_workflow(&issue, &runtime.config.issues) {
            let target = scheduled_issue_target(&issue);
            checkout_scheduled_target_branch(runtime, &target)?;
            runtime.paths.set_scheduled_target(Some(target))?;
            runtime.refresh_config()?;
            runtime.paths.append_journal_entry(
                "Scheduled Issue Execution Started",
                &format!(
                    "Continuing issue #{} [{}] {} from the scheduled workflow.\n\n{}",
                    issue.number,
                    issue.primary_label(),
                    issue.title,
                    issue.url
                ),
            )?;
            return Ok(ScheduledExecution::Prompt(build_issue_execution_prompt(
                &issue,
            )));
        } else {
            runtime.paths.set_current_issue(None, Some("cleared"))?;
            runtime.refresh_config()?;
        }
    }

    let issues = load_eligible_issues(runtime, &repo)?;
    if let Some(issue) = select_next_issue(&issues, &runtime.config.issues) {
        persist_current_issue_with_action(runtime, issue, "scheduled")?;
        let target = scheduled_issue_target(issue);
        checkout_scheduled_target_branch(runtime, &target)?;
        runtime.paths.set_scheduled_target(Some(target))?;
        runtime.refresh_config()?;
        runtime.paths.append_journal_entry(
            "Scheduled Issue Execution Started",
            &format!(
                "Selected issue #{} [{}] {} from the scheduled workflow.\n\n{}",
                issue.number,
                issue.primary_label(),
                issue.title,
                issue.url
            ),
        )?;
        return Ok(ScheduledExecution::Prompt(build_issue_execution_prompt(
            issue,
        )));
    }

    runtime.paths.set_scheduled_target(None)?;
    runtime.refresh_config()?;
    Ok(ScheduledExecution::None)
}

fn handle_git(
    args: Vec<&str>,
    runtime: &BondRuntimeContext,
    config: &BondAgentConfig,
) -> Result<ReplDirective> {
    let (program_args, description) = match args.first().copied().unwrap_or("status") {
        "status" => (
            vec!["-c", "color.ui=always", "status", "--short", "--branch"],
            "git status",
        ),
        "diff" => (vec!["-c", "color.ui=always", "diff", "--stat"], "git diff"),
        "log" => (
            vec!["--no-pager", "log", "--oneline", "-n", "10"],
            "git log",
        ),
        other => bail!("Unsupported /git subcommand: {other}. Use status, diff, or log."),
    };

    ensure_command_allowed(&config.permissions, description)?;
    run_process("git", &program_args, &runtime.paths.repo_root, description)?;
    Ok(ReplDirective::Continue)
}

fn handle_issues(args: Vec<&str>, runtime: &mut BondRuntimeContext) -> Result<ReplDirective> {
    let subcommand = args.first().copied().unwrap_or("next");

    match subcommand {
        "list" => {
            let repo = detect_github_repo(&runtime.paths.repo_root)?;
            let issues = load_eligible_issues(runtime, &repo)?;
            if issues.is_empty() {
                println!("No eligible issues found.");
            } else {
                for issue in &issues {
                    println!(
                        "#{} [{}] {}",
                        issue.number,
                        issue.primary_label(),
                        issue.title
                    );
                    println!("{}", issue.url);
                }
            }
        }
        "next" => {
            let repo = detect_github_repo(&runtime.paths.repo_root)?;
            let issues = load_eligible_issues(runtime, &repo)?;
            if let Some(issue) = select_next_issue(&issues, &runtime.config.issues) {
                select_issue(runtime, issue, "Next issue")?;
            } else {
                println!("No eligible issues found.");
            }
        }
        "select" => {
            let issue_number = args
                .get(1)
                .ok_or_else(|| anyhow!("Usage: /issues select <number>"))?
                .parse::<u64>()
                .context("Issue number must be an integer")?;
            let repo = detect_github_repo(&runtime.paths.repo_root)?;
            let issue = load_issue_by_number(runtime, &repo, issue_number)?;

            if !issue_matches_workflow(&issue, &runtime.config.issues) {
                bail!(
                    "Issue #{} does not satisfy the configured intake workflow.",
                    issue.number
                );
            }

            select_issue(runtime, &issue, "Selected issue")?;
        }
        "reopen" => {
            let issue_number = args
                .get(1)
                .ok_or_else(|| anyhow!("Usage: /issues reopen <number> [message]"))?
                .parse::<u64>()
                .context("Issue number must be an integer")?;
            let body = args.iter().skip(2).copied().collect::<Vec<_>>().join(" ");
            reopen_issue(
                runtime,
                issue_number,
                if body.trim().is_empty() {
                    None
                } else {
                    Some(body.trim())
                },
            )?;
        }
        "reopen-current" => {
            let body = args.iter().skip(1).copied().collect::<Vec<_>>().join(" ");
            let last_issue = last_issue_config(runtime)?.clone();
            reopen_issue(
                runtime,
                last_issue.number,
                if body.trim().is_empty() {
                    None
                } else {
                    Some(body.trim())
                },
            )?;
        }
        "resume" => {
            resume_issue(runtime)?;
        }
        "previous" => {
            let issue = previous_issue_config(runtime)?.clone();
            runtime
                .paths
                .set_current_issue(Some(issue.clone()), Some("restored"))?;
            runtime.refresh_config()?;
            runtime.paths.append_journal_entry(
                "Issue Restored",
                &format!(
                    "Restored issue #{} [{}] {} from local history.\n\n{}",
                    issue.number, issue.label, issue.title, issue.url
                ),
            )?;
            println!(
                "Restored previous issue: #{} [{}] {}",
                issue.number, issue.label, issue.title
            );
            println!("{}", issue.url);
        }
        "history" => {
            let filters = args.iter().skip(1).copied().collect::<Vec<_>>();
            let current_issue_number = runtime
                .config
                .current_issue
                .as_ref()
                .map(|issue| issue.number);
            let filtered = runtime
                .config
                .issue_history
                .iter()
                .filter(|issue| {
                    issue_matches_history_filters(issue, &filters, current_issue_number)
                })
                .collect::<Vec<_>>();

            if filtered.is_empty() {
                if filters.is_empty() {
                    println!("No issue history recorded yet.");
                } else {
                    println!(
                        "No issue history entries found for filters: {}.",
                        filters.join(", ")
                    );
                }
            } else {
                if !filters.is_empty() {
                    println!("History filters: {}", filters.join(", "));
                }
                for issue in filtered {
                    println!("#{} [{}] {}", issue.number, issue.label, issue.title);
                    println!("{}", issue.url);
                    print_issue_metadata(issue);
                }
            }
        }
        "park" => {
            let issue = current_issue_config(runtime)?.clone();
            let note = args.iter().skip(1).copied().collect::<Vec<_>>().join(" ");
            runtime.paths.set_current_issue(None, Some("parked"))?;
            runtime.refresh_config()?;
            let body = if note.trim().is_empty() {
                format!(
                    "Parked issue #{} [{}] {}.\n\n{}",
                    issue.number, issue.label, issue.title, issue.url
                )
            } else {
                format!(
                    "Parked issue #{} [{}] {}.\n\n{}\n\n{}",
                    issue.number,
                    issue.label,
                    issue.title,
                    issue.url,
                    note.trim()
                )
            };
            runtime.paths.append_journal_entry("Issue Parked", &body)?;
            println!(
                "Parked current issue: #{} [{}] {}",
                issue.number, issue.label, issue.title
            );
            println!("{}", issue.url);
        }
        "sync" => {
            sync_current_issue(runtime)?;
        }
        "prompt" => {
            let issue = current_issue_detail(runtime)?;
            let prompt = build_issue_execution_prompt(&issue);
            println!("{prompt}");
        }
        "start" => {
            let issue = current_issue_detail(runtime)?;
            let prompt = build_issue_execution_prompt(&issue);
            runtime.paths.append_journal_entry(
                "Issue Execution Started",
                &format!(
                    "Started work on issue #{} [{}] {}\n\n{}",
                    issue.number,
                    issue.primary_label(),
                    issue.title,
                    issue.url
                ),
            )?;
            return Ok(ReplDirective::Prompt(prompt));
        }
        "comment" => {
            let body = args.iter().skip(1).copied().collect::<Vec<_>>().join(" ");
            if body.trim().is_empty() {
                bail!("Usage: /issues comment <message>");
            }
            comment_on_current_issue(runtime, &body)?;
            runtime.paths.append_journal_entry(
                "Issue Comment Added",
                &format!("Added a comment to current issue.\n\n{}", body.trim()),
            )?;
            println!("Commented on current issue.");
        }
        "complete" => {
            let body = args.iter().skip(1).copied().collect::<Vec<_>>().join(" ");
            complete_current_issue(
                runtime,
                if body.trim().is_empty() {
                    None
                } else {
                    Some(body.trim())
                },
            )?;
            println!("Completed current issue and cleared selection.");
        }
        "current" => {
            if let Some(issue) = &runtime.config.current_issue {
                println!(
                    "Current issue: #{} [{}] {}",
                    issue.number, issue.label, issue.title
                );
                println!("{}", issue.url);
            } else {
                println!("No current issue selected.");
            }
        }
        "clear" => {
            runtime.paths.set_current_issue(None, Some("cleared"))?;
            runtime.refresh_config()?;
            runtime.paths.append_journal_entry(
                "Issue Selection Cleared",
                "Cleared the persisted current issue selection.",
            )?;
            println!("Cleared current issue selection.");
        }
        other => {
            println!("Unknown /issues subcommand: {other}");
            println!("Use: /issues list | /issues next | /issues select <number> | /issues reopen <number> [message] | /issues reopen-current [message] | /issues resume | /issues previous | /issues history [action] | /issues park [message] | /issues sync | /issues current | /issues prompt | /issues start | /issues comment <message> | /issues complete [message] | /issues clear");
        }
    }

    Ok(ReplDirective::Continue)
}

fn handle_test(runtime: &BondRuntimeContext, config: &BondAgentConfig) -> Result<ReplDirective> {
    run_repo_commands(
        &runtime.config.commands.test,
        &runtime.paths.repo_root,
        &config.permissions,
    )?;
    Ok(ReplDirective::Continue)
}

fn handle_lint(runtime: &BondRuntimeContext, config: &BondAgentConfig) -> Result<ReplDirective> {
    run_repo_commands(
        &runtime.config.commands.lint,
        &runtime.paths.repo_root,
        &config.permissions,
    )?;
    Ok(ReplDirective::Continue)
}

fn handle_tree(
    args: Vec<&str>,
    runtime: &BondRuntimeContext,
    config: &BondAgentConfig,
) -> Result<ReplDirective> {
    let path_arg = args.first().copied().unwrap_or(".");
    let max_depth = args
        .get(1)
        .copied()
        .or_else(|| {
            args.first()
                .copied()
                .filter(|value| value.chars().all(|ch| ch.is_ascii_digit()))
        })
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(2);
    let path_arg = if path_arg.chars().all(|ch| ch.is_ascii_digit()) {
        "."
    } else {
        path_arg
    };

    let root = resolve_repo_path(&runtime.paths.repo_root, path_arg)?;
    config
        .dir_restrictions
        .check_path(&root.display().to_string())?;
    let header = if root == runtime.paths.repo_root {
        ".".to_string()
    } else {
        relative_display(&root, &runtime.paths.repo_root)
    };

    println!("{header}");
    print_tree_entries(&root, &runtime.paths.repo_root, 0, max_depth)?;
    Ok(ReplDirective::Continue)
}

fn resolve_repo_path(repo_root: &Path, input: &str) -> Result<PathBuf> {
    let candidate = if input == "." {
        repo_root.to_path_buf()
    } else {
        repo_root.join(input)
    };

    let resolved = candidate
        .canonicalize()
        .with_context(|| format!("failed to resolve {}", candidate.display()))?;

    if !resolved.starts_with(repo_root) {
        bail!("Path is outside the repository: {input}");
    }

    Ok(resolved)
}

fn print_tree_entries(dir: &Path, repo_root: &Path, depth: usize, max_depth: usize) -> Result<()> {
    if depth >= max_depth {
        return Ok(());
    }

    let mut entries = fs::read_dir(dir)
        .with_context(|| format!("failed to read {}", dir.display()))?
        .collect::<std::io::Result<Vec<_>>>()
        .with_context(|| format!("failed to read entries under {}", dir.display()))?;

    entries.retain(|entry| !is_tree_excluded(entry.file_name().as_ref()));
    entries.sort_by(|left, right| {
        let left_is_dir = left
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);
        let right_is_dir = right
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);

        right_is_dir
            .cmp(&left_is_dir)
            .then_with(|| left.file_name().cmp(&right.file_name()))
    });

    for (index, entry) in entries.iter().enumerate() {
        let path = entry.path();
        let is_last = index + 1 == entries.len();
        let marker = if is_last { "`--" } else { "|--" };
        let indent = "|   ".repeat(depth);
        let mut label = relative_display(&path, repo_root);
        let is_dir = entry
            .file_type()
            .map(|file_type| file_type.is_dir())
            .unwrap_or(false);

        if is_dir {
            label.push('/');
        }

        println!("{indent}{marker} {label}");

        if is_dir {
            print_tree_entries(&path, repo_root, depth + 1, max_depth)?;
        }
    }

    Ok(())
}

fn is_tree_excluded(name: &OsStr) -> bool {
    matches!(name.to_str(), Some(".git" | "target" | "node_modules"))
}

fn relative_display(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn run_process(program: &str, args: &[&str], cwd: &Path, description: &str) -> Result<()> {
    println!("> {description}");
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run {program}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow!("{program} produced invalid UTF-8 on stdout: {error}"))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|error| anyhow!("{program} produced invalid UTF-8 on stderr: {error}"))?;

    if !stdout.is_empty() {
        print!("{stdout}");
        if !stdout.ends_with('\n') {
            println!();
        }
    }

    if !stderr.is_empty() {
        eprint!("{stderr}");
        if !stderr.ends_with('\n') {
            eprintln!();
        }
    }

    if !output.status.success() {
        bail!("{description} failed with status {}", output.status);
    }

    Ok(())
}

fn detect_github_repo(repo_root: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(repo_root)
        .output()
        .context("failed to run git config --get remote.origin.url")?;

    if !output.status.success() {
        bail!("Repository is not configured with a GitHub origin remote.");
    }

    let remote = String::from_utf8(output.stdout)
        .map_err(|error| anyhow!("git produced invalid UTF-8 on stdout: {error}"))?;
    parse_github_repo_slug(&remote)
}

fn parse_github_repo_slug(remote: &str) -> Result<String> {
    let trimmed = remote.trim();
    let normalized = trimmed
        .strip_prefix("git@github.com:")
        .or_else(|| trimmed.strip_prefix("https://github.com/"))
        .or_else(|| trimmed.strip_prefix("http://github.com/"))
        .or_else(|| trimmed.strip_prefix("ssh://git@github.com/"))
        .ok_or_else(|| anyhow!("Repository is not configured with a GitHub origin remote."))?;

    let slug = normalized.trim_end_matches('/').trim_end_matches(".git");
    let mut parts = slug.split('/');
    let owner = parts.next().unwrap_or_default();
    let repo = parts.next().unwrap_or_default();

    if owner.is_empty() || repo.is_empty() || parts.next().is_some() {
        bail!("Unsupported GitHub remote format: {trimmed}");
    }

    Ok(format!("{owner}/{repo}"))
}

fn build_setup_issue_body(runtime: &BondRuntimeContext, repo: &str) -> String {
    format!(
        "## Inputs\n\n- Repository: `{repo}`\n- Review `.bond/IDENTITY.md`, `.bond/PERSONALITY.md`, and `.bond/JOURNAL.md`.\n- Review the generated prompt-contract templates under `.github/ISSUE_TEMPLATE/`.\n\n## Expected Output\n\n- `.bond` reflects the repository's real purpose, tone, and guardrails.\n- The repository owner confirms onboarding is complete.\n- The bond can move from bootstrap to issue-driven work.\n\n## Constraints\n\n- Do not enable autonomous execution until the `.bond` files are reviewed by a human.\n- Keep repository-specific policy in `.bond`, not only in issue comments.\n- Preserve any existing repository contribution rules.\n\n## Edge Cases\n\n- If this repository is mirrored or read-only, document the write path before closing this issue.\n- If existing issue templates conflict with the generated ones, reconcile them intentionally.\n- If the repository should not use GitHub issues for work intake, document the alternative workflow here.\n\n## Acceptance Criteria\n\n- [ ] `.bond/IDENTITY.md` is customized.\n- [ ] `.bond/PERSONALITY.md` is customized.\n- [ ] `.bond/JOURNAL.md` has a repository-specific onboarding note.\n- [ ] Generated issue templates were reviewed.\n- [ ] `/setup complete` is the next action after this review.\n\nGenerated by doublenot-bond from `{}`.\n",
        runtime.paths.repo_root.display()
    )
}

fn run_command_capture(program: &str, args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to run {program}"))?;

    let stdout = String::from_utf8(output.stdout)
        .map_err(|error| anyhow!("{program} produced invalid UTF-8 on stdout: {error}"))?;
    let stderr = String::from_utf8(output.stderr)
        .map_err(|error| anyhow!("{program} produced invalid UTF-8 on stderr: {error}"))?;

    if !output.status.success() {
        let details = stderr.trim();
        if details.is_empty() {
            bail!("{program} failed with status {}", output.status);
        }
        bail!("{program} failed: {details}");
    }

    Ok(stdout.trim().to_string())
}

fn parse_issue_number(url: &str) -> Option<u64> {
    url.trim()
        .rsplit('/')
        .next()
        .and_then(|segment| segment.parse::<u64>().ok())
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubIssue {
    number: u64,
    title: String,
    body: String,
    url: String,
    #[serde(default)]
    state: Option<String>,
    labels: Vec<GitHubLabel>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubPullRequest {
    number: u64,
    title: String,
    url: String,
    #[serde(default)]
    body: String,
    head_ref_name: String,
    #[serde(default)]
    created_at: Option<String>,
    #[serde(default)]
    review_decision: Option<String>,
    #[serde(default)]
    reviews: Vec<GitHubPullRequestReview>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GitHubPullRequestReview {
    state: String,
    #[serde(default)]
    body: String,
    #[serde(default)]
    submitted_at: Option<String>,
    #[serde(default)]
    author: Option<GitHubActor>,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubActor {
    login: String,
}

#[derive(Debug, Clone, Deserialize)]
struct GitHubLabel {
    name: String,
}

impl GitHubIssue {
    fn label_names(&self) -> Vec<&str> {
        self.labels
            .iter()
            .map(|label| label.name.as_str())
            .collect()
    }

    fn primary_label(&self) -> &str {
        self.labels
            .first()
            .map(|label| label.name.as_str())
            .unwrap_or("unlabeled")
    }
}

impl GitHubPullRequest {
    fn is_bond_issue_pull_request(&self) -> bool {
        self.head_ref_name.starts_with("bond/issue-")
    }

    fn has_requested_changes(&self) -> bool {
        matches!(self.review_decision.as_deref(), Some("CHANGES_REQUESTED"))
    }

    fn issue_number(&self) -> Result<u64> {
        parse_issue_number_from_branch_name(&self.head_ref_name).ok_or_else(|| {
            anyhow!(
                "Open bond PR #{} uses an unsupported branch name: {}",
                self.number,
                self.head_ref_name
            )
        })
    }
}

fn current_issue_detail(runtime: &BondRuntimeContext) -> Result<GitHubIssue> {
    let selected = runtime
        .config
        .current_issue
        .as_ref()
        .ok_or_else(|| anyhow!("No current issue selected. Use /issues next first."))?;

    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    load_issue_by_number(runtime, &repo, selected.number)
}

fn load_issue_by_number(
    runtime: &BondRuntimeContext,
    repo: &str,
    issue_number: u64,
) -> Result<GitHubIssue> {
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());
    let raw = run_command_capture(
        &gh_bin,
        &[
            "issue",
            "view",
            &issue_number.to_string(),
            "--repo",
            repo,
            "--json",
            "number,title,body,url,state,labels",
        ],
        &runtime.paths.repo_root,
    )?;

    serde_json::from_str(&raw).context("failed to parse gh issue view JSON")
}

fn select_issue(
    runtime: &mut BondRuntimeContext,
    issue: &GitHubIssue,
    heading: &str,
) -> Result<()> {
    persist_current_issue_with_action(runtime, issue, "selected")?;
    runtime.paths.append_journal_entry(
        "Issue Selected",
        &format!(
            "Selected issue #{} [{}] {}\n\n{}",
            issue.number,
            issue.primary_label(),
            issue.title,
            issue.url
        ),
    )?;
    runtime.refresh_config()?;
    println!(
        "{heading}: #{} [{}] {}",
        issue.number,
        issue.primary_label(),
        issue.title
    );
    println!("{}", issue.url);
    println!();
    println!("Body:");
    println!("{}", issue.body.trim());
    Ok(())
}

fn sync_current_issue(runtime: &mut BondRuntimeContext) -> Result<()> {
    let selected = current_issue_config(runtime)?.clone();
    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let issue = load_issue_by_number(runtime, &repo, selected.number)?;

    if matches!(issue.state.as_deref(), Some("CLOSED" | "closed")) {
        runtime.paths.set_current_issue(None, Some("cleared"))?;
        runtime.refresh_config()?;
        runtime.paths.append_journal_entry(
            "Issue Selection Cleared",
            &format!(
                "Cleared current issue selection after GitHub reported issue #{} as closed.\n\n{}",
                issue.number, issue.url
            ),
        )?;
        println!("Current issue is closed on GitHub. Cleared selection.");
        return Ok(());
    }

    persist_current_issue_with_action(runtime, &issue, "synced")?;
    runtime.paths.append_journal_entry(
        "Issue Synced",
        &format!(
            "Synchronized current issue #{} [{}] {}\n\n{}",
            issue.number,
            issue.primary_label(),
            issue.title,
            issue.url
        ),
    )?;
    println!(
        "Synchronized current issue: #{} [{}] {}",
        issue.number,
        issue.primary_label(),
        issue.title
    );
    println!("{}", issue.url);
    if !issue_matches_workflow(&issue, &runtime.config.issues) {
        println!("Warning: the current issue no longer matches the configured intake workflow.");
    }
    Ok(())
}

fn reopen_issue(
    runtime: &mut BondRuntimeContext,
    issue_number: u64,
    body: Option<&str>,
) -> Result<()> {
    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());

    run_command_capture(
        &gh_bin,
        &[
            "issue",
            "reopen",
            &issue_number.to_string(),
            "--repo",
            &repo,
        ],
        &runtime.paths.repo_root,
    )?;

    if let Some(body) = body {
        comment_on_issue_number(runtime, issue_number, body)?;
    }

    let issue = load_issue_by_number(runtime, &repo, issue_number)?;
    if !issue_matches_workflow(&issue, &runtime.config.issues) {
        bail!(
            "Issue #{} does not satisfy the configured intake workflow.",
            issue.number
        );
    }

    persist_current_issue_with_action(runtime, &issue, "reopened")?;
    runtime.paths.append_journal_entry(
        "Issue Reopened",
        &format!(
            "Reopened issue #{} [{}] {} and restored it as the current selection.\n\n{}{}",
            issue.number,
            issue.primary_label(),
            issue.title,
            issue.url,
            body.map(|text| format!("\n\n{}", text)).unwrap_or_default()
        ),
    )?;
    println!(
        "Reopened issue: #{} [{}] {}",
        issue.number,
        issue.primary_label(),
        issue.title
    );
    println!("{}", issue.url);
    Ok(())
}

fn persist_current_issue_with_action(
    runtime: &mut BondRuntimeContext,
    issue: &GitHubIssue,
    action: &str,
) -> Result<()> {
    runtime.paths.set_current_issue(
        Some(CurrentIssue {
            number: issue.number,
            title: issue.title.clone(),
            url: issue.url.clone(),
            label: issue.primary_label().to_string(),
            last_action: None,
            last_action_at: None,
        }),
        Some(action),
    )?;
    runtime.refresh_config()?;
    Ok(())
}

fn build_issue_execution_prompt(issue: &GitHubIssue) -> String {
    format!(
        "Work on GitHub issue #{} [{}] {}.\n\nIssue URL: {}\n\nIssue body:\n{}\n\nInstructions:\n- Inspect the repository before changing code.\n- Follow the issue's prompt-contract sections as the task contract.\n- Make focused changes only for this issue.\n- If you change files, stage and commit them with a focused git message before concluding.\n- Run relevant verification before concluding.\n- Summarize what changed, what was verified, and any blockers.",
        issue.number,
        issue.primary_label(),
        issue.title,
        issue.url,
        issue.body.trim()
    )
}

fn build_pr_feedback_execution_prompt(
    pull_request: &GitHubPullRequest,
    issue: &GitHubIssue,
) -> String {
    let requested_changes = requested_changes_summary(&pull_request.reviews);
    let pull_request_body = if pull_request.body.trim().is_empty() {
        "(no PR body provided)"
    } else {
        pull_request.body.trim()
    };

    format!(
        "Address requested changes on existing GitHub PR #{} {} for issue #{} [{}] {}.\n\nPR URL: {}\nIssue URL: {}\nTarget branch: {}\nReview decision: {}\n\nPR body:\n{}\n\nIssue body:\n{}\n\nRequested changes:\n{}\nInstructions:\n- Work only on the existing PR branch for this issue.\n- Address explicit requested changes before starting any new issue work.\n- Keep the update scoped to the current PR and issue.\n- Run relevant verification before concluding.\n- Summarize what changed, what was verified, and any blockers.",
        pull_request.number,
        pull_request.title,
        issue.number,
        issue.primary_label(),
        issue.title,
        pull_request.url,
        issue.url,
        pull_request.head_ref_name,
        pull_request
            .review_decision
            .as_deref()
            .unwrap_or("unknown"),
        pull_request_body,
        issue.body.trim(),
        requested_changes
    )
}

fn current_issue_config(runtime: &BondRuntimeContext) -> Result<&CurrentIssue> {
    runtime
        .config
        .current_issue
        .as_ref()
        .ok_or_else(|| anyhow!("No current issue selected. Use /issues next first."))
}

fn last_issue_config(runtime: &BondRuntimeContext) -> Result<&CurrentIssue> {
    runtime
        .config
        .last_issue
        .as_ref()
        .ok_or_else(|| anyhow!("No last issue recorded yet. Select or complete an issue first."))
}

fn previous_issue_config(runtime: &BondRuntimeContext) -> Result<&CurrentIssue> {
    let current_number = runtime
        .config
        .current_issue
        .as_ref()
        .map(|issue| issue.number);

    let previous = if let Some(current_number) = current_number {
        runtime
            .config
            .issue_history
            .iter()
            .find(|issue| issue.number != current_number)
    } else {
        runtime.config.issue_history.first()
    };

    previous.ok_or_else(|| anyhow!("No previous issue recorded yet. Select an issue first."))
}

fn resume_issue(runtime: &mut BondRuntimeContext) -> Result<()> {
    if let Some(issue) = runtime.config.current_issue.clone() {
        runtime
            .paths
            .set_current_issue(Some(issue.clone()), Some("resumed"))?;
        runtime.refresh_config()?;
        runtime.paths.append_journal_entry(
            "Issue Resumed",
            &format!(
                "Resumed current issue #{} [{}] {}.\n\n{}",
                issue.number, issue.label, issue.title, issue.url
            ),
        )?;
        println!(
            "Resuming current issue: #{} [{}] {}",
            issue.number, issue.label, issue.title
        );
        println!("{}", issue.url);
        return Ok(());
    }

    if let Ok(issue) = previous_issue_config(runtime).cloned() {
        runtime
            .paths
            .set_current_issue(Some(issue.clone()), Some("resumed"))?;
        runtime.refresh_config()?;
        runtime.paths.append_journal_entry(
            "Issue Resumed",
            &format!(
                "Resumed prior issue #{} [{}] {} from local history.\n\n{}",
                issue.number, issue.label, issue.title, issue.url
            ),
        )?;
        println!(
            "Resumed previous issue: #{} [{}] {}",
            issue.number, issue.label, issue.title
        );
        println!("{}", issue.url);
        return Ok(());
    }

    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let issues = load_eligible_issues(runtime, &repo)?;
    if let Some(issue) = select_next_issue(&issues, &runtime.config.issues) {
        select_issue(runtime, issue, "Resumed next issue")?;
    } else {
        println!("No current, previous, or eligible issues found.");
    }

    Ok(())
}

fn comment_on_current_issue(runtime: &BondRuntimeContext, body: &str) -> Result<()> {
    let issue = current_issue_config(runtime)?;
    comment_on_issue_number(runtime, issue.number, body)
}

fn comment_on_issue_number(
    runtime: &BondRuntimeContext,
    issue_number: u64,
    body: &str,
) -> Result<()> {
    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());

    run_command_capture(
        &gh_bin,
        &[
            "issue",
            "comment",
            &issue_number.to_string(),
            "--repo",
            &repo,
            "--body",
            body,
        ],
        &runtime.paths.repo_root,
    )?;

    Ok(())
}

fn complete_current_issue(runtime: &mut BondRuntimeContext, body: Option<&str>) -> Result<()> {
    let issue = current_issue_config(runtime)?.clone();
    if let Some(body) = body {
        comment_on_current_issue(runtime, body)?;
    }

    let repo = detect_github_repo(&runtime.paths.repo_root)?;
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());
    run_command_capture(
        &gh_bin,
        &["issue", "close", &issue.number.to_string(), "--repo", &repo],
        &runtime.paths.repo_root,
    )?;

    runtime.paths.set_current_issue(None, Some("completed"))?;
    runtime.refresh_config()?;
    let note = body.unwrap_or("Closed the current issue.");
    runtime.paths.append_journal_entry(
        "Issue Completed",
        &format!(
            "Completed issue #{} [{}] {}\n\n{}\n\n{}",
            issue.number, issue.label, issue.title, issue.url, note
        ),
    )?;

    Ok(())
}

fn print_issue_metadata(issue: &CurrentIssue) {
    if let Some(action) = issue.last_action.as_deref() {
        println!("action: {action}");
    }
    if let Some(timestamp) = issue.last_action_at.as_deref() {
        println!("at: {timestamp}");
    }
}

fn print_issue_posture(runtime: &BondRuntimeContext) {
    let parked_issues = runtime
        .config
        .issue_history
        .iter()
        .filter(|issue| issue.last_action.as_deref() == Some("parked"))
        .collect::<Vec<_>>();
    let active_count = usize::from(runtime.config.current_issue.is_some());

    println!(
        "issue_posture: active={}, parked={}",
        active_count,
        parked_issues.len()
    );

    if let Some(issue) = parked_issues.first() {
        println!(
            "latest_parked: #{} [{}] {}",
            issue.number, issue.label, issue.title
        );
    } else {
        println!("latest_parked: none");
    }
}

fn issue_matches_history_filters(
    issue: &CurrentIssue,
    filters: &[&str],
    current_issue_number: Option<u64>,
) -> bool {
    filters.iter().all(|filter| match *filter {
        "parked" => issue.last_action.as_deref() == Some("parked"),
        "current" => Some(issue.number) == current_issue_number,
        other => {
            if let Some(action) = other.strip_prefix("action:") {
                issue.last_action.as_deref() == Some(action)
            } else if let Some(label) = other.strip_prefix("label:") {
                issue.label.eq_ignore_ascii_case(label)
            } else {
                issue.last_action.as_deref() == Some(other)
            }
        }
    })
}

fn load_eligible_issues(runtime: &BondRuntimeContext, repo: &str) -> Result<Vec<GitHubIssue>> {
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());
    let raw = run_command_capture(
        &gh_bin,
        &[
            "issue",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,body,url,labels",
        ],
        &runtime.paths.repo_root,
    )?;

    let issues: Vec<GitHubIssue> =
        serde_json::from_str(&raw).context("failed to parse gh issue list JSON")?;
    let mut eligible = Vec::new();
    for issue in issues {
        if should_mark_format_issue(&issue, &runtime.config.issues) {
            let _ = ensure_format_issue_label(&gh_bin, &runtime.paths.repo_root, repo, &issue);
            continue;
        }

        if should_mark_blocked_dependency_issue(runtime, repo, &issue)? {
            let _ =
                ensure_blocked_dependency_label(&gh_bin, &runtime.paths.repo_root, repo, &issue);
            continue;
        }

        if issue_matches_workflow(&issue, &runtime.config.issues) {
            eligible.push(issue);
        }
    }

    Ok(eligible)
}

fn load_open_bond_pull_requests(
    runtime: &BondRuntimeContext,
    repo: &str,
) -> Result<Vec<GitHubPullRequest>> {
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());
    let raw = run_command_capture(
        &gh_bin,
        &[
            "pr",
            "list",
            "--repo",
            repo,
            "--state",
            "open",
            "--limit",
            "100",
            "--json",
            "number,title,url,headRefName,reviewDecision,createdAt",
        ],
        &runtime.paths.repo_root,
    )?;

    let mut pull_requests: Vec<GitHubPullRequest> =
        serde_json::from_str(&raw).context("failed to parse gh pr list JSON")?;
    pull_requests.retain(GitHubPullRequest::is_bond_issue_pull_request);
    pull_requests.sort_by(|left, right| {
        left.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(right.created_at.as_deref().unwrap_or(""))
            .then_with(|| left.number.cmp(&right.number))
            .then_with(|| left.head_ref_name.cmp(&right.head_ref_name))
    });
    Ok(pull_requests)
}

fn load_pull_request_by_number(
    runtime: &BondRuntimeContext,
    repo: &str,
    pull_request_number: u64,
) -> Result<GitHubPullRequest> {
    let gh_bin = env::var("BOND_GH_BIN").unwrap_or_else(|_| "gh".to_string());
    let raw = run_command_capture(
        &gh_bin,
        &[
            "pr",
            "view",
            &pull_request_number.to_string(),
            "--repo",
            repo,
            "--json",
            "number,title,url,body,headRefName,reviewDecision,reviews",
        ],
        &runtime.paths.repo_root,
    )?;

    serde_json::from_str(&raw).context("failed to parse gh pr view JSON")
}

fn issue_matches_workflow(issue: &GitHubIssue, workflow: &IssueWorkflow) -> bool {
    let label_names = issue.label_names();
    if label_names.iter().any(|label| {
        label.eq_ignore_ascii_case("blocked")
            || label.eq_ignore_ascii_case("needs-human")
            || label.eq_ignore_ascii_case("format-issue")
            || label.eq_ignore_ascii_case("blocked-dependent")
    }) {
        return false;
    }

    let label_match = label_names.iter().any(|label| {
        workflow
            .eligible_labels
            .iter()
            .any(|eligible| eligible == label)
    });

    if !label_match {
        return false;
    }

    if workflow.require_prompt_contract {
        return has_prompt_contract_sections(&issue.body);
    }

    true
}

fn should_mark_format_issue(issue: &GitHubIssue, workflow: &IssueWorkflow) -> bool {
    if !workflow.require_prompt_contract {
        return false;
    }

    let label_names = issue.label_names();
    let eligible = label_names.iter().any(|label| {
        workflow
            .eligible_labels
            .iter()
            .any(|eligible| eligible == label)
    });

    eligible
        && !label_names
            .iter()
            .any(|label| label.eq_ignore_ascii_case("format-issue"))
        && !has_prompt_contract_sections(&issue.body)
}

fn should_mark_blocked_dependency_issue(
    runtime: &BondRuntimeContext,
    repo: &str,
    issue: &GitHubIssue,
) -> Result<bool> {
    if !issue_matches_workflow(issue, &runtime.config.issues) {
        return Ok(false);
    }

    for dependency in dependency_issue_numbers(&issue.body) {
        let dependency_issue = load_issue_by_number(runtime, repo, dependency);
        match dependency_issue {
            Ok(issue) if !matches!(issue.state.as_deref(), Some("CLOSED" | "closed")) => {
                return Ok(true);
            }
            Ok(_) => {}
            Err(_) => return Ok(true),
        }
    }

    Ok(false)
}

fn ensure_format_issue_label(
    gh_bin: &str,
    repo_root: &Path,
    repo: &str,
    issue: &GitHubIssue,
) -> Result<()> {
    let _ = run_command_capture(
        gh_bin,
        &[
            "label",
            "create",
            "format-issue",
            "--color",
            "D93F0B",
            "--description",
            "Issue does not match the required prompt-contract format",
        ],
        repo_root,
    );

    run_command_capture(
        gh_bin,
        &[
            "issue",
            "edit",
            &issue.number.to_string(),
            "--repo",
            repo,
            "--add-label",
            "format-issue",
        ],
        repo_root,
    )?;

    Ok(())
}

fn ensure_blocked_dependency_label(
    gh_bin: &str,
    repo_root: &Path,
    repo: &str,
    issue: &GitHubIssue,
) -> Result<()> {
    let _ = run_command_capture(
        gh_bin,
        &[
            "label",
            "create",
            "blocked-dependent",
            "--color",
            "B60205",
            "--description",
            "Issue depends on unresolved prior work and should wait for that dependency",
        ],
        repo_root,
    );

    run_command_capture(
        gh_bin,
        &[
            "issue",
            "edit",
            &issue.number.to_string(),
            "--repo",
            repo,
            "--add-label",
            "blocked-dependent",
        ],
        repo_root,
    )?;

    Ok(())
}

fn dependency_issue_numbers(body: &str) -> Vec<u64> {
    let mut numbers = Vec::new();

    for line in body.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(index) = lower.find("depends on:") {
            let remainder = &line[index + "depends on:".len()..];
            numbers.extend(extract_issue_numbers(remainder));
        }
    }

    numbers.sort_unstable();
    numbers.dedup();
    numbers
}

fn extract_issue_numbers(text: &str) -> Vec<u64> {
    let mut numbers = Vec::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '#' {
            continue;
        }

        let mut digits = String::new();
        while let Some(next) = chars.peek() {
            if next.is_ascii_digit() {
                digits.push(*next);
                chars.next();
            } else {
                break;
            }
        }

        if let Ok(number) = digits.parse::<u64>() {
            numbers.push(number);
        }
    }

    numbers
}

fn has_prompt_contract_sections(body: &str) -> bool {
    const REQUIRED: [&str; 5] = [
        "## Inputs",
        "## Expected Output",
        "## Constraints",
        "## Edge Cases",
        "## Acceptance Criteria",
    ];

    REQUIRED.iter().all(|section| body.contains(section))
}

fn select_next_issue<'a>(
    issues: &'a [GitHubIssue],
    workflow: &IssueWorkflow,
) -> Option<&'a GitHubIssue> {
    let mut ranked: Vec<&GitHubIssue> = issues.iter().collect();
    ranked.sort_by(|left, right| {
        issue_priority(left, workflow)
            .cmp(&issue_priority(right, workflow))
            .then_with(|| left.number.cmp(&right.number))
    });
    ranked.into_iter().next()
}

fn scheduled_issue_target(issue: &GitHubIssue) -> ScheduledTarget {
    ScheduledTarget {
        kind: ScheduledTargetKind::Issue,
        issue_number: Some(issue.number),
        issue_title: Some(issue.title.clone()),
        issue_url: Some(issue.url.clone()),
        branch_name: Some(issue_branch_name(issue.number, &issue.title)),
        pr_number: None,
        pr_title: None,
        pr_url: None,
    }
}

fn requested_changes_summary(reviews: &[GitHubPullRequestReview]) -> String {
    let requested = reviews
        .iter()
        .filter(|review| review.state == "CHANGES_REQUESTED")
        .collect::<Vec<_>>();

    if requested.is_empty() {
        return "- GitHub reported requested changes, but no review body was returned. Inspect the PR review history directly if needed.".to_string();
    }

    requested
        .iter()
        .enumerate()
        .map(|(index, review)| {
            let author = review
                .author
                .as_ref()
                .map(|author| author.login.as_str())
                .unwrap_or("unknown reviewer");
            let submitted_at = review.submitted_at.as_deref().unwrap_or("unknown time");
            let body = if review.body.trim().is_empty() {
                "(no review body provided)"
            } else {
                review.body.trim()
            };
            format!("{}. {} at {}\n{}", index + 1, author, submitted_at, body)
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn parse_issue_number_from_branch_name(branch_name: &str) -> Option<u64> {
    let remainder = branch_name.strip_prefix("bond/issue-")?;
    let digits = remainder
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();

    if digits.is_empty() {
        return None;
    }

    digits.parse().ok()
}

fn select_scheduled_pull_request(
    pull_requests: &[GitHubPullRequest],
    multiple_issues: bool,
) -> ScheduledPullRequestDisposition<'_> {
    let ordering = |left: &&GitHubPullRequest, right: &&GitHubPullRequest| {
        left.created_at
            .as_deref()
            .unwrap_or("")
            .cmp(right.created_at.as_deref().unwrap_or(""))
            .then_with(|| left.number.cmp(&right.number))
            .then_with(|| left.head_ref_name.cmp(&right.head_ref_name))
    };

    if let Some(pull_request) = pull_requests
        .iter()
        .filter(|pull_request| pull_request.has_requested_changes())
        .min_by(ordering)
    {
        return ScheduledPullRequestDisposition::Actionable(pull_request);
    }

    if multiple_issues {
        ScheduledPullRequestDisposition::None
    } else if let Some(pull_request) = pull_requests.iter().min_by(ordering) {
        ScheduledPullRequestDisposition::MergeWait(pull_request)
    } else {
        ScheduledPullRequestDisposition::None
    }
}

fn checkout_scheduled_target_branch(
    runtime: &BondRuntimeContext,
    target: &ScheduledTarget,
) -> Result<()> {
    let Some(branch_name) = target.branch_name.as_deref() else {
        return Ok(());
    };

    let _ = Command::new("git")
        .args(["fetch", "origin", branch_name])
        .current_dir(&runtime.paths.repo_root)
        .output();

    let remote_ref = format!("refs/remotes/origin/{branch_name}");
    if git_ref_exists(&runtime.paths.repo_root, &remote_ref)? {
        run_command_capture(
            "git",
            &[
                "checkout",
                "-B",
                branch_name,
                &format!("origin/{branch_name}"),
            ],
            &runtime.paths.repo_root,
        )?;
        return Ok(());
    }

    let current_branch = run_command_capture(
        "git",
        &["branch", "--show-current"],
        &runtime.paths.repo_root,
    )?;
    if current_branch.trim() == branch_name {
        return Ok(());
    }

    let local_ref = format!("refs/heads/{branch_name}");
    if git_ref_exists(&runtime.paths.repo_root, &local_ref)? {
        run_command_capture("git", &["checkout", branch_name], &runtime.paths.repo_root)?;
    } else {
        run_command_capture(
            "git",
            &["checkout", "-b", branch_name],
            &runtime.paths.repo_root,
        )?;
    }

    Ok(())
}

fn git_ref_exists(repo_root: &Path, reference: &str) -> Result<bool> {
    let status = Command::new("git")
        .args(["show-ref", "--verify", "--quiet", reference])
        .current_dir(repo_root)
        .status()
        .context("failed to run git show-ref")?;
    Ok(status.success())
}

fn issue_priority(issue: &GitHubIssue, workflow: &IssueWorkflow) -> usize {
    let labels = issue.label_names();
    workflow
        .priority_labels
        .iter()
        .position(|priority| labels.iter().any(|label| label == priority))
        .unwrap_or(workflow.priority_labels.len())
}

fn run_repo_commands(
    commands: &[RepoCommand],
    repo_root: &Path,
    permissions: &crate::cli::PermissionConfig,
) -> Result<()> {
    if commands.is_empty() {
        bail!("No commands configured for this workflow in .bond/config.yml.");
    }

    for command in commands {
        ensure_command_allowed(permissions, &command.description)?;
        let arg_refs: Vec<&str> = command.args.iter().map(String::as_str).collect();
        run_process(&command.program, &arg_refs, repo_root, &command.description)?;
    }

    Ok(())
}

fn ensure_command_allowed(permissions: &crate::cli::PermissionConfig, command: &str) -> Result<()> {
    match permissions.check(command) {
        Some(false) => bail!("Command denied by permission rule: {command}"),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        parse_issue_number_from_branch_name, select_scheduled_pull_request, GitHubPullRequest,
        GitHubPullRequestReview, ScheduledPullRequestDisposition,
    };

    fn pull_request(
        number: u64,
        created_at: &str,
        review_decision: Option<&str>,
    ) -> GitHubPullRequest {
        GitHubPullRequest {
            number,
            title: format!("PR {number}"),
            url: format!("https://github.com/acme/widgets/pull/{number}"),
            body: String::new(),
            head_ref_name: format!("bond/issue-{number}-example"),
            created_at: Some(created_at.to_string()),
            review_decision: review_decision.map(str::to_string),
            reviews: vec![GitHubPullRequestReview {
                state: review_decision.unwrap_or("REVIEW_REQUIRED").to_string(),
                body: String::new(),
                submitted_at: None,
                author: None,
            }],
        }
    }

    #[test]
    fn parse_issue_number_from_branch_name_extracts_issue_number() {
        assert_eq!(
            parse_issue_number_from_branch_name("bond/issue-123-fix-parser"),
            Some(123)
        );
        assert_eq!(parse_issue_number_from_branch_name("bond/issue-9"), Some(9));
        assert_eq!(parse_issue_number_from_branch_name("feature/example"), None);
    }

    #[test]
    fn scheduled_pull_request_selection_prefers_oldest_requested_changes() {
        let pull_requests = vec![
            pull_request(42, "2026-04-05T12:00:00Z", Some("CHANGES_REQUESTED")),
            pull_request(12, "2026-04-05T10:00:00Z", Some("CHANGES_REQUESTED")),
            pull_request(18, "2026-04-05T11:00:00Z", Some("APPROVED")),
        ];

        match select_scheduled_pull_request(&pull_requests, true) {
            ScheduledPullRequestDisposition::Actionable(pull_request) => {
                assert_eq!(pull_request.number, 12);
            }
            _ => panic!("expected actionable PR selection"),
        }
    }

    #[test]
    fn scheduled_pull_request_selection_returns_merge_wait_in_single_issue_mode() {
        let pull_requests = vec![pull_request(12, "2026-04-05T10:00:00Z", Some("APPROVED"))];

        match select_scheduled_pull_request(&pull_requests, false) {
            ScheduledPullRequestDisposition::MergeWait(pull_request) => {
                assert_eq!(pull_request.number, 12);
            }
            _ => panic!("expected merge-wait selection"),
        }
    }

    #[test]
    fn scheduled_pull_request_selection_skips_merge_wait_in_multi_issue_mode() {
        let pull_requests = vec![pull_request(12, "2026-04-05T10:00:00Z", Some("APPROVED"))];

        assert!(matches!(
            select_scheduled_pull_request(&pull_requests, true),
            ScheduledPullRequestDisposition::None
        ));
    }

    #[test]
    fn collect_schedule_description_joins_args_and_strips_quotes() {
        use super::collect_schedule_description;

        assert_eq!(
            collect_schedule_description(&["every", "6", "hours"]),
            "every 6 hours"
        );
        // REPL tokenizes `'every 6 hours'` into `["'every", "6", "hours'"]`
        // joining and then stripping surrounding quotes gives "every 6 hours"
        assert_eq!(
            collect_schedule_description(&["'every", "6", "hours'"]),
            "every 6 hours"
        );
        assert_eq!(
            collect_schedule_description(&["'every 6 hours'"]),
            "every 6 hours"
        );
        assert_eq!(
            collect_schedule_description(&["\"every 6 hours\""]),
            "every 6 hours"
        );
        assert_eq!(collect_schedule_description(&[]), "");
    }

    #[test]
    fn clean_cron_response_strips_markdown_and_extra_lines() {
        use super::clean_cron_response;

        assert_eq!(clean_cron_response("0 */6 * * *"), "0 */6 * * *");
        assert_eq!(clean_cron_response("```\n0 */6 * * *\n```"), "0 */6 * * *");
        assert_eq!(
            clean_cron_response("```cron\n0 */6 * * *\n```"),
            "0 */6 * * *"
        );
        assert_eq!(
            clean_cron_response("  0 */6 * * *  \n\nsome explanation"),
            "0 */6 * * *"
        );
    }
}
