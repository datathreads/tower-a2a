//! A2A Protocol v0.3.0 types

use serde::{Deserialize, Serialize};

use crate::common::security::AgentCardSignature;

/// Role of a message sender
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Agent,
}

/// File with base64-encoded bytes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileWithBytes {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub name: String,
    pub bytes: String,
}

/// File with URI reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileWithUri {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub name: String,
    pub uri: String,
}

/// File content — either inline bytes or URI reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum FileContent {
    Bytes(FileWithBytes),
    Uri(FileWithUri),
}

/// A message part — discriminated union via `kind` field (v0.3.0 spec)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Part {
    Text {
        text: String,
    },
    File {
        file: FileContent,
    },
    Data {
        data: serde_json::Value,
    },
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    pub fn file_uri(name: impl Into<String>, uri: impl Into<String>) -> Self {
        Self::File {
            file: FileContent::Uri(FileWithUri {
                mime_type: None,
                name: name.into(),
                uri: uri.into(),
            }),
        }
    }

    pub fn data(data: serde_json::Value) -> Self {
        Self::Data { data }
    }
}

/// Task lifecycle states (kebab-case per v0.3.0 spec)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum TaskState {
    Queued,
    Running,
    Completed,
    Failed,
    Canceled,
    AuthRequired,
    InputRequired,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled
        )
    }
}

/// A message in the A2A v0.3.0 protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,

    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Map<String, serde_json::Value>>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![Part::text(text)],
            task_id: None,
            context_id: None,
            metadata: None,
        }
    }

    pub fn agent(text: impl Into<String>) -> Self {
        Self {
            role: Role::Agent,
            parts: vec![Part::text(text)],
            task_id: None,
            context_id: None,
            metadata: None,
        }
    }
}

/// Current status of a task (v0.3.0)
///
/// Note: `message` is a `Message` type, not a string.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStatus {
    pub state: TaskState,

    /// Optional message providing context — this is a Message object, not a string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<Message>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<String>,
}

/// Artifact produced by a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artifact {
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub parts: Vec<Part>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
}

fn default_task_kind() -> String {
    "task".to_string()
}

/// A task in the A2A v0.3.0 protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub context_id: String,
    pub status: TaskStatus,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<Vec<Message>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// Fixed value `"task"` per spec
    #[serde(default = "default_task_kind")]
    pub kind: String,
}

impl Task {
    pub fn is_terminal(&self) -> bool {
        self.status.state.is_terminal()
    }
}

/// Push notification authentication info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PushNotificationAuthInfo {
    #[serde(rename = "type")]
    pub auth_type: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Task-scoped push notification config (per spec TaskPushNotificationConfig)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskPushNotificationConfig {
    pub id: String,
    pub task_id: String,
    pub config: PushNotificationConfig,
}

/// Push notification webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PushNotificationConfig {
    pub url: String,

    #[serde(rename = "authenticationInfo", skip_serializing_if = "Option::is_none")]
    pub authentication_info: Option<PushNotificationAuthInfo>,
}

/// Configuration for sending messages (v0.3.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MessageSendConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_tool_use_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<u64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
}

/// Transport protocol enum
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransportProtocol {
    #[serde(rename = "JSONRPC")]
    JsonRpc,
    #[serde(rename = "GRPC")]
    Grpc,
    #[serde(rename = "HTTP+JSON")]
    HttpJson,
}

/// Agent interface (v0.3.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInterface {
    pub url: url::Url,
    pub transport: TransportProtocol,
}

/// Agent provider (v0.3.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProvider {
    pub organization: String,
    pub url: String,
}

/// Agent capabilities (v0.3.0)
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_transition_history: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<AgentExtension>>,
}

/// Agent skill (v0.3.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentSkill {
    pub id: String,
    pub name: String,
    pub description: String,

    #[serde(default)]
    pub tags: Vec<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_modes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_modes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<std::collections::HashMap<String, Vec<String>>>>,
}

/// Agent extension
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentExtension {
    pub uri: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// Agent card (v0.3.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub protocol_version: String,
    pub name: String,
    pub description: String,
    pub url: url::Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub preferred_transport: Option<TransportProtocol>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_interfaces: Option<Vec<AgentInterface>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub icon_url: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<AgentProvider>,

    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,

    pub capabilities: AgentCapabilities,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<std::collections::HashMap<String, crate::common::security::SecurityScheme>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<std::collections::HashMap<String, Vec<String>>>>,

    pub default_input_modes: Vec<String>,
    pub default_output_modes: Vec<String>,
    pub skills: Vec<AgentSkill>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub supports_authenticated_extended_card: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Vec<AgentCardSignature>>,
}

/// A2A v0.3.0 operations
#[derive(Debug, Clone)]
pub enum Operation {
    SendMessage {
        message: Message,
        task_id: Option<String>,
        skill_id: Option<String>,
        configuration: Option<MessageSendConfiguration>,
    },
    StreamMessage {
        message: Message,
        task_id: Option<String>,
        skill_id: Option<String>,
        configuration: Option<MessageSendConfiguration>,
    },
    GetTask {
        id: String,
    },
    CancelTask {
        id: String,
    },
    Resubscribe {
        task_id: String,
    },
    SetPushConfig {
        task_id: String,
        config_id: String,
        config: PushNotificationConfig,
    },
    GetPushConfig {
        task_id: String,
        config_id: String,
    },
    ListPushConfigs {
        task_id: String,
    },
    DeletePushConfig {
        task_id: String,
        config_id: String,
    },
    GetExtendedCard,
    DiscoverAgent,
}

/// Response from an A2A v0.3.0 operation
#[derive(Debug, Clone)]
pub enum A2AResponse {
    Task(Box<Task>),
    Message(Box<Message>),
    TaskList(Vec<Task>),
    AgentCard(Box<AgentCard>),
    PushConfig(Box<TaskPushNotificationConfig>),
    PushConfigList(Vec<TaskPushNotificationConfig>),
    Empty,
}

impl A2AResponse {
    pub fn into_task(self) -> Option<Task> {
        match self {
            A2AResponse::Task(t) => Some(*t),
            _ => None,
        }
    }

    pub fn into_message(self) -> Option<Message> {
        match self {
            A2AResponse::Message(m) => Some(*m),
            _ => None,
        }
    }

    pub fn into_task_list(self) -> Option<Vec<Task>> {
        match self {
            A2AResponse::TaskList(tasks) => Some(tasks),
            _ => None,
        }
    }

    pub fn into_agent_card(self) -> Option<AgentCard> {
        match self {
            A2AResponse::AgentCard(c) => Some(*c),
            _ => None,
        }
    }

    pub fn into_push_config(self) -> Option<TaskPushNotificationConfig> {
        match self {
            A2AResponse::PushConfig(c) => Some(*c),
            _ => None,
        }
    }

    pub fn into_push_config_list(self) -> Option<Vec<TaskPushNotificationConfig>> {
        match self {
            A2AResponse::PushConfigList(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(self, A2AResponse::Empty)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_state_serialization() {
        assert_eq!(
            serde_json::to_string(&TaskState::Queued).unwrap(),
            "\"queued\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Running).unwrap(),
            "\"running\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::AuthRequired).unwrap(),
            "\"auth-required\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Canceled).unwrap(),
            "\"canceled\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::InputRequired).unwrap(),
            "\"input-required\""
        );
    }

    #[test]
    fn test_input_required_not_terminal() {
        assert!(!TaskState::InputRequired.is_terminal());
        assert!(!TaskState::AuthRequired.is_terminal());
    }

    #[test]
    fn test_message_optional_fields() {
        let msg = Message {
            role: Role::User,
            parts: vec![Part::text("Hello")],
            task_id: Some("task-1".into()),
            context_id: Some("ctx-1".into()),
            metadata: None,
        };
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["taskId"], "task-1");
        assert_eq!(json["contextId"], "ctx-1");
        assert!(json.get("metadata").is_none());
    }

    #[test]
    fn test_message_constructors_have_none_fields() {
        let msg = Message::user("Hello");
        assert!(msg.task_id.is_none());
        assert!(msg.context_id.is_none());
        assert!(msg.metadata.is_none());
        let json = serde_json::to_value(&msg).unwrap();
        assert!(json.get("taskId").is_none());
        assert!(json.get("contextId").is_none());
    }

    #[test]
    fn test_part_kind_discriminator() {
        let part = Part::text("Hello");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["kind"], "text");
        assert_eq!(json["text"], "Hello");
    }

    #[test]
    fn test_file_part_serialization() {
        let part = Part::file_uri("doc.pdf", "https://example.com/doc.pdf");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["kind"], "file");
        assert!(json["file"].is_object());
    }

    #[test]
    fn test_task_status_message_is_message_type() {
        // Verify message field is a Message, not a string
        let status = TaskStatus {
            state: TaskState::Running,
            message: Some(Message::agent("Processing...")),
            timestamp: None,
        };
        let json = serde_json::to_value(&status).unwrap();
        // The message field should be a Message object with role and parts
        assert!(json["message"].is_object());
        assert_eq!(json["message"]["role"], "agent");
    }

    #[test]
    fn test_task_kind_field() {
        let task = Task {
            id: "task-1".to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Running,
                message: None,
                timestamp: None,
            },
            history: None,
            artifacts: None,
            metadata: None,
            kind: "task".to_string(),
        };
        let json = serde_json::to_value(&task).unwrap();
        assert_eq!(json["kind"], "task");
    }
}
