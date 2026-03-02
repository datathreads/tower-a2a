//! A2A Protocol v1.0.0 types

use serde::{Deserialize, Serialize};

use crate::common::security::AgentCardSignature;

/// Role of a message sender
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Agent,
}

/// A message part — flat struct (proto3 oneof style)
///
/// Exactly one of `text`, `data`, or `file_uri` should be populated.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_uri: Option<String>,
}

impl Part {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: Some(text.into()),
            mime_type: None,
            data: None,
            file_uri: None,
        }
    }

    pub fn data(data: serde_json::Value) -> Self {
        Self {
            text: None,
            mime_type: None,
            data: Some(data),
            file_uri: None,
        }
    }

    pub fn file_uri(uri: impl Into<String>, mime_type: Option<String>) -> Self {
        Self {
            text: None,
            mime_type,
            data: None,
            file_uri: Some(uri.into()),
        }
    }
}

/// Task lifecycle states (SCREAMING_SNAKE_CASE per v1.0.0 spec)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskState {
    Created,
    Working,
    Completed,
    Failed,
    Canceled,
    Rejected,
    InputRequired,
    AuthRequired,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Failed | TaskState::Canceled | TaskState::Rejected
        )
    }

    pub fn requires_action(&self) -> bool {
        matches!(self, TaskState::InputRequired | TaskState::AuthRequired)
    }
}

/// Current status of a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TaskStatus {
    pub state: TaskState,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

/// A message in the A2A v1 protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub reference_task_ids: Option<Vec<String>>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            parts: vec![Part::text(text)],
            context_id: None,
            task_id: None,
            message_id: None,
            reference_task_ids: None,
        }
    }

    pub fn agent(text: impl Into<String>) -> Self {
        Self {
            role: Role::Agent,
            parts: vec![Part::text(text)],
            context_id: None,
            task_id: None,
            message_id: None,
            reference_task_ids: None,
        }
    }
}

/// Artifact produced by a task
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Artifact {
    pub artifact_id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    pub parts: Vec<Part>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}

/// A task in the A2A v1 protocol
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Task {
    pub id: String,
    pub context_id: String,
    pub status: TaskStatus,

    /// Convenience duplicate of `status.state` required by spec
    pub state: TaskState,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<Vec<Artifact>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_time: Option<chrono::DateTime<chrono::Utc>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_time: Option<chrono::DateTime<chrono::Utc>>,
}

impl Task {
    pub fn is_terminal(&self) -> bool {
        self.status.state.is_terminal()
    }
}

/// Push notification authentication info
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PushNotificationAuthInfo {
    pub scheme: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub header_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
}

/// Push notification webhook configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PushNotificationConfig {
    pub url: url::Url,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<PushNotificationAuthInfo>,
}

/// Configuration for sending messages
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SendMessageConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accepted_output_modes: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notification_config: Option<PushNotificationConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocking: Option<bool>,
}

/// Parameters for listing tasks
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<TaskState>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_token: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history_length: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_timestamp_after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_artifacts: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant: Option<String>,
}

/// Agent capabilities
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentCapabilities {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub streaming: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub push_notifications: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extended_agent_card: Option<bool>,
}

/// Agent skill
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

/// Agent interface endpoint
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentInterface {
    #[serde(rename = "type")]
    pub interface_type: String,

    pub uri: url::Url,
}

/// Agent provider
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentProvider {
    pub id: String,
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Agent card (v1.0.0)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentCard {
    pub id: String,
    pub name: String,
    pub description: String,
    pub provider: AgentProvider,
    pub capabilities: AgentCapabilities,
    pub skills: Vec<AgentSkill>,
    pub interfaces: Vec<AgentInterface>,
    pub version: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_schemes: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub security: Option<Vec<serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<AgentExtension>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<AgentCardSignature>,
}

/// Response from a list tasks operation
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListTasksResponse {
    pub tasks: Vec<Task>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// A2A v1 operations
#[derive(Debug, Clone)]
pub enum Operation {
    SendMessage {
        message: Message,
        configuration: Option<SendMessageConfiguration>,
    },
    SendStreamingMessage {
        message: Message,
        configuration: Option<SendMessageConfiguration>,
    },
    GetTask {
        id: String,
        history_length: Option<u32>,
    },
    ListTasks(ListTasksParams),
    CancelTask {
        id: String,
    },
    SubscribeToTask {
        id: String,
    },
    CreatePushConfig {
        task_id: String,
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
    GetExtendedAgentCard,
    DiscoverAgent,
}

/// Response from an A2A v1 operation
#[derive(Debug, Clone)]
pub enum A2AResponse {
    Task(Box<Task>),
    Message(Box<Message>),
    TaskList {
        tasks: Vec<Task>,
        next_page_token: Option<String>,
    },
    AgentCard(Box<AgentCard>),
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
            A2AResponse::TaskList { tasks, .. } => Some(tasks),
            _ => None,
        }
    }

    pub fn into_agent_card(self) -> Option<AgentCard> {
        match self {
            A2AResponse::AgentCard(c) => Some(*c),
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
            serde_json::to_string(&TaskState::Created).unwrap(),
            "\"CREATED\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::Working).unwrap(),
            "\"WORKING\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::InputRequired).unwrap(),
            "\"INPUT_REQUIRED\""
        );
        assert_eq!(
            serde_json::to_string(&TaskState::AuthRequired).unwrap(),
            "\"AUTH_REQUIRED\""
        );
    }

    #[test]
    fn test_part_serialization() {
        let part = Part::text("Hello");
        let json = serde_json::to_value(&part).unwrap();
        assert_eq!(json["text"], "Hello");
        assert!(json.get("mimeType").is_none());
        assert!(json.get("data").is_none());
        assert!(json.get("fileUri").is_none());
    }

    #[test]
    fn test_message_user() {
        let msg = Message::user("Hello");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.parts.len(), 1);
        let json = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["role"], "user");
    }

    #[test]
    fn test_role_serialization() {
        let json = serde_json::to_string(&Role::User).unwrap();
        assert_eq!(json, "\"user\"");
        let json = serde_json::to_string(&Role::Agent).unwrap();
        assert_eq!(json, "\"agent\"");
    }
}
