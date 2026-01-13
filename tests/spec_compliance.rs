//! A2A Protocol Specification Compliance Tests
//!
//! These tests verify that the implementation matches the A2A Protocol specification v1.0+

use serde_json::json;
use tower_a2a::protocol::{
    message::{Message, MessagePart, Role},
    task::{Task, TaskStatus},
    Artifact,
};

#[test]
fn test_role_serialization() {
    // Verify role serializes to lowercase "user" and "agent" per spec
    let user_msg = Message::user("Hello");
    let json = serde_json::to_value(&user_msg).unwrap();
    assert_eq!(json["role"], "user");

    let agent_msg = Message::agent("Hi there");
    let json = serde_json::to_value(&agent_msg).unwrap();
    assert_eq!(json["role"], "agent");
}

#[test]
fn test_message_part_text_serialization() {
    // Verify text part matches spec format: {"text": "content"}
    let part = MessagePart::text("Hello, world!");
    let json = serde_json::to_value(&part).unwrap();

    assert_eq!(json["text"], "Hello, world!");
    assert!(json.get("mimeType").is_none()); // Should not have mimeType
}

#[test]
fn test_message_part_file_serialization() {
    // Verify file part matches spec v1.0+ format with nested structure
    let part = MessagePart::file_with_type(
        "document.pdf",
        "https://example.com/doc.pdf",
        "application/pdf",
    );
    let json = serde_json::to_value(&part).unwrap();

    // Should have nested structure
    assert!(json.get("file").is_some());
    assert_eq!(json["file"]["name"], "document.pdf");
    assert_eq!(json["file"]["fileWithUri"], "https://example.com/doc.pdf");
    assert_eq!(json["file"]["mediaType"], "application/pdf");

    // Should NOT have flat structure
    assert!(json.get("fileUri").is_none());
    assert!(json.get("mimeType").is_none());
}

#[test]
fn test_message_part_file_with_bytes_serialization() {
    // Verify file part with base64 bytes matches spec
    let part = MessagePart::file_with_bytes(
        "image.png",
        "base64encodeddata==",
        Some("image/png".to_string()),
    );
    let json = serde_json::to_value(&part).unwrap();

    assert_eq!(json["file"]["name"], "image.png");
    assert_eq!(json["file"]["fileWithBytes"], "base64encodeddata==");
    assert_eq!(json["file"]["mediaType"], "image/png");
    assert!(json["file"].get("fileWithUri").is_none());
}

#[test]
fn test_message_part_data_serialization() {
    // Verify data part matches spec format: {"data": {...}}
    let data = json!({"key": "value", "count": 42});
    let part = MessagePart::data(data.clone());
    let json = serde_json::to_value(&part).unwrap();

    assert_eq!(json["data"], data);
    // Should NOT have mimeType at same level
    assert!(json.get("mimeType").is_none());
}

#[test]
fn test_message_field_naming() {
    // Verify message fields use camelCase per spec
    let msg = Message::builder()
        .role(Role::User)
        .part(MessagePart::text("Test"))
        .message_id("msg-123")
        .task_id("task-456")
        .context_id("ctx-789")
        .build();

    let json = serde_json::to_value(&msg).unwrap();

    assert_eq!(json["messageId"], "msg-123"); // camelCase
    assert_eq!(json["taskId"], "task-456"); // camelCase
    assert_eq!(json["contextId"], "ctx-789"); // camelCase

    // Should NOT use snake_case
    assert!(json.get("message_id").is_none());
    assert!(json.get("task_id").is_none());
    assert!(json.get("context_id").is_none());
}

#[test]
fn test_task_status_serialization() {
    // Verify task status uses kebab-case per spec
    let msg = Message::user("Test");

    let task = Task::new("task-123", msg.clone()).with_status(TaskStatus::InputRequired);
    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(json["status"], "input-required");

    let task = Task::new("task-124", msg.clone()).with_status(TaskStatus::AuthRequired);
    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(json["status"], "auth-required");

    let task = Task::new("task-125", msg).with_status(TaskStatus::Submitted);
    let json = serde_json::to_value(&task).unwrap();
    assert_eq!(json["status"], "submitted");
}

#[test]
fn test_task_field_naming() {
    // Verify task fields use camelCase per spec
    let msg = Message::user("Test");
    let task = Task::new("task-123", msg).with_context_id("ctx-456");
    let json = serde_json::to_value(&task).unwrap();

    assert!(json.get("createdAt").is_some()); // camelCase
    assert!(json.get("contextId").is_some()); // camelCase

    // Should NOT use snake_case
    assert!(json.get("created_at").is_none());
    assert!(json.get("context_id").is_none());
}

#[test]
fn test_task_artifacts_field() {
    // Verify task has artifacts field per spec
    let msg = Message::user("Test");
    let artifact = Artifact {
        artifact_id: "artifact-1".to_string(),
        name: Some("result".to_string()),
        description: None,
        parts: vec![MessagePart::text("Output")],
        metadata: None,
        extensions: vec![],
    };

    let task = Task::new("task-123", msg).with_artifact(artifact);
    let json = serde_json::to_value(&task).unwrap();

    assert!(json.get("artifacts").is_some());
    assert!(json["artifacts"].is_array());
    assert_eq!(json["artifacts"][0]["artifact_id"], "artifact-1");
}

#[test]
fn test_task_history_field() {
    // Verify task has history field per spec
    let msg = Message::user("Test");
    let history_msg = Message::agent("Response");

    let task = Task::new("task-123", msg).with_history_message(history_msg);
    let json = serde_json::to_value(&task).unwrap();

    assert!(json.get("history").is_some());
    assert!(json["history"].is_array());
    assert_eq!(json["history"][0]["role"], "agent");
}

#[test]
fn test_optional_fields_omitted() {
    // Verify optional fields are omitted when None per spec
    let msg = Message::user("Test");
    let json = serde_json::to_value(&msg).unwrap();

    // These should be omitted when None
    assert!(json.get("messageId").is_none());
    assert!(json.get("taskId").is_none());
    assert!(json.get("contextId").is_none());
    assert!(json.get("metadata").is_none());
    assert!(json.get("extensions").is_none());
}

#[test]
fn test_message_deserialization_from_spec_example() {
    // Test deserializing a message that follows spec format
    let spec_json = json!({
        "role": "user",
        "parts": [
            {"text": "What is the weather?"},
            {
                "file": {
                    "name": "image.jpg",
                    "mediaType": "image/jpeg",
                    "fileWithUri": "https://example.com/image.jpg"
                }
            }
        ],
        "messageId": "msg-123",
        "contextId": "ctx-456"
    });

    let msg: Message = serde_json::from_value(spec_json).unwrap();
    assert_eq!(msg.role, Role::User);
    assert_eq!(msg.parts.len(), 2);
    assert_eq!(msg.message_id, Some("msg-123".to_string()));
    assert_eq!(msg.context_id, Some("ctx-456".to_string()));

    // Verify file part structure
    match &msg.parts[1] {
        MessagePart::File { file } => {
            assert_eq!(file.name, "image.jpg");
            assert_eq!(file.media_type, Some("image/jpeg".to_string()));
            assert_eq!(
                file.file_with_uri,
                Some("https://example.com/image.jpg".to_string())
            );
        }
        _ => panic!("Expected File part"),
    }
}

#[test]
fn test_round_trip_serialization() {
    // Verify messages can be serialized and deserialized without data loss
    let original = Message::builder()
        .role(Role::Agent)
        .part(MessagePart::text("Hello"))
        .part(MessagePart::file("doc.pdf", "https://example.com/doc.pdf"))
        .part(MessagePart::data(json!({"key": "value"})))
        .message_id("msg-123")
        .build();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: Message = serde_json::from_str(&json).unwrap();

    assert_eq!(original, deserialized);
}
