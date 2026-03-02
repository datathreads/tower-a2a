//! # Tower A2A
//!
//! A Tower-based implementation of the Agent2Agent (A2A) protocol.
//!
//! This crate provides composable, transport-agnostic implementations of the A2A protocol
//! using Tower's Service and Layer abstractions.
//!
//! ## Features
//!
//! - **Transport Agnostic**: Works with HTTP, gRPC, WebSocket, or custom transports
//! - **Composable Middleware**: Auth, retry, timeout, validation as Tower layers
//! - **Type Safe**: Compile-time guarantees for protocol operations
//! - **Async**: Built on tokio for high performance
//!
//! - `v1` (default) — A2A Protocol v1.0.0 types and client (TaskState: CREATED, WORKING, …)
//! - `v0_3` — A2A Protocol v0.3.0 types and client (TaskState: queued, running, …)
//!
//! Enable both features when you need to communicate with agents on different versions:
//! ```toml
//! [dependencies]
//! tower-a2a = { version = "1", features = ["v1", "v0_3"] }
//! ```
//!
//! ## Example (v1, default)
//!
//! ```rust,no_run
//! use tower_a2a::prelude::*;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let url = "https://agent.example.com".parse().unwrap();
//!     let mut client = A2AClientBuilder::new_http(url)
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

pub mod common;

#[cfg(feature = "v1")]
pub mod v1;

#[cfg(feature = "v0_3")]
pub mod v0_3;

/// v1 types
#[cfg(feature = "v1")]
pub use v1::*;

/// v0_3 types
#[cfg(all(feature = "v0_3", not(feature = "v1")))]
pub use v0_3::*;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::common::{A2AError, A2AResult, AuthCredentials, AuthLayer, RequestContext};

    // v1 (takes priority when both features enabled)
    #[cfg(feature = "v1")]
    pub use crate::v1::{
        A2AClientBuilder, A2AProtocolService, A2AResponse, AgentCapabilities, AgentCard,
        AgentClient, AgentInterface, AgentProvider, AgentSkill, Artifact, JsonRpcCodec,
        ListTasksParams, Message, Operation, Part, Role, SendMessageConfiguration, Task, TaskState,
        TaskStatus, ValidationLayer,
    };

    // v0_3 in prelude only when v1 is disnabled
    #[cfg(all(feature = "v0_3", not(feature = "v1")))]
    pub use crate::v0_3::{
        A2AClientBuilder, A2AProtocolService, A2AResponse, AgentCapabilities, AgentCard,
        AgentClient, AgentInterface, AgentProvider, AgentSkill, Artifact, JsonRpcCodec, Message,
        MessageSendConfiguration, Operation, Part, Role, Task, TaskState, TaskStatus,
        ValidationLayer,
    };
}
