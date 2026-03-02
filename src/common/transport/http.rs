//! HTTP transport implementation for A2A protocol

use std::task::{Context, Poll};

use async_trait::async_trait;
use futures::stream::Stream;
use url::Url;

use crate::common::error::A2AError;

use super::{Transport, TransportRequest, TransportResponse};

/// SSE event from a streaming response
#[derive(Debug, Clone)]
pub struct SseEvent {
    pub event_type: Option<String>,
    pub data: String,
    pub id: Option<String>,
}

/// HTTP transport implementation using reqwest
#[derive(Clone, Debug)]
pub struct HttpTransport {
    client: reqwest::Client,
    base_url: Url,
}

impl HttpTransport {
    pub fn new(base_url: Url) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub fn with_client(base_url: Url, client: reqwest::Client) -> Self {
        Self { client, base_url }
    }

    /// Execute a streaming SSE request
    pub async fn execute_streaming(
        &self,
        request: TransportRequest,
    ) -> Result<impl Stream<Item = Result<eventsource_stream::Event, A2AError>>, A2AError> {
        use eventsource_stream::Eventsource;
        use futures::StreamExt;

        let url = format!("{}{}", self.base_url, request.endpoint.trim_start_matches('/'));

        let mut req_builder = match request.method.as_str() {
            "POST" => self.client.post(&url),
            "GET" => self.client.get(&url),
            _ => {
                return Err(A2AError::Transport(format!(
                    "Unsupported HTTP method for streaming: {}",
                    request.method
                )))
            }
        };

        req_builder = req_builder.header("Accept", "text/event-stream");

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        if !request.body.is_empty() {
            req_builder = req_builder.body(request.body);
        }

        let response = req_builder.send().await?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(A2AError::Transport(format!(
                "HTTP streaming request failed with status {}: {}",
                status, body
            )));
        }

        let stream = response
            .bytes_stream()
            .eventsource()
            .map(|r| r.map_err(|e| A2AError::Transport(e.to_string())));

        Ok(stream)
    }
}

#[async_trait]
impl Transport for HttpTransport {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), A2AError>> {
        Poll::Ready(Ok(()))
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        let url = format!("{}{}", self.base_url, request.endpoint.trim_start_matches('/'));

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

        for (key, value) in request.headers {
            req_builder = req_builder.header(key, value);
        }

        if !request.body.is_empty() {
            req_builder = req_builder.body(request.body);
        }

        let response = req_builder.send().await?;

        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();

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
