//! Server-Sent Events (SSE) codec for streaming A2A responses
//!
//! This codec handles parsing SSE event streams that contain JSON-RPC 2.0 responses.

use eventsource_stream::Eventsource;
use futures::stream::{Stream, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::protocol::error::A2AError;

/// SSE streaming event containing A2A protocol data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SseEvent {
    /// Event kind (e.g., "artifact-update", "status-update")
    pub kind: String,

    /// Event payload
    pub payload: Value,

    /// Whether this is the final event in the stream
    #[serde(default)]
    pub final_event: bool,
}

impl SseEvent {
    /// Check if this event represents a terminal state
    pub fn is_terminal(&self) -> bool {
        if self.final_event {
            return true;
        }

        // Check for terminal states in the payload
        if let Some(state) = self.payload.get("state").and_then(|s| s.as_str()) {
            matches!(state, "completed" | "failed" | "canceled" | "rejected")
        } else {
            false
        }
    }

    /// Check if this event represents an error state
    pub fn is_error(&self) -> bool {
        if let Some(state) = self.payload.get("state").and_then(|s| s.as_str()) {
            matches!(state, "failed" | "canceled" | "rejected")
        } else {
            false
        }
    }
}

/// SSE codec for parsing streaming responses
#[derive(Debug, Clone, Default)]
pub struct SseCodec;

impl SseCodec {
    /// Create a new SSE codec
    pub fn new() -> Self {
        Self
    }

    /// Parse an SSE byte stream into a stream of events
    ///
    /// This method takes a byte stream (typically from reqwest) and parses it
    /// into individual SSE events containing JSON-RPC responses.
    pub fn parse_stream<S>(&self, byte_stream: S) -> impl Stream<Item = Result<SseEvent, A2AError>>
    where
        S: Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    {
        byte_stream.eventsource().map(|result| {
            match result {
                Ok(event) => {
                    // Parse the event data as JSON-RPC response
                    let jsonrpc: Value = serde_json::from_str(&event.data).map_err(|e| {
                        A2AError::Protocol(format!("Failed to parse SSE event data: {}", e))
                    })?;

                    // Check for JSON-RPC error
                    if let Some(error) = jsonrpc.get("error") {
                        let error_msg = error
                            .get("message")
                            .and_then(|m| m.as_str())
                            .unwrap_or("Unknown error");
                        return Err(A2AError::Protocol(format!(
                            "SSE stream error: {}",
                            error_msg
                        )));
                    }

                    // Extract result from JSON-RPC response
                    let result = jsonrpc.get("result").ok_or_else(|| {
                        A2AError::Protocol("SSE event missing 'result' field".to_string())
                    })?;

                    // Determine if this is a final event
                    let final_event = result
                        .get("final")
                        .and_then(|f| f.as_bool())
                        .unwrap_or(false);

                    // Extract event kind
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
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use serde_json::json;

    use super::*;

    #[test]
    fn test_sse_event_is_terminal() {
        let event = SseEvent {
            kind: "status-update".to_string(),
            payload: json!({
                "state": "completed"
            }),
            final_event: false,
        };
        assert!(event.is_terminal());

        let event = SseEvent {
            kind: "artifact-update".to_string(),
            payload: json!({}),
            final_event: true,
        };
        assert!(event.is_terminal());

        let event = SseEvent {
            kind: "status-update".to_string(),
            payload: json!({
                "state": "running"
            }),
            final_event: false,
        };
        assert!(!event.is_terminal());
    }

    #[test]
    fn test_sse_event_is_error() {
        let event = SseEvent {
            kind: "status-update".to_string(),
            payload: json!({
                "state": "failed"
            }),
            final_event: false,
        };
        assert!(event.is_error());

        let event = SseEvent {
            kind: "status-update".to_string(),
            payload: json!({
                "state": "completed"
            }),
            final_event: false,
        };
        assert!(!event.is_error());
    }

    #[tokio::test]
    async fn test_parse_sse_stream() {
        use futures::pin_mut;

        let codec = SseCodec;

        // Create a mock byte stream with SSE events
        let sse_data = "data: {\"jsonrpc\":\"2.0\",\"result\":{\"kind\":\"status-update\",\"state\":\"running\"},\"id\":\"1\"}\n\n\
                        data: {\"jsonrpc\":\"2.0\",\"result\":{\"kind\":\"artifact-update\",\"final\":true},\"id\":\"2\"}\n\n";

        let byte_stream = futures::stream::once(async move {
            Ok::<bytes::Bytes, reqwest::Error>(bytes::Bytes::from(sse_data))
        });

        let event_stream = codec.parse_stream(byte_stream);
        pin_mut!(event_stream);

        // First event
        let event1 = event_stream.next().await.unwrap().unwrap();
        assert_eq!(event1.kind, "status-update");
        assert!(!event1.final_event);

        // Second event
        let event2 = event_stream.next().await.unwrap().unwrap();
        assert_eq!(event2.kind, "artifact-update");
        assert!(event2.final_event);
    }

    #[tokio::test]
    async fn test_parse_sse_error() {
        use futures::pin_mut;

        let codec = SseCodec;

        let sse_data = "data: {\"jsonrpc\":\"2.0\",\"error\":{\"code\":-32600,\"message\":\"Invalid Request\"},\"id\":\"1\"}\n\n";

        let byte_stream = futures::stream::once(async move {
            Ok::<bytes::Bytes, reqwest::Error>(bytes::Bytes::from(sse_data))
        });

        let event_stream = codec.parse_stream(byte_stream);
        pin_mut!(event_stream);

        let result = event_stream.next().await.unwrap();
        assert!(result.is_err());

        match result {
            Err(A2AError::Protocol(msg)) => {
                assert!(msg.contains("Invalid Request"));
            }
            _ => panic!("Expected Protocol error"),
        }
    }
}
