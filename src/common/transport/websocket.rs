//! WebSocket transport implementation for A2A protocol

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

use crate::common::error::A2AError;

use super::{Transport, TransportRequest, TransportResponse};

type WsStream = WebSocketStream<MaybeTlsStream<tokio::net::TcpStream>>;
type WsSink = SplitSink<WsStream, Message>;
type WsSource = SplitStream<WsStream>;

struct WebSocketConnection {
    sink: WsSink,
    pending_requests: Arc<RwLock<HashMap<String, mpsc::UnboundedSender<Value>>>>,
}

impl WebSocketConnection {
    async fn new(url: &Url) -> Result<(Self, WsSource), A2AError> {
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

    async fn send_message(&mut self, message: Value) -> Result<(), A2AError> {
        let text = serde_json::to_string(&message)?;
        self.sink
            .send(Message::Text(text))
            .await
            .map_err(|e| A2AError::Transport(format!("WebSocket send failed: {}", e)))?;
        Ok(())
    }

    async fn register_request(&self, id: String, tx: mpsc::UnboundedSender<Value>) {
        let mut pending = self.pending_requests.write().await;
        pending.insert(id, tx);
    }

    async fn handle_response(&self, id: String, result: Value) {
        let mut pending = self.pending_requests.write().await;
        if let Some(tx) = pending.remove(&id) {
            let _ = tx.send(result);
        }
    }
}

/// WebSocket transport for A2A protocol
#[derive(Clone)]
pub struct WebSocketTransport {
    url: Url,
    connection: Arc<Mutex<Option<Arc<Mutex<WebSocketConnection>>>>>,
    message_handler: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl WebSocketTransport {
    pub fn new(url: impl Into<Url>) -> Self {
        Self {
            url: url.into(),
            connection: Arc::new(Mutex::new(None)),
            message_handler: Arc::new(Mutex::new(None)),
        }
    }

    async fn get_connection(&self) -> Result<Arc<Mutex<WebSocketConnection>>, A2AError> {
        let mut conn_guard = self.connection.lock().await;

        if conn_guard.is_none() {
            let (connection, source) = WebSocketConnection::new(&self.url).await?;
            let conn_arc = Arc::new(Mutex::new(connection));
            *conn_guard = Some(conn_arc.clone());
            self.start_message_handler(source, conn_arc.clone()).await;
            Ok(conn_arc)
        } else {
            Ok(conn_guard.as_ref().unwrap().clone())
        }
    }

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
                        if let Ok(jsonrpc) = serde_json::from_str::<Value>(&text) {
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
                    Ok(Message::Close(_)) => break,
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
    pub async fn execute_streaming(
        &self,
        request: TransportRequest,
    ) -> Result<impl Stream<Item = Result<Value, A2AError>>, A2AError> {
        let jsonrpc: Value = serde_json::from_slice(&request.body)?;
        let connection = self.get_connection().await?;

        {
            let mut conn = connection.lock().await;
            conn.send_message(jsonrpc.clone()).await?;
        }

        let (tx, rx) = mpsc::unbounded_channel();
        let request_id = jsonrpc
            .get("id")
            .and_then(|i| i.as_str())
            .unwrap_or("")
            .to_string();

        {
            let conn = connection.lock().await;
            conn.register_request(request_id, tx).await;
        }

        let stream = futures::stream::unfold(rx, |mut rx| async move {
            rx.recv().await.map(|value| (Ok(value), rx))
        });

        Ok(stream)
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
        Poll::Ready(Ok(()))
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        let jsonrpc: Value = serde_json::from_slice(&request.body)?;

        let request_id = jsonrpc
            .get("id")
            .and_then(|i| i.as_str())
            .map(|s| s.to_string())
            .unwrap_or_else(|| Uuid::now_v7().to_string());

        let connection = self.get_connection().await?;

        let (tx, mut rx) = mpsc::unbounded_channel();

        {
            let conn = connection.lock().await;
            conn.register_request(request_id.clone(), tx).await;
        }

        {
            let mut conn = connection.lock().await;
            conn.send_message(jsonrpc).await?;
        }

        let response_value = tokio::time::timeout(std::time::Duration::from_secs(30), rx.recv())
            .await
            .map_err(|_| A2AError::Timeout)?
            .ok_or_else(|| A2AError::Transport("Response channel closed".to_string()))?;

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
}
