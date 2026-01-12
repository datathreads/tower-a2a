//! WebSocket transport implementation for A2A protocol
//!
//! This transport provides bidirectional, stateful communication with A2A agents
//! over WebSockets, supporting both request/response and streaming patterns.

use std::{
    collections::HashMap,
    sync::Arc,
    task::{Context, Poll},
};

use async_trait::async_trait;
use futures::{
    stream::{SplitSink, SplitStream, Stream, StreamExt},
    SinkExt,
};
use serde_json::Value;
use tokio::sync::{mpsc, Mutex, RwLock};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message, MaybeTlsStream, WebSocketStream,
};
use url::Url;
use uuid::Uuid;

use crate::{
    codec::sse::SseEvent,
    protocol::error::A2AError,
    transport::{Transport, TransportRequest, TransportResponse},
};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

/// WebSocket connection state
struct WebSocketConnection {
    /// Outgoing message sink
    sink: WsSink,

    /// Response channels for pending requests
    pending_requests: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Value>>>>,
}

impl WebSocketConnection {
    /// Create a new WebSocket connection
    async fn new(url: &Url) -> Result<(Self, WsSource), A2AError> {
        // Connect to WebSocket
        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| A2AError::Transport(format!("WebSocket connection failed: {}", e)))?;

        let (sink, source) = ws_stream.split();

        let connection = Self {
            sink,
            pending_requests: Arc::new(RwLock::new(HashMap::new())),
        };

        Ok((connection, source))
    }

    /// Send a JSON-RPC message
    async fn send_message(&mut self, message: Value) -> Result<(), A2AError> {
        let text = serde_json::to_string(&message)?;
        self.sink
            .send(Message::Text(text))
            .await
            .map_err(|e| A2AError::Transport(format!("WebSocket send failed: {}", e)))?;
        Ok(())
    }

    /// Register a pending request
    async fn register_request(&self, id: String, tx: mpsc::UnboundedSender<Value>) {
        let mut pending = self.pending_requests.write().await;
        pending.insert(id, tx);
    }

    /// Handle incoming message
    async fn handle_response(&self, id: String, result: Value) {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(result);
        }
    }
}

/// WebSocket transport for A2A protocol
///
/// This transport maintains a persistent WebSocket connection and supports
/// concurrent requests, streaming responses, and task subscriptions.
#[derive(Clone)]
pub struct WebSocketTransport {
    url: Url,
    connection: Arc<Mutex<Option<Arc<Mutex<WebSocketConnection>>>>>,
    message_handler: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl WebSocketTransport {
    /// Create a new WebSocket transport
    ///
    /// # Arguments
    ///
    /// * `url` - The WebSocket URL (e.g., "ws://agent.example.com" or "wss://agent.example.com")
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            connection: Arc::new(Mutex::new(None)),
            message_handler: Arc::new(Mutex::new(None)),
        }
    }

    /// Get or establish a WebSocket connection
    async fn get_connection(&self) -> Result<Arc<Mutex<WebSocketConnection>>, A2AError> {
        let mut conn_guard = self.connection.lock().await;

        if conn_guard.is_none() {
            // Establish new connection
            let (connection, source) = WebSocketConnection::new(&self.url).await?;
            let conn_arc = Arc::new(Mutex::new(connection));
            *conn_guard = Some(conn_arc.clone());

            // Start message handler task
            self.start_message_handler(source, conn_arc.clone()).await;

            Ok(conn_arc)
        } else {
            Ok(conn_guard.as_ref().unwrap().clone())
        }
    }

    /// Start the background task that handles incoming WebSocket messages
    async fn start_message_handler(
        &self,
        mut source: WsSource,
        connection: Arc<Mutex<WebSocketConnection>>,
    ) {
        let mut handler_guard = self.message_handler.lock().await;

        let handle = tokio::spawn(async move {
            while let Some(result) = source.next().await {
                match result {
                    Ok(Message::Text(text)) => {
                        // Parse JSON-RPC response
                        if let Ok(jsonrpc) = serde_json::from_str::<Value>(&text) {
                            // Extract id and result
                            if let (Some(id), Some(result)) = (
                                jsonrpc
                                    .get("id")
                                    .and_then(|i| i.as_str())
                                    .map(|s| s.to_string()),
                                jsonrpc.get("result"),
                            ) {
                                let conn = connection.lock().await;
                                conn.handle_response(id, result.clone()).await;
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        // Connection closed
                        break;
                    }
                    Err(e) => {
                        tracing::error!("WebSocket receive error: {}", e);
                        break;
                    }
                    _ => {}
                }
            }
        });

        *handler_guard = Some(handle);
    }

    /// Execute a streaming request (for task subscription)
    ///
    /// This method sends a WebSocket message and returns a stream of events.
    pub async fn execute_streaming(
        &self,
        request: TransportRequest,
    ) -> Result<impl Stream<Item = Result<SseEvent, A2AError>>, A2AError> {
        // Parse request body as JSON-RPC
        let jsonrpc: Value = serde_json::from_slice(&request.body)?;

        // Get connection
        let connection = self.get_connection().await?;

        // Send message
        {
            let mut conn = connection.lock().await;
            conn.send_message(jsonrpc.clone()).await?;
        }

        // Create a channel for streaming events
        let (tx, rx) = mpsc::unbounded_channel();

        // Extract request ID
        let request_id = jsonrpc
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();

        // Register the streaming response handler
        {
            let conn = connection.lock().await;
            conn.register_request(request_id, tx).await;
        }

        // Convert receiver into a stream
        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|value| {
                // Convert Value to SseEvent
                let event = Self::value_to_sse_event(value);
                (event, rx)
            })
        });

        Ok(stream)
    }

    /// Convert a JSON value to an SSE event
    fn value_to_sse_event(value: Value) -> Result<SseEvent, A2AError> {
        let kind = value
            .get("kind")
            .and_then(|k| k.as_str())
            .unwrap_or("event")
            .to_string();

        let final_event = value
            .get("final")
            .and_then(|f| f.as_bool())
            .unwrap_or(false);

        Ok(SseEvent {
            kind,
            payload: value,
            final_event,
        })
    }
}

impl std::fmt::Debug for WebSocketTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebSocketTransport")
            .field("url", &self.url)
            .finish()
    }
}

#[async_trait]
impl Transport for WebSocketTransport {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), A2AError>> {
        // WebSocket is always ready (buffered)
        Poll::Ready(Ok(()))
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        // Parse request body as JSON-RPC
        let jsonrpc: Value = serde_json::from_slice(&request.body)?;

        // Generate request ID if not present
        let request_id = jsonrpc
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        // Get connection
        let connection = self.get_connection().await?;

        // Create channel for response
        let (tx, mut rx) = mpsc::unbounded_channel();

        // Register request
        {
            let conn = connection.lock().await;
            conn.register_request(request_id.clone(), tx).await;
        }

        // Send message
        {
            let mut conn = connection.lock().await;
            conn.send_message(jsonrpc).await?;
        }

        // Wait for response (with timeout)
        let response_value = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv())
            .await
            .map_err(|_| A2AError::Timeout)?
            .ok_or_else(|| A2AError::Transport("Response channel closed".to_string()))?;

        // Convert response to TransportResponse
        let body = serde_json::to_vec(&response_value)?;

        Ok(TransportResponse {
            status: 200,
            headers: HashMap::new(),
            body: body.into(),
        })
    }

    fn base_url(&self) -> &Url {
        &self.url
    }

    fn supports_streaming(&self) -> bool {
        true
    }

    fn clone_box(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_websocket_transport_creation() {
        let transport = WebSocketTransport::new(Url::parse("ws://example.com").unwrap());
        assert_eq!(transport.base_url().as_str(), "ws://example.com/");
        assert!(transport.supports_streaming());
    }

    #[test]
    fn test_value_to_sse_event() {
        let value = serde_json::json!({
            "kind": "status-update",
            "state": "running",
            "final": false
        });

        let event = WebSocketTransport::value_to_sse_event(value).unwrap();
        assert_eq!(event.kind, "status-update");
        assert!(!event.final_event);
    }

    #[test]
    fn test_value_to_sse_event_final() {
        let value = serde_json::json!({
            "kind": "artifact-update",
            "final": true
        });

        let event = WebSocketTransport::value_to_sse_event(value).unwrap();
        assert_eq!(event.kind, "artifact-update");
        assert!(event.final_event);
    }
}
