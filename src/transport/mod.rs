//! Transport abstraction layer for A2A protocol

pub mod http;
#[cfg(test)]
pub mod mock;
pub mod websocket;

use std::{
    collections::HashMap,
    task::{Context, Poll},
};

pub use http::HttpTransport;
use reqwest::Url;
pub use websocket::WebSocketTransport;

use async_trait::async_trait;
use bytes::Bytes;

/// Protocol-agnostic transport request
#[derive(Debug, Clone)]
pub struct TransportRequest {
    /// The endpoint path (e.g., "/tasks", "/tasks/123")
    pub endpoint: String,

    /// HTTP method or equivalent operation (e.g., "POST", "GET", "PUT", "DELETE")
    pub method: String,

    /// Headers or metadata for the request
    pub headers: HashMap<String, String>,

    /// Request body as bytes
    pub body: Bytes,
}

impl TransportRequest {
    /// Create a new transport request
    pub fn new(endpoint: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            method: method.into(),
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    /// Add a header to the request
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the request body
    pub fn body(mut self, body: Bytes) -> Self {
        self.body = body;
        self
    }
}

/// Protocol-agnostic transport response
#[derive(Debug)]
pub struct TransportResponse {
    /// Status code (e.g., HTTP status code)
    pub status: u16,

    /// Response headers or metadata
    pub headers: HashMap<String, String>,

    /// Response body as bytes
    pub body: Bytes,
}

impl TransportResponse {
    /// Create a new transport response
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    /// Add a header to the response
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the response body
    pub fn body(mut self, body: Bytes) -> Self {
        self.body = body;
        self
    }

    /// Check if the response indicates success (2xx status code)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if the response indicates a client error (4xx status code)
    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    /// Check if the response indicates a server error (5xx status code)
    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }
}

/// Core transport trait for executing protocol-agnostic requests
///
/// This trait abstracts over different network protocols (HTTP, gRPC, WebSocket, etc.)
/// allowing the A2A protocol layer to work with any underlying transport.
#[async_trait]
pub trait Transport: Clone + Send + Sync + 'static {
    /// Check if the transport is ready to accept requests
    ///
    /// This is used by Tower's Service trait to implement backpressure
    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), crate::protocol::error::A2AError>>;

    /// Execute a transport request asynchronously
    ///
    /// # Arguments
    ///
    /// * `request` - The protocol-agnostic request to execute
    ///
    /// # Returns
    ///
    /// A protocol-agnostic response or an error
    async fn execute(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, crate::protocol::error::A2AError>;

    /// Get the base URL or identifier for this transport
    ///
    /// For HTTP transports, this would be the base URL (e.g., "<https://agent.example.com>")
    /// For in-memory transports, this might be "memory://"
    fn base_url(&self) -> &Url;

    /// Check if this transport supports streaming responses
    fn supports_streaming(&self) -> bool {
        false
    }
}

/// Implement Transport for `Box<dyn Transport>`
#[async_trait]
impl<T: Transport> Transport for Box<T> {
    fn poll_ready(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<(), crate::protocol::error::A2AError>> {
        (**self).poll_ready(cx)
    }

    async fn execute(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, crate::protocol::error::A2AError> {
        (**self).execute(request).await
    }

    fn base_url(&self) -> &Url {
        (**self).base_url()
    }

    fn supports_streaming(&self) -> bool {
        (**self).supports_streaming()
    }
}
