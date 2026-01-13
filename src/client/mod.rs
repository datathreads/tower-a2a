//! High-level client API for A2A protocol

pub mod agent;
pub mod builder;
pub mod config;

pub use agent::AgentClient;
pub use builder::A2AClientBuilder;
pub use config::ClientConfig;
