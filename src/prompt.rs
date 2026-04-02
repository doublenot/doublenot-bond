use anyhow::{bail, Result};
use yoagent::agent::Agent;
use yoagent::{AgentEvent, AgentMessage, Content, Message, StreamDelta};

pub async fn run_prompt(agent: &mut Agent, input: &str) -> Result<()> {
    let mut rx = agent.prompt(input).await;
    let mut saw_text = false;

    while let Some(event) = rx.recv().await {
        match event {
            AgentEvent::MessageUpdate {
                delta: StreamDelta::Text { delta },
                ..
            } => {
                print!("{delta}");
                saw_text = true;
            }
            AgentEvent::MessageUpdate {
                delta: StreamDelta::Thinking { .. },
                ..
            } => {}
            AgentEvent::ToolExecutionStart { tool_name, .. } => {
                eprintln!("\n[tool:start] {tool_name}");
            }
            AgentEvent::ToolExecutionEnd {
                tool_name,
                is_error,
                ..
            } => {
                if is_error {
                    eprintln!("[tool:error] {tool_name}");
                } else {
                    eprintln!("[tool:done] {tool_name}");
                }
            }
            AgentEvent::MessageEnd { message, .. } => {
                if !saw_text {
                    let fallback = extract_text(&message);
                    if !fallback.is_empty() {
                        print!("{fallback}");
                        saw_text = true;
                    }
                }
            }
            _ => {}
        }
    }

    if !saw_text {
        bail!("The agent returned no text response.");
    }

    println!();
    Ok(())
}

fn extract_text(message: &AgentMessage) -> String {
    let Some(message) = message.as_llm() else {
        return String::new();
    };

    let content = match message {
        Message::User { content, .. }
        | Message::Assistant { content, .. }
        | Message::ToolResult { content, .. } => content,
    };

    content
        .iter()
        .filter_map(|block| match block {
            Content::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect::<String>()
}
