//! JSON-RPC 2.0 codec for A2A protocol
//!
//! This codec wraps A2A operations in JSON-RPC 2.0 envelopes for compatibility
//! with agents that use the JSON-RPC protocol binding.

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    codec::Codec,
    protocol::{error::A2AError, operation::A2AOperation},
    service::response::A2AResponse,
};

use super::json::JsonCodec;

/// JSON-RPC 2.0 request envelope
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: Value,
    id: String,
}

/// JSON-RPC 2.0 response envelope
#[derive(Debug, Deserialize)]
#[allow(unused)]
struct JsonRpcResponse {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
    id: Value,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Deserialize)]
#[allow(unused)]
struct JsonRpcError {
    code: i64,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// JSON-RPC 2.0 codec that wraps A2A operations
///
/// This codec implements the JSON-RPC 2.0 protocol binding for A2A.
/// It wraps operations in JSON-RPC request envelopes and unwraps responses.
#[derive(Debug, Clone)]
pub struct JsonRpcCodec {
    /// Inner JSON codec for encoding the params
    inner: JsonCodec,
}

impl JsonRpcCodec {
    /// Create a new JSON-RPC codec
    pub fn new() -> Self {
        Self {
            inner: JsonCodec::new(),
        }
    }

    /// Map an A2A operation to a JSON-RPC method name
    fn operation_to_method(operation: &A2AOperation) -> &'static str {
        match operation {
            A2AOperation::SendMessage { stream, .. } => {
                if *stream {
                    "message/stream"
                } else {
                    "message/send"
                }
            }
            A2AOperation::GetTask { .. } => "task/get",
            A2AOperation::ListTasks { .. } => "task/list",
            A2AOperation::CancelTask { .. } => "task/cancel",
            A2AOperation::DiscoverAgent => "agent/discover",
            A2AOperation::SubscribeTask { .. } => "task/subscribe",
            A2AOperation::RegisterWebhook { .. } => "webhook/register",
        }
    }
}

impl Default for JsonRpcCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Codec for JsonRpcCodec {
    fn encode_request(&self, operation: &A2AOperation) -> Result<Bytes, A2AError> {
        // Encode the operation using the inner JSON codec
        let params_bytes = self.inner.encode_request(operation)?;
        let params: Value = serde_json::from_slice(&params_bytes)?;

        // Wrap in JSON-RPC 2.0 envelope
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            method: Self::operation_to_method(operation).to_string(),
            params,
            id: Uuid::now_v7().to_string(),
        };

        let bytes = serde_json::to_vec(&request)?;
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

        // Parse JSON-RPC response envelope
        let jsonrpc_response: JsonRpcResponse = serde_json::from_slice(body)
            .map_err(|e| A2AError::Protocol(format!("Failed to parse JSON-RPC response: {}", e)))?;

        // Check for JSON-RPC error
        if let Some(error) = jsonrpc_response.error {
            return Err(A2AError::Protocol(format!(
                "JSON-RPC error {}: {}",
                error.code, error.message
            )));
        }

        // Extract result
        let result = jsonrpc_response.result.ok_or_else(|| {
            A2AError::Protocol("JSON-RPC response missing 'result' field".to_string())
        })?;

        // Decode the result using the inner JSON codec
        let result_bytes = serde_json::to_vec(&result)?;
        self.inner.decode_response(&result_bytes, operation)
    }

    fn content_type(&self) -> &str {
        "application/a2a+json"
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::message::Message;

    use super::*;

    #[test]
    fn test_encode_send_message() {
        let codec = JsonRpcCodec::new();
        let message = Message::user("Hello");

        let operation = A2AOperation::SendMessage {
            message,
            stream: false,
            context_id: None,
            task_id: None,
        };

        let bytes = codec.encode_request(&operation).unwrap();
        assert!(!bytes.is_empty());

        // Verify it's valid JSON-RPC
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "message/send");
        assert!(json["params"].is_object());
        assert!(json["id"].is_string());
    }

    #[test]
    fn test_encode_streaming_message() {
        let codec = JsonRpcCodec::new();
        let message = Message::user("Hello");

        let operation = A2AOperation::SendMessage {
            message,
            stream: true,
            context_id: None,
            task_id: None,
        };

        let bytes = codec.encode_request(&operation).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "message/stream");
    }

    #[test]
    fn test_operation_method_mapping() {
        let message = Message::user("test");

        let op = A2AOperation::SendMessage {
            message: message.clone(),
            stream: false,
            context_id: None,
            task_id: None,
        };
        assert_eq!(JsonRpcCodec::operation_to_method(&op), "message/send");

        let op = A2AOperation::SendMessage {
            message,
            stream: true,
            context_id: None,
            task_id: None,
        };
        assert_eq!(JsonRpcCodec::operation_to_method(&op), "message/stream");

        let op = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };
        assert_eq!(JsonRpcCodec::operation_to_method(&op), "task/get");

        let op = A2AOperation::CancelTask {
            task_id: "task-123".to_string(),
        };
        assert_eq!(JsonRpcCodec::operation_to_method(&op), "task/cancel");

        let op = A2AOperation::DiscoverAgent;
        assert_eq!(JsonRpcCodec::operation_to_method(&op), "agent/discover");
    }

    #[test]
    fn test_decode_success_response() {
        let codec = JsonRpcCodec::new();
        let json = r#"{
            "jsonrpc": "2.0",
            "result": {
                "id": "task-123",
                "status": "submitted",
                "input": {
                    "role": "user",
                    "parts": [{"text": "Hello"}]
                },
                "createdAt": "2024-01-01T00:00:00Z"
            },
            "id": "req-123"
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
    fn test_decode_error_response() {
        let codec = JsonRpcCodec::new();
        let json = r#"{
            "jsonrpc": "2.0",
            "error": {
                "code": -32600,
                "message": "Invalid Request"
            },
            "id": "req-123"
        }"#;

        let operation = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };

        let result = codec.decode_response(json.as_bytes(), &operation);
        assert!(result.is_err());

        match result {
            Err(A2AError::Protocol(msg)) => {
                assert!(msg.contains("-32600"));
                assert!(msg.contains("Invalid Request"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_decode_missing_result() {
        let codec = JsonRpcCodec::new();
        let json = r#"{
            "jsonrpc": "2.0",
            "id": "req-123"
        }"#;

        let operation = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };

        let result = codec.decode_response(json.as_bytes(), &operation);
        assert!(result.is_err());

        match result {
            Err(A2AError::Protocol(msg)) => {
                assert!(msg.contains("missing 'result' field"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }

    #[test]
    fn test_content_type() {
        let codec = JsonRpcCodec::new();
        assert_eq!(codec.content_type(), "application/a2a+json");
    }
}
