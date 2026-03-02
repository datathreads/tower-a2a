use std::time::Duration;

use tower_a2a::prelude::*;

// Configuration - update these to match your agent
const AGENT_URL: &str = "https://your-agent-url";
const AUTH_TOKEN: &str = "your-auth-token";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging
    tracing_subscriber::fmt::init();

    println!("Tower-A2A Simple Client Example\n");

    // Build the A2A client with HTTP transport and bearer authentication
    let url = AGENT_URL.parse().unwrap();
    let mut client = A2AClientBuilder::new_http(url)
        .with_bearer_auth(AUTH_TOKEN.to_string())
        .with_timeout(Duration::from_secs(30))
        .build()?;

    println!("Client configured for: {AGENT_URL}\n");

    // Step 1: Discover agent capabilities
    println!("Discovering agent capabilities...");
    match client.discover().await {
        Ok(AgentCard {
            name,
            description,
            capabilities:
                AgentCapabilities {
                    streaming,
                    push_notifications,
                    ..
                },
            ..
        }) => {
            println!("Connected to: {name}");
            println!("  Description: {description}");
            println!("  Capabilities:");
            println!("    - Streaming: {streaming:?}");
            println!("    - Push notifications: {push_notifications:?}");
            println!();
        }
        Err(e) => {
            eprintln!(
                "Failed to discover agent: {e}\n\
                Note: Make sure AGENT_URL points to a running A2A agent"
            );
            return Ok(());
        }
    }

    // Step 2: Send a message to the agent
    println!("Sending message to agent...");
    let message = Message::user("What is the weather like in San Francisco?");

    let (id, artifacts) = match client.send_message(message).await {
        Ok(Task {
            id,
            status,
            artifacts,
            ..
        }) => {
            println!("Task created: {id}");
            println!("  State: {:?}", status.state);
            (id, artifacts)
        }
        Err(e) => {
            eprintln!("Failed to send message: {e}");
            return Ok(());
        }
    };

    // Step 3: Poll for task completion
    println!("\nPolling for task completion...");
    match client.poll_until_complete(id, 1000, 30).await {
        Ok(Task { status, .. }) => {
            println!("Task completed!");
            println!("  State: {:?}", status.state);

            if let Some(artifacts) = artifacts {
                if !artifacts.is_empty() {
                    println!("\nAgent artifacts:");
                    for Artifact {
                        artifact_id, parts, ..
                    } in &artifacts
                    {
                        println!("  Artifact: {artifact_id}");
                        for part in parts {
                            if let Some(text) = &part.text {
                                println!("    {text}");
                            } else if let Some(uri) = &part.file_uri {
                                println!("    [File: {uri}]");
                            } else if part.data.is_some() {
                                println!("    [Structured data]");
                            }
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to poll task: {e}");
        }
    }

    // Step 4: List all tasks
    println!("\nListing all tasks...");
    match client.list_all_tasks().await {
        Ok(tasks) => {
            println!("Found {} tasks", tasks.len());
            for (i, Task { id, status, .. }) in tasks.iter().take(5).enumerate() {
                println!("  {}. {id} - {:?}", i + 1, status.state);
            }
        }
        Err(e) => {
            eprintln!("Failed to list tasks: {e}");
        }
    }

    println!("\nExample completed successfully!");

    Ok(())
}
