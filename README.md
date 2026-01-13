<div align="center"><img src="docs/icon.png" width="150" height="150" alt="Tower A2A Icon" style="border-radius: 50%;">

`tower` middleware layer for the A2A Protocol
---
</div>

This library provides a composable, transport-agnostic implementation of the A2A protocol as Tower middleware. It enables Rust applications to communicate with AI agents using standardized protocol operations, task management, and authentication schemes.

## Why Tower A2A?

By leveraging the `tower::Service` abstraction, this library separates the protocol logic from the transport layer. This allows you to swap HTTP for WebSockets or add standard middleware like `tower-http` (for tracing or compression) directly into your agent communication pipeline.

Easily inject `Retry`, `RateLimit`, or `Timeout` layers using the standard Tower ecosystem. We have a production-ready HTTP client, but allow you to build custom Connector implementations for gRPC or proprietary backends.

```ascii
[ Your App ] -> [ tower-a2a ] -> [ Layer: Auth ] -> [ Layer: Log ] -> [ Transport: HTTP | gRPC | WebSocket ] ( -> [ TCP ])
```

## Features

- **Tower Integration** - Implements Tower's `Service` and `Layer` traits for composable middleware
- **Transport Agnostic** - HTTP transport included, extensible to gRPC, WebSocket, and custom transports
- **Multiple Auth Schemes** - Built-in support for Bearer tokens, API keys, and Basic authentication
- **Type-Safe Protocol** - Strongly-typed message, task, and agent types with serde serialization
- **Task Lifecycle Management** - Full support for task submission, polling, cancellation, and status tracking
- **Agent Discovery** - Automatic agent capability discovery via standard agent-card endpoint
- **Async/Await** - Built on Tokio for high-performance async I/O

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tower-a2a = "0.1.0"
```

## Quick Start

```rust
use tower_a2a::prelude::*;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build client with HTTP transport and authentication
    let mut client = A2AClientBuilder::new("https://agent.example.com")
        .with_http()
        .with_bearer_auth("your-token".to_string())
        .with_timeout(Duration::from_secs(30))
        .build()?;

    // Discover agent capabilities
    let agent_card = client.discover().await?;
    println!("Connected to: {}", agent_card.name);

    // Send a message
    let task = client.send_message(
        Message::user("What is the weather in San Francisco?")
    ).await?;

    // Poll for completion
    let result = client.poll_until_complete(task.id, 1000, 30).await?;
    println!("Task status: {:?}", result.status);

    // Access artifacts from completed task
    for Artifact { artifact_id, .. } in &result.artifacts {
        println!("Artifact: {artifact_id}");
    }

    Ok(())
}
```

## Examples

See [`examples/simple_client.rs`](examples/simple_client.rs) for a complete working example demonstrating agent discovery, message sending, task polling, and task listing.

Run the example with:

```bash
cargo run --example simple_client
```

## 🚧 TODO:

We welcome contributions! Here are areas where help is needed:

### Server Implementation
- [ ] **A2A Server Service** - Implement Tower service to *host* A2A-compliant agents (currently only the client-side is implemented)
- [ ] **Request Router** - Route incoming A2A operations to appropriate task handlers

### Transports
- [ ] **gRPC Transport** - Implement `Transport` trait for gRPC
- [ ] **Custom Transport Guide** - Documentation for implementing custom transports

### Performance
- [ ] **Buffer Pool** - Implement `protocol/buffer_pool.rs` for pre-allocated serialization buffers
- [ ] **Criterion Benchmarks** - Add benchmarks for protocol parsing and serialization (target: <100μs round-trip)

### Streaming
- [ ] **SSE Streaming** - Complete Server-Sent Events support for real-time task updates
- [ ] **WebSocket Bidirectional Streaming** - Full duplex streaming for long-running tasks

### Documentation & Examples
- [ ] **Server Example** - Example showing how to host an A2A-compliant agent
- [ ] **Multi-Agent Example** - Demo of agents communicating with each other
- [ ] **Integration Tests** - Expand test coverage for full layer stack scenarios

Please feel free to start a discussion to let us know what you think, open an issue or submit a pull request. Please be kind and respectful.

## Resources

- **[A2A Protocol Specification](https://a2a-protocol.org/latest/specification/)** - Official protocol documentation
- **[Examples Directory](examples/)** - Complete working examples

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
