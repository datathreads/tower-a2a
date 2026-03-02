//! A2A Protocol v0.3.0 implementation

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
    AgentSkill, Artifact, FileContent, FileWithBytes, FileWithUri, Message, MessageSendConfiguration,
    Operation, Part, PushNotificationAuthInfo, PushNotificationConfig, Role, Task,
    TaskPushNotificationConfig, TaskState, TaskStatus, TransportProtocol,
};
