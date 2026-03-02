//! A2A Protocol v1.0.0 implementation

pub mod client;
pub mod codec;
pub mod layer;
pub mod service;
pub mod types;

pub use client::{A2AClientBuilder, AgentClient, ClientConfig};
pub use codec::{JsonRpcCodec, SseCodec, SseEvent};
pub use layer::{ValidationLayer, ValidationService};
pub use service::A2AProtocolService;
pub use types::{
    A2AResponse, AgentCapabilities, AgentCard, AgentExtension, AgentInterface, AgentProvider,
    AgentSkill, Artifact, ListTasksParams, ListTasksResponse, Message, Operation, Part,
    PushNotificationAuthInfo, PushNotificationConfig, Role, SendMessageConfiguration, Task,
    TaskState, TaskStatus,
};
