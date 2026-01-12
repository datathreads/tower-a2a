use std::time::Duration;

use tower_a2a::{
    prelude::*,
    protocol::{AgentCapabilities, TaskError},
};

// Configuration - update these to match your agent
const AGENT_URL: &str = "https://your-agent-url";
const AUTH_TOKEN: &str = "your-auth-token";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("🚀 Tower-A2A Simple Client Example\n");

    // Build the A2A client with HTTP transport and bearer authentication
    let url = AGENT_URL.parse().unwrap();
    let mut client = A2AClientBuilder::new(url)
        .with_http()
        .with_bearer_auth(AUTH_TOKEN.to_string())
        .with_timeout(Duration::from_secs(30))
        .build()?;

    println!("✓ Client configured for: {AGENT_URL}\n");

    // Step 1: Discover agent capabilities
    println!("📋 Discovering agent capabilities...");
    match client.discover().await {
        Ok(AgentCard {
            name,
            description,
            capabilities:
                AgentCapabilities {
                    streaming,
                    task_management,
                    multi_turn,
                    ..
                },
            ..
        }) => {
            println!("✓ Connected to: {name}");
            println!("  Description: {description}");
            println!("  Capabilities:");
            println!("    - Streaming: {streaming}");
            println!("    - Task Management: {task_management}");
            println!("    - Multi-turn: {multi_turn}");
            println!();
        }
        Err(e) => {
            eprintln!(
                r#"✗ Failed to discover agent: {e}

    Note: Make sure AGENT_URL points to a running A2A agent"#
            );
            return Ok(());
        }
    }

    // Step 2: Send a message to the agent
    println!("💬 Sending message to agent...");
    let message = Message::user("What is the weather like in San Francisco?");

    let (id, output) = match client.send_message(message).await {
        Ok(Task {
            id, status, output, ..
        }) => {
            println!("✓ Task created: {id}");
            println!("  Status: {status:?}");
            (id, output)
        }
        Err(e) => {
            eprintln!("✗ Failed to send message: {}", e);
            return Ok(());
        }
    };

    // Step 3: Poll for task completion
    println!("\n⏳ Polling for task completion...");
    match client.poll_until_complete(id, 1000, 30).await {
        Ok(Task { status, error, .. }) => {
            println!("✓ Task completed!");
            println!("  Status: {status:?}");

            if let Some(output) = output {
                println!("\n📝 Agent response:");
                for part in &output.parts {
                    match part {
                        MessagePart::Text { text } => {
                            println!("  {text}");
                        }
                        MessagePart::File { file_uri, .. } => {
                            println!("  [File: {file_uri}]");
                        }
                        MessagePart::Data { .. } => {
                            println!("  [Structured data]");
                        }
                    }
                }
            }

            if let Some(TaskError { message, .. }) = error {
                println!("\n⚠️  Task error: {message}");
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to poll task: {e}");
        }
    }

    // Step 4: List all tasks
    println!("\n📚 Listing all tasks...");
    match client.list_all_tasks().await {
        Ok(tasks) => {
            println!("✓ Found {} tasks", tasks.len());
            for (i, Task { id, status, .. }) in tasks.iter().take(5).enumerate() {
                println!("  {}. {id} - {status:?}", i + 1);
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to list tasks: {e}");
        }
    }

    println!("\n✅ Example completed successfully!");

    Ok(())
}
