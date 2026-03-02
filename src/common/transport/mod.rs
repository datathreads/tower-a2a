//! Transport abstraction layer for A2A protocol

pub mod http;
pub mod mock;
pub mod websocket;

pub use http::HttpTransport;
pub use websocket::WebSocketTransport;

use std::{
    collections::HashMap,
    task::{Context, Poll},
};

use async_trait::async_trait;
use bytes::Bytes;
use url::Url;

use crate::common::error::A2AError;

/// Protocol-agnostic transport request
#[derive(Debug, Clone)]
pub struct TransportRequest {
    /// The endpoint path (e.g., "/", "/.well-known/agent-card.json")
    pub endpoint: String,

    /// HTTP method ("POST", "GET", "DELETE")
    pub method: String,

    /// Request headers
    pub headers: HashMap<String, String>,

    /// Request body as bytes
    pub body: Bytes,
}

impl TransportRequest {
    pub fn new(endpoint: impl Into<String>, method: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            method: method.into(),
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: Bytes) -> Self {
        self.body = body;
        self
    }
}

/// Protocol-agnostic transport response
#[derive(Debug)]
pub struct TransportResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Bytes,
}

impl TransportResponse {
    pub fn new(status: u16) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Bytes::new(),
        }
    }

    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    pub fn body(mut self, body: Bytes) -> Self {
        self.body = body;
        self
    }

    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    pub fn is_client_error(&self) -> bool {
        self.status >= 400 && self.status < 500
    }

    pub fn is_server_error(&self) -> bool {
        self.status >= 500 && self.status < 600
    }
}

/// Core transport trait for executing protocol-agnostic requests
#[async_trait]
pub trait Transport: Clone + Send + Sync + 'static {
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), A2AError>>;

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError>;

    fn base_url(&self) -> &Url;

    fn supports_streaming(&self) -> bool {
        false
    }
}

#[async_trait]
impl<T: Transport> Transport for Box<T> {
    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), A2AError>> {
        (**self).poll_ready(cx)
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        (**self).execute(request).await
    }

    fn base_url(&self) -> &Url {
        (**self).base_url()
    }

    fn supports_streaming(&self) -> bool {
        (**self).supports_streaming()
    }
}
