//! v0.3.0 codec — JSON-RPC 2.0 with message/tasks method names

use bytes::Bytes;
use eventsource_stream::Eventsource;
use futures::stream::{Stream, StreamExt};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::common::error::A2AError;

use super::types::{AgentCard, A2AResponse, Message, Operation, Task, TaskPushNotificationConfig};

/// Codec trait for v0.3.0 operations
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

/// v0.3.0 JSON-RPC codec (slash-notation method names)
#[derive(Debug, Clone, Default)]
pub struct JsonRpcCodec;

impl JsonRpcCodec {
    pub fn new() -> Self {
        Self
    }

    fn method_for(op: &Operation) -> &'static str {
        match op {
            Operation::SendMessage { .. } => "message/send",
            Operation::StreamMessage { .. } => "message/stream",
            Operation::GetTask { .. } => "tasks/get",
            Operation::CancelTask { .. } => "tasks/cancel",
            Operation::Resubscribe { .. } => "tasks/resubscribe",
            Operation::SetPushConfig { .. } => "tasks/pushNotificationConfig/set",
            Operation::GetPushConfig { .. } => "tasks/pushNotificationConfig/get",
            Operation::ListPushConfigs { .. } => "tasks/pushNotificationConfig/list",
            Operation::DeletePushConfig { .. } => "tasks/pushNotificationConfig/delete",
            Operation::GetExtendedCard => "agent/getAuthenticatedExtendedCard",
            Operation::DiscoverAgent => "",
        }
    }

    fn params_for(op: &Operation) -> Result<Value, A2AError> {
        let params = match op {
            Operation::SendMessage {
                message,
                task_id,
                skill_id,
                configuration,
            }
            | Operation::StreamMessage {
                message,
                task_id,
                skill_id,
                configuration,
            } => {
                let mut p = json!({ "message": message });
                if let Some(tid) = task_id {
                    p["taskId"] = json!(tid);
                }
                if let Some(sid) = skill_id {
                    p["skillId"] = json!(sid);
                }
                if let Some(cfg) = configuration {
                    p["configuration"] = json!(cfg);
                }
                p
            }
            Operation::GetTask { id } => json!({ "id": id }),
            Operation::CancelTask { id } => json!({ "id": id }),
            Operation::Resubscribe { task_id } => json!({ "id": task_id }),
            Operation::SetPushConfig { task_id, config_id, config } => {
                json!({ "id": config_id, "taskId": task_id, "config": config })
            }
            Operation::GetPushConfig { task_id, config_id } => {
                json!({ "taskId": task_id, "configId": config_id })
            }
            Operation::ListPushConfigs { task_id } => json!({ "taskId": task_id }),
            Operation::DeletePushConfig { task_id, config_id } => {
                json!({ "taskId": task_id, "configId": config_id })
            }
            Operation::GetExtendedCard | Operation::DiscoverAgent => json!({}),
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

        // Agent card discovery returns plain JSON
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

        match operation {
            Operation::SendMessage { .. } => {
                match result.get("kind").and_then(|k| k.as_str()) {
                    Some("task") => {
                        let task: Task = serde_json::from_value(result)?;
                        Ok(A2AResponse::Task(Box::new(task)))
                    }
                    _ => {
                        let msg: Message = serde_json::from_value(result)?;
                        Ok(A2AResponse::Message(Box::new(msg)))
                    }
                }
            }
            Operation::GetTask { .. } | Operation::CancelTask { .. } => {
                let task: Task = serde_json::from_value(result)?;
                Ok(A2AResponse::Task(Box::new(task)))
            }
            Operation::GetExtendedCard => {
                let card: AgentCard = serde_json::from_value(result)?;
                Ok(A2AResponse::AgentCard(Box::new(card)))
            }
            Operation::SetPushConfig { .. } | Operation::GetPushConfig { .. } => {
                let cfg: TaskPushNotificationConfig = serde_json::from_value(result)?;
                Ok(A2AResponse::PushConfig(Box::new(cfg)))
            }
            Operation::ListPushConfigs { .. } => {
                let cfgs: Vec<TaskPushNotificationConfig> = serde_json::from_value(result)?;
                Ok(A2AResponse::PushConfigList(cfgs))
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
    pub kind: String,
    pub payload: Value,
    pub final_event: bool,
}

impl SseEvent {
    pub fn is_terminal(&self) -> bool {
        if self.final_event {
            return true;
        }
        // TaskStatusUpdateEvent nests state under taskStatus
        if let Some(state) = self.payload
            .get("taskStatus")
            .and_then(|ts| ts.get("state"))
            .and_then(|s| s.as_str())
        {
            return matches!(state, "completed" | "failed" | "canceled");
        }
        false
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
            Ok(event) => Self::parse_sse_data(&event.data),
            Err(e) => Err(A2AError::Transport(format!("SSE stream error: {}", e))),
        })
    }

    /// Parse an already-decoded SSE event stream (e.g., from `HttpTransport::execute_streaming`).
    pub fn parse_event_stream<S>(
        &self,
        event_stream: S,
    ) -> impl Stream<Item = Result<SseEvent, A2AError>>
    where
        S: Stream<Item = Result<eventsource_stream::Event, A2AError>> + Send + 'static,
    {
        event_stream.map(|result| match result {
            Ok(event) => Self::parse_sse_data(&event.data),
            Err(e) => Err(e),
        })
    }

    fn parse_sse_data(data: &str) -> Result<SseEvent, A2AError> {
        let jsonrpc: Value = serde_json::from_str(data)
            .map_err(|e| A2AError::Protocol(format!("Failed to parse SSE event: {}", e)))?;

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
}

#[cfg(test)]
mod tests {
    use crate::v0_3::types::Message;

    use super::*;

    #[test]
    fn test_encode_send_message() {
        let codec = JsonRpcCodec;
        let op = Operation::SendMessage {
            message: Message::user("Hello"),
            task_id: None,
            skill_id: None,
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["jsonrpc"], "2.0");
        assert_eq!(json["method"], "message/send");
    }

    #[test]
    fn test_encode_stream_message() {
        let codec = JsonRpcCodec;
        let op = Operation::StreamMessage {
            message: Message::user("Hello"),
            task_id: None,
            skill_id: None,
            configuration: None,
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "message/stream");
    }

    #[test]
    fn test_encode_get_task() {
        let codec = JsonRpcCodec;
        let op = Operation::GetTask {
            id: "task-123".to_string(),
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "tasks/get");
        assert_eq!(json["params"]["id"], "task-123");
    }

    #[test]
    fn test_encode_push_config_set() {
        let codec = JsonRpcCodec;
        let op = Operation::SetPushConfig {
            task_id: "task-1".to_string(),
            config_id: "cfg-1".to_string(),
            config: crate::v0_3::types::PushNotificationConfig {
                url: "https://example.com/webhook".to_string(),
                authentication_info: None,
            },
        };
        let bytes = codec.encode_request(&op).unwrap();
        let json: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(json["method"], "tasks/pushNotificationConfig/set");
    }
}
