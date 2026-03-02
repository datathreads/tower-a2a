//! v1 codec — JSON-RPC 2.0 with A2A. method prefix

use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::common::error::A2AError;

use super::types::{AgentCard, A2AResponse, ListTasksResponse, Message, Operation, Task};

/// Codec trait for v1 operations
pub trait Codec: Send + Sync {
    fn encode_request(&self, operation: &Operation) -> Result<Bytes, A2AError>;
    fn decode_response(&self, body: &[u8], operation: &Operation) -> Result<A2AResponse, A2AError>;
    fn content_type(&self) -> &str;
}

/// JSON-RPC 2.0 response envelope
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(unused)]
    jsonrpc: String,
    result: Option<Value>,
    error: Option<JsonRpcError>,
    #[allow(unused)]
    id: Value,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

/// v1 JSON-RPC codec (method names use "A2A." prefix)
#[derive(Debug, Clone, Default)]
pub struct JsonRpcCodec;

impl JsonRpcCodec {
    pub fn new() -> Self {
        Self
    }

    fn method_for(op: &Operation) -> &'static str {
        match op {
            Operation::SendMessage { .. } => "A2A.SendMessage",
            Operation::SendStreamingMessage { .. } => "A2A.SendStreamingMessage",
            Operation::GetTask { .. } => "A2A.GetTask",
            Operation::ListTasks(_) => "A2A.ListTasks",
            Operation::CancelTask { .. } => "A2A.CancelTask",
            Operation::SubscribeToTask { .. } => "A2A.SubscribeToTask",
            Operation::CreatePushConfig { .. } => "A2A.CreateTaskPushNotificationConfig",
            Operation::GetPushConfig { .. } => "A2A.GetTaskPushNotificationConfig",
            Operation::ListPushConfigs { .. } => "A2A.ListTaskPushNotificationConfigs",
            Operation::DeletePushConfig { .. } => "A2A.DeleteTaskPushNotificationConfig",
            Operation::GetExtendedAgentCard => "A2A.GetExtendedAgentCard",
            Operation::DiscoverAgent => "",
        }
    }

    fn params_for(op: &Operation) -> Result<Value, A2AError> {
        let params = match op {
            Operation::SendMessage {
                message,
                configuration,
            }
            | Operation::SendStreamingMessage {
                message,
                configuration,
            } => {
                let mut p = json!({ "message": message });
                if let Some(cfg) = configuration {
                    p["configuration"] = json!(cfg);
                }
                p
            }
            Operation::GetTask { id, history_length } => {
                let mut p = json!({ "taskId": id });
                if let Some(hl) = history_length {
                    p["historyLength"] = json!(hl);
                }
                p
            }
            Operation::ListTasks(params) => json!(params),
            Operation::CancelTask { id } => json!({ "taskId": id }),
            Operation::SubscribeToTask { id } => json!({ "taskId": id }),
            Operation::CreatePushConfig { task_id, config } => {
                json!({ "taskId": task_id, "config": config })
            }
            Operation::GetPushConfig { task_id, config_id } => {
                json!({ "taskId": task_id, "id": config_id })
            }
            Operation::ListPushConfigs { task_id } => json!({ "taskId": task_id }),
            Operation::DeletePushConfig { task_id, config_id } => {
                json!({ "taskId": task_id, "id": config_id })
            }
            Operation::GetExtendedAgentCard | Operation::DiscoverAgent => json!({}),
        };
        Ok(params)
    }
}

impl Codec for JsonRpcCodec {
    fn encode_request(&self, operation: &Operation) -> Result<Bytes, A2AError> {
        let params = Self::params_for(operation)?;
        let method = Self::method_for(operation);

        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": Uuid::now_v7().to_string(),
        });

        Ok(Bytes::from(serde_json::to_vec(&body)?))
    }

    fn decode_response(&self, body: &[u8], operation: &Operation) -> Result<A2AResponse, A2AError> {
        if body.is_empty() {
            return Ok(A2AResponse::Empty);
        }

        // Agent card discovery returns plain JSON (not JSON-RPC)
        if matches!(operation, Operation::DiscoverAgent) {
            let card: AgentCard = serde_json::from_slice(body)?;
            return Ok(A2AResponse::AgentCard(Box::new(card)));
        }

        let resp: JsonRpcResponse = serde_json::from_slice(body).map_err(|e| {
            A2AError::Protocol(format!("Failed to parse JSON-RPC response: {}", e))
        })?;

        if let Some(err) = resp.error {
            return Err(A2AError::Protocol(format!(
                "JSON-RPC error {}: {}",
                err.code, err.message
            )));
        }

        let result = resp.result.ok_or_else(|| {
            A2AError::Protocol("JSON-RPC response missing 'result' field".to_string())
        })?;

        let result_bytes = serde_json::to_vec(&result)?;

        match operation {
            Operation::SendMessage { .. } => {
                if let Ok(task) = serde_json::from_slice::<Task>(&result_bytes) {
                    Ok(A2AResponse::Task(Box::new(task)))
                } else {
                    let msg: Message = serde_json::from_slice(&result_bytes)?;
                    Ok(A2AResponse::Message(Box::new(msg)))
                }
            }
            Operation::GetTask { .. } | Operation::CancelTask { .. } => {
                let task: Task = serde_json::from_slice(&result_bytes)?;
                Ok(A2AResponse::Task(Box::new(task)))
            }
            Operation::ListTasks(_) => {
                let list: ListTasksResponse = serde_json::from_slice(&result_bytes)?;
                Ok(A2AResponse::TaskList {
                    tasks: list.tasks,
                    next_page_token: list.next_page_token,
                })
            }
            Operation::GetExtendedAgentCard => {
                let card: AgentCard = serde_json::from_slice(&result_bytes)?;
                Ok(A2AResponse::AgentCard(Box::new(card)))
            }
            _ => Ok(A2AResponse::Empty),
        }
    }

    fn content_type(&self) -> &str {
        "application/json"
    }
}

/// SSE event from streaming response
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// Event kind (e.g., "status-update", "artifact-update")
    pub kind: String,
    pub payload: Value,
    pub final_event: bool,
}

impl SseEvent {
    pub fn is_terminal(&self) -> bool {
        if self.final_event {
            return true;
        }
        if let Some(state) = self.payload.get("state").and_then(|s| s.as_str()) {
            matches!(state, "COMPLETED" | "FAILED" | "CANCELED" | "REJECTED")
        } else {
            false
        }
    }
}

/// SSE codec for parsing streaming responses
#[derive(Debug, Clone, Default)]
pub struct SseCodec;

impl SseCodec {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_stream<S>(&self, byte_stream: S) -> impl Stream<Item = Result<SseEvent, A2AError>>
    where
        S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    {
        byte_stream.eventsource().map(|result| match result {
            Ok(event) => {
                let jsonrpc: Value =
                    serde_json::from_str(&event.data).map_err(|e| {
                        A2AError::Protocol(format!("Failed to parse SSE event: {}", e))
                    })?;

                if let Some(error) = jsonrpc.get("error") {
                    let msg = error
                        .get("message")
                        .and_then(|m| m.as_str())
                        .unwrap_or("Unknown error");
                    return Err(A2AError::Protocol(format!("SSE stream error: {}", msg)));
                }

                let result = jsonrpc.get("result").ok_or_else(|| {
                    A2AError::Protocol("SSE event missing 'result' field".to_string())
                })?;

                let final_event = result
                    .get("final")
                    .and_then(|f| f.as_bool())
                    .unwrap_or(false);

                let kind = result
                    .get("kind")
                    .and_then(|k| k.as_str())
                    .unwrap_or("event")
                    .to_string();

                Ok(SseEvent {
                    kind,
                    payload: result.clone(),
                    final_event,
                })
            }
            Err(e) => Err(A2AError::Transport(format!("SSE stream error: {}", e))),
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::v1::types::Message;

    use super::*;

    #[test]
    fn test_encode_send_message() {
        let codec = JsonRpcCodec;
        let op = Operation::SendMessage {
            message: Message::user("Hello"),
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "A2A.SendMessage");
        assert!(json["params"]["message"].is_object());
    }

    #[test]
    fn test_encode_streaming_message() {
        let codec = JsonRpcCodec;
        let op = Operation::SendStreamingMessage {
            message: Message::user("Hello"),
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.SendStreamingMessage");
    }

    #[test]
    fn test_encode_get_task() {
        let codec = JsonRpcCodec;
        let op = Operation::GetTask {
            id: "task-123".to_string(),
            history_length: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "A2A.GetTask");
        assert_eq!(json["params"]["taskId"], "task-123");
    }

    #[test]
    fn test_decode_error_response() {
        let codec = JsonRpcCodec;
        let json = r#"{"jsonrpc":"2.0","error":{"code":-32600,"message":"Invalid Request"},"id":"1"}"#;
        let op = Operation::GetTask {
            id: "task-1".to_string(),
            history_length: None,
        };
        let result = codec.decode_response(json.as_bytes(), &op);
        assert!(result.is_err());
        match result.unwrap_err() {
            A2AError::Protocol(msg) => assert!(msg.contains("-32600")),
            _ => panic!("Expected Protocol error"),
        }
    }
}
