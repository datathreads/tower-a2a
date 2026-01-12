//! # Tower A2A
//!
//! A Tower-based implementation of the Agent2Agent (A2A) protocol.
//!
//! This library provides a composable, transport-agnostic implementation of the A2A protocol
//! using Tower's Service and Layer abstractions. It supports multiple transport protocols
//! (HTTP, gRPC, WebSocket) through a unified interface.
//!
//! ## Features
//!
//! - **Transport Agnostic**: Works with HTTP, gRPC, WebSocket, or custom transports
//! - **Composable Middleware**: Auth, retry, timeout, validation as Tower layers
//! - **Type Safe**: Compile-time guarantees for protocol operations
//! - **Async**: Built on tokio for high performance
//!
//! ## Example
//!
//! ```rust,no_run
//! use tower_a2a::prelude::*;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let url = "https://agent.example.com".parse().unwrap();
//!     let mut client = A2AClientBuilder::new(url)
//!         .with_http()
//!         .with_bearer_auth("token123".to_string())
//!         .with_timeout(Duration::from_secs(30))
//!         .build()?;
//!
//!     let agent_card = client.discover().await?;
//!     println!("Connected to: {}", agent_card.name);
//!
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod codec;
pub mod layer;
pub mod protocol;
pub mod service;
pub mod transport;

/// Prelude module for convenient imports
pub mod prelude {
    pub use crate::{
        client::{A2AClientBuilder, AgentClient},
        protocol::error::A2AError,
        protocol::{A2AOperation, AgentCard, Message, MessagePart, Role, Task, TaskStatus},
    };
}
