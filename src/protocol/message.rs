//! A2A message types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A message in the A2A protocol
///
/// Messages are the primary unit of communication between agents.
/// Each message has a role (user or assistant), one or more parts (text, file, or data),
/// and optional metadata and extensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// Role of the message sender
    pub role: Role,

    /// Message content parts (at least one required)
    pub parts: Vec<MessagePart>,

    /// Optional message identifier
    #[serde(rename = "messageId", skip_serializing_if = "Option::is_none")]
    pub message_id: Option<String>,

    /// Optional task identifier (for associating message with a task)
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,

    /// Optional context identifier (for multi-turn conversations)
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    /// Optional metadata for the message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, Value>>,

    /// Optional extensions indicating additional protocol features
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, Value>>,
}

impl Message {
    /// Create a new message with text content
    pub fn new(role: Role, text: impl Into<String>) -> Self {
        Self {
            role,
            parts: vec![MessagePart::Text { text: text.into() }],
            message_id: None,
            task_id: None,
            context_id: None,
            metadata: None,
            extensions: None,
        }
    }

    /// Create a user message with text content
    pub fn user(text: impl Into<String>) -> Self {
        Self::new(Role::User, text)
    }

    /// Create an agent message with text content
    pub fn agent(text: impl Into<String>) -> Self {
        Self::new(Role::Agent, text)
    }

    /// Create a new message builder
    pub fn builder() -> MessageBuilder {
        MessageBuilder::new()
    }

    /// Add a metadata field to the message
    pub fn with_metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add an extension to the message
    pub fn with_extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add a message part
    pub fn with_part(mut self, part: MessagePart) -> Self {
        self.parts.push(part);
        self
    }
}

/// Builder for constructing Message instances
#[derive(Debug, Default)]
pub struct MessageBuilder {
    role: Option<Role>,
    parts: Vec<MessagePart>,
    message_id: Option<String>,
    task_id: Option<String>,
    context_id: Option<String>,
    metadata: Option<HashMap<String, Value>>,
    extensions: Option<HashMap<String, Value>>,
}

impl MessageBuilder {
    /// Create a new message builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the role of the message
    pub fn role(mut self, role: Role) -> Self {
        self.role = Some(role);
        self
    }

    /// Set the message parts
    pub fn parts(mut self, parts: Vec<MessagePart>) -> Self {
        self.parts = parts;
        self
    }

    /// Add a single part to the message
    pub fn part(mut self, part: MessagePart) -> Self {
        self.parts.push(part);
        self
    }

    /// Set the message ID
    pub fn message_id(mut self, id: impl Into<String>) -> Self {
        self.message_id = Some(id.into());
        self
    }

    /// Set the task ID
    pub fn task_id(mut self, id: impl Into<String>) -> Self {
        self.task_id = Some(id.into());
        self
    }

    /// Set the context ID
    pub fn context_id(mut self, id: impl Into<String>) -> Self {
        self.context_id = Some(id.into());
        self
    }

    /// Add a metadata field
    pub fn metadata(mut self, key: impl Into<String>, value: Value) -> Self {
        self.metadata
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Add an extension
    pub fn extension(mut self, key: impl Into<String>, value: Value) -> Self {
        self.extensions
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value);
        self
    }

    /// Build the message
    ///
    /// # Panics
    ///
    /// Panics if role is not set or if parts are empty
    pub fn build(self) -> Message {
        let role = self.role.expect("Message role is required");
        assert!(
            !self.parts.is_empty(),
            "Message must have at least one part"
        );

        Message {
            role,
            parts: self.parts,
            message_id: self.message_id,
            task_id: self.task_id,
            context_id: self.context_id,
            metadata: self.metadata,
            extensions: self.extensions,
        }
    }
}

/// Role of a message sender
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    /// Message from a user
    User,

    /// Message from an AI agent
    Agent,
}

/// File content for file parts
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
    /// MIME type of the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub media_type: Option<String>,

    /// Name of the file
    pub name: String,

    /// URI reference to the file
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_with_uri: Option<String>,

    /// Base64-encoded file content
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_with_bytes: Option<String>,
}

/// A part of a message
///
/// According to the A2A spec: "A Part MUST contain exactly one of the following: text, file, data"
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessagePart {
    /// Text content
    Text {
        /// The text content
        text: String,
    },

    /// File reference
    File {
        /// File content (nested structure per spec v1.0+)
        file: FileContent,
    },

    /// Structured data
    Data {
        /// The structured data
        data: Value,
    },
}

impl MessagePart {
    /// Create a text part
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }

    /// Create a file part with URI reference
    pub fn file(name: impl Into<String>, file_uri: impl Into<String>) -> Self {
        Self::File {
            file: FileContent {
                media_type: None,
                name: name.into(),
                file_with_uri: Some(file_uri.into()),
                file_with_bytes: None,
            },
        }
    }

    /// Create a file part with URI and media type
    pub fn file_with_type(
        name: impl Into<String>,
        file_uri: impl Into<String>,
        media_type: impl Into<String>,
    ) -> Self {
        Self::File {
            file: FileContent {
                media_type: Some(media_type.into()),
                name: name.into(),
                file_with_uri: Some(file_uri.into()),
                file_with_bytes: None,
            },
        }
    }

    /// Create a file part with base64-encoded bytes
    pub fn file_with_bytes(
        name: impl Into<String>,
        file_bytes: impl Into<String>,
        media_type: Option<String>,
    ) -> Self {
        Self::File {
            file: FileContent {
                media_type,
                name: name.into(),
                file_with_uri: None,
                file_with_bytes: Some(file_bytes.into()),
            },
        }
    }

    /// Create a data part
    pub fn data(data: Value) -> Self {
        Self::Data { data }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::user("Hello, agent!");
        assert_eq!(msg.role, Role::User);
        assert_eq!(msg.parts.len(), 1);

        match &msg.parts[0] {
            MessagePart::Text { text } => assert_eq!(text, "Hello, agent!"),
            _ => panic!("Expected text part"),
        }
    }

    #[test]
    fn test_message_with_metadata() {
        let msg = Message::user("Test")
            .with_metadata("key", json!("value"))
            .with_extension("ext", json!({"enabled": true}));

        assert!(msg.metadata.is_some());
        assert!(msg.extensions.is_some());
    }

    #[test]
    fn test_message_serialization() {
        let msg = Message::user("Test message");
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"role\":\"user\""));
        assert!(json.contains("\"text\":\"Test message\""));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn test_message_part_types() {
        let text = MessagePart::text("Hello");
        let file = MessagePart::file("myfile.txt", "file://path/to/file");
        let data = MessagePart::data(json!({"key": "value"}));

        assert!(matches!(text, MessagePart::Text { .. }));
        assert!(matches!(file, MessagePart::File { .. }));
        assert!(matches!(data, MessagePart::Data { .. }));
    }

    #[test]
    fn test_message_builder() {
        let msg = Message::builder()
            .role(Role::Agent)
            .parts(vec![MessagePart::text("Hello")])
            .message_id("msg-123")
            .task_id("task-456")
            .context_id("ctx-789")
            .build();

        assert_eq!(msg.role, Role::Agent);
        assert_eq!(msg.parts.len(), 1);
        assert_eq!(msg.message_id, Some("msg-123".to_string()));
        assert_eq!(msg.task_id, Some("task-456".to_string()));
        assert_eq!(msg.context_id, Some("ctx-789".to_string()));
    }

    #[test]
    fn test_message_builder_with_part() {
        let msg = Message::builder()
            .role(Role::Agent)
            .part(MessagePart::text("First"))
            .part(MessagePart::text("Second"))
            .build();

        assert_eq!(msg.parts.len(), 2);
    }

    #[test]
    #[should_panic(expected = "Message role is required")]
    fn test_message_builder_missing_role() {
        Message::builder()
            .parts(vec![MessagePart::text("Hello")])
            .build();
    }

    #[test]
    #[should_panic(expected = "Message must have at least one part")]
    fn test_message_builder_no_parts() {
        Message::builder().role(Role::User).build();
    }

    #[test]
    fn test_message_serialization_with_ids() {
        let msg = Message::builder()
            .role(Role::User)
            .parts(vec![MessagePart::text("Test")])
            .message_id("msg-123")
            .task_id("task-456")
            .build();

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"messageId\":\"msg-123\""));
        assert!(json.contains("\"taskId\":\"task-456\""));

        let deserialized: Message = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }
}
