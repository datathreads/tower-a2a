//! HTTP transport implementation for A2A protocol

use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::stream::Stream;
use url::Url;

use crate::{
    codec::{sse::SseEvent, SseCodec},
    protocol::error::A2AError,
};

use super::{Transport, TransportRequest, TransportResponse};

/// HTTP transport implementation using reqwest
///
/// This transport implements the HTTP+JSON binding of the A2A protocol.
#[derive(Clone, Debug)]
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: Url,
}

impl HttpTransport {
    /// Create a new HTTP transport
    ///
    /// # Arguments
    ///
    /// * `base_url` - The base URL of the agent (e.g., "<https://agent.example.com>")
    pub fn new(base_url: Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    /// Create a new HTTP transport with a custom reqwest client
    pub fn with_client(base_url: Url, client: reqwest::Client) -> Self {
        Self { client, base_url }
    }

    /// Execute a streaming request (Server-Sent Events)
    ///
    /// This method is used for streaming A2A operations that return multiple events
    /// over time (e.g., message/stream RPC method).
    ///
    /// # Arguments
    ///
    /// * `request` - The transport request to execute
    ///
    /// # Returns
    ///
    /// A stream of SSE events or an error
    pub async fn execute_streaming(
        &self,
        request: TransportRequest,
    ) -> Result<impl Stream<Item = Result<SseEvent, A2AError>>, A2AError> {
        let url = format!("{}{}", self.base_url, request.endpoint);

        let mut req_builder = match request.method.as_str() {
            "POST" => self.client.post(&url),
            "GET" => self.client.get(&url),
            "PUT" => self.client.put(&url),
            _ => {
                return Err(A2AError::Transport(format!(
                    "Unsupported HTTP method for streaming: {}",
                    request.method
                )))
            }
        };

        // Add Accept header for SSE
        req_builder = req_builder.header("Accept", "text/event-stream");

        // Add other headers
        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add body if not empty
        if !request.body.is_empty() {
            req_builder = req_builder.body(request.body);
        }

        // Execute the request
        let response = req_builder.send().await?;

        // Check status
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(A2AError::Transport(format!(
                "HTTP streaming request failed with status {}: {}",
                status, body
            )));
        }

        // Get byte stream
        let byte_stream = response.bytes_stream();

        // Parse SSE events
        let sse_codec = SseCodec::new();
        Ok(sse_codec.parse_stream(byte_stream))
    }
}

#[async_trait]
impl Transport for HttpTransport {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), A2AError>> {
        // HTTP client is always ready
        Poll::Ready(Ok(()))
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        let url = format!("{}{}", self.base_url, request.endpoint);

        let mut req_builder = match request.method.as_str() {
            "POST" => self.client.post(&url),
            "GET" => self.client.get(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            _ => {
                return Err(A2AError::Transport(format!(
                    "Unsupported HTTP method: {}",
                    request.method
                )))
            }
        };

        // Add headers
        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        // Add body if not empty
        if !request.body.is_empty() {
            req_builder = req_builder.body(request.body);
        }

        // Execute the request
        let response = req_builder.send().await?;

        // Extract status and headers
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

        // Extract body
        let body = response.bytes().await?;

        Ok(TransportResponse {
            status,
            headers,
            body,
        })
    }

    fn base_url(&self) -> &Url {
        &self.base_url
    }

    fn supports_streaming(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_transport_creation() {
        let transport = HttpTransport::new(Url::parse("https://example.com").unwrap());
        assert_eq!(transport.base_url().as_str(), "https://example.com/");
        assert!(transport.supports_streaming());
    }
}
