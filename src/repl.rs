use crate::agent::BondAgentConfig;
use crate::bond::BondRuntimeContext;
use crate::commands::{dispatch_command, ReplDirective};
use crate::prompt;
use anyhow::Result;
use rustyline::DefaultEditor;
use yoagent::agent::Agent;

pub async fn run_repl(runtime: &mut BondRuntimeContext, config: &BondAgentConfig) -> Result<()> {
    let mut editor = DefaultEditor::new()?;
    let mut agent: Option<Agent> = None;

    if !runtime.config.configured {
        println!("Bond setup is incomplete.");
        println!("Run /setup status to inspect configuration or /setup complete once the .bond files are ready.");
    }

    loop {
        let line = match editor.readline("bond> ") {
            Ok(line) => line,
            Err(rustyline::error::ReadlineError::Interrupted)
            | Err(rustyline::error::ReadlineError::Eof) => break,
            Err(error) => return Err(error.into()),
        };

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        editor.add_history_entry(trimmed)?;

        if trimmed.starts_with('/') {
            match dispatch_command(trimmed, runtime, config) {
                Ok(ReplDirective::Continue) => continue,
                Ok(ReplDirective::Exit) => break,
                Ok(ReplDirective::Prompt(prompt_text)) => {
                    let agent = match agent.as_mut() {
                        Some(agent) => agent,
                        None => {
                            agent = Some(config.build_agent()?);
                            agent.as_mut().expect("agent should be initialized")
                        }
                    };
                    prompt::run_prompt(agent, &prompt_text).await?;
                    continue;
                }
                Err(error) => {
                    eprintln!("error: {error:#}");
                    continue;
                }
            }
        } else {
            let agent = match agent.as_mut() {
                Some(agent) => agent,
                None => {
                    agent = Some(config.build_agent()?);
                    agent.as_mut().expect("agent should be initialized")
                }
            };
            prompt::run_prompt(agent, trimmed).await?;
        }
    }

    Ok(())
}
