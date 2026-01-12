//! High-level client API for A2A protocol

pub mod builder;
pub mod agent;
pub mod config;

pub use builder::A2AClientBuilder;
pub use agent::AgentClient;
pub use config::ClientConfig;
