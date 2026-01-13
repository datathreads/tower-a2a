//! JSON codec for HTTP+JSON binding

use bytes::Bytes;
use serde_json::json;

use crate::{
    codec::Codec,
    protocol::{
        agent::AgentCard,
        error::A2AError,
        operation::A2AOperation,
        task::{Task, TaskListResponse},
    },
    service::response::A2AResponse,
};

/// JSON codec for the HTTP+JSON protocol binding
#[derive(Debug, Clone, Default)]
pub struct JsonCodec;

impl JsonCodec {
    /// Create a new JSON codec
    pub fn new() -> Self {
        Self
    }
}

impl Codec for JsonCodec {
    fn encode_request(&self, operation: &A2AOperation) -> Result<Bytes, A2AError> {
        let json = match operation {
            A2AOperation::SendMessage {
                message,
                stream,
                context_id,
                task_id,
            } => {
                let mut obj = json!({
                    "message": message,
                    "stream": stream,
                });

                if let Some(ctx_id) = context_id {
                    obj["contextId"] = json!(ctx_id);
                }
                if let Some(t_id) = task_id {
                    obj["taskId"] = json!(t_id);
                }

                obj
            }
            A2AOperation::CancelTask { task_id } => {
                json!({
                    "taskId": task_id,
                })
            }
            A2AOperation::RegisterWebhook { url, events, auth } => {
                let mut obj = json!({
                    "url": url,
                    "events": events,
                });

                if let Some(auth_str) = auth {
                    obj["auth"] = json!(auth_str);
                }

                obj
            }
            // GET requests typically don't have bodies
            _ => json!({}),
        };

        let bytes = serde_json::to_vec(&json)?;
        Ok(Bytes::from(bytes))
    }

    fn decode_response(
        &self,
        body: &[u8],
        operation: &A2AOperation,
    ) -> Result<A2AResponse, A2AError> {
        // Empty responses
        if body.is_empty() {
            return Ok(A2AResponse::Empty);
        }

        match operation {
            A2AOperation::SendMessage { .. } | A2AOperation::GetTask { .. } => {
                let task: Task = serde_json::from_slice(body)?;
                Ok(A2AResponse::Task(Box::new(task)))
            }
            A2AOperation::ListTasks { .. } => {
                let list: TaskListResponse = serde_json::from_slice(body)?;
                Ok(A2AResponse::TaskList {
                    tasks: list.tasks,
                    total: list.total,
                    next_token: list.next_token,
                })
            }
            A2AOperation::DiscoverAgent => {
                let card: AgentCard = serde_json::from_slice(body)?;
                Ok(A2AResponse::AgentCard(Box::new(card)))
            }
            A2AOperation::CancelTask { .. } => {
                // Cancel typically returns the updated task
                let task: Task = serde_json::from_slice(body)?;
                Ok(A2AResponse::Task(Box::new(task)))
            }
            A2AOperation::SubscribeTask { .. } => {
                // Streaming responses handled separately
                Ok(A2AResponse::Empty)
            }
            A2AOperation::RegisterWebhook { .. } => Ok(A2AResponse::Empty),
        }
    }

    fn content_type(&self) -> &str {
        "application/a2a+json"
    }
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::*;
    use crate::protocol::message::Message;

    #[test]
    fn test_encode_send_message() {
        let codec = JsonCodec;
        let message = Message::user("Hello");

        let operation = A2AOperation::SendMessage {
            message,
            stream: false,
            context_id: None,
            task_id: None,
        };

        let bytes = codec.encode_request(&operation).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's valid JSON
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert!(json["message"].is_object());
        assert_eq!(json["stream"], false);
    }

    #[test]
    fn test_decode_task_response() {
        let codec = JsonCodec;
        let json = r#"{
            "id": "task-123",
            "status": "submitted",
            "input": {
                "role": "user",
                "parts": [{"text": "Hello"}]
            },
            "createdAt": "2024-01-01T00:00:00Z"
        }"#;

        let operation = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };

        let response = codec.decode_response(json.as_bytes(), &operation).unwrap();

        match response {
            A2AResponse::Task(task) => {
                assert_eq!(task.id, "task-123");
            }
            _ => panic!("Expected Task response"),
        }
    }

    #[test]
    fn test_content_type() {
        let codec = JsonCodec;
        assert_eq!(codec.content_type(), "application/a2a+json");
    }
}
