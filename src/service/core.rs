//! Core A2A protocol service implementation

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tower_service::Service;

use crate::{
    codec::Codec,
    protocol::{error::A2AError, operation::A2AOperation},
    service::{A2ARequest, A2AResponse},
    transport::{Transport, TransportRequest},
};

/// Core A2A protocol service that wraps a transport
///
/// This service implements the Tower `Service` trait and provides the core logic
/// for executing A2A operations over any transport (HTTP, gRPC, WebSocket, etc.)
pub struct A2AProtocolService<T> {
    transport: T,
    codec: Arc<dyn Codec>,
}

impl<T> A2AProtocolService<T>
where
    T: Transport,
{
    /// Create a new A2A protocol service
    ///
    /// # Arguments
    ///
    /// * `transport` - The underlying transport implementation
    /// * `codec` - The codec for serialization/deserialization
    pub fn new(transport: T, codec: Arc<dyn Codec>) -> Self {
        Self { transport, codec }
    }

    /// Build a transport request from an A2A operation
    fn build_transport_request(
        req: &A2ARequest,
        codec: &dyn Codec,
    ) -> Result<TransportRequest, A2AError> {
        let endpoint = req.operation.endpoint();
        let method = req.operation.method();

        let mut transport_req = TransportRequest::new(endpoint, method);

        // Add required A2A protocol headers
        transport_req = transport_req.header("Content-Type", codec.content_type());
        transport_req = transport_req.header("Accept", codec.content_type());
        transport_req = transport_req.header("A2A-Version", "1.0");

        // Add authentication headers if present
        if let Some(auth) = &req.context.auth {
            let (header, value) = auth.to_header();
            transport_req = transport_req.header(header, value);
        }

        // Add custom metadata headers
        for (key, value) in &req.context.metadata {
            transport_req = transport_req.header(key.clone(), value.clone());
        }

        // Encode request body (if needed)
        let body = codec.encode_request(&req.operation)?;
        if !body.is_empty() && method != "GET" {
            transport_req = transport_req.body(body);
        }

        Ok(transport_req)
    }

    /// Parse a transport response into an A2A response
    fn parse_transport_response(
        transport_resp: crate::transport::TransportResponse,
        codec: &dyn Codec,
        operation: &A2AOperation,
    ) -> Result<A2AResponse, A2AError> {
        // Check for error status codes
        if !transport_resp.is_success() {
            return Err(Self::handle_error_response(&transport_resp));
        }

        // Decode the response body
        codec.decode_response(&transport_resp.body, operation)
    }

    /// Handle error responses from the transport
    fn handle_error_response(transport_resp: &crate::transport::TransportResponse) -> A2AError {
        // Try to parse error body as JSON
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&transport_resp.body) {
            if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                return match transport_resp.status {
                    401 | 403 => A2AError::Auth(message.to_string()),
                    404 => {
                        if let Some(task_id) = json.get("taskId").and_then(|v| v.as_str()) {
                            A2AError::TaskNotFound {
                                task_id: task_id.to_string(),
                            }
                        } else {
                            A2AError::Protocol(message.to_string())
                        }
                    }
                    429 => A2AError::RateLimitExceeded,
                    _ => {
                        A2AError::Transport(format!("HTTP {}: {}", transport_resp.status, message))
                    }
                };
            }
        }

        // Fallback error
        A2AError::Transport(format!("HTTP error: {}", transport_resp.status))
    }
}

impl<T> Service<A2ARequest> for A2AProtocolService<T>
where
    T: Transport + Clone,
{
    type Response = A2AResponse;
    type Error = A2AError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.transport.poll_ready(cx)
    }

    fn call(&mut self, req: A2ARequest) -> Self::Future {
        let transport = self.transport.clone();
        let codec = self.codec.clone();

        Box::pin(async move {
            // Convert A2A request to transport request
            let transport_req = Self::build_transport_request(&req, codec.as_ref())?;

            // Execute via transport
            let transport_resp = transport.execute(transport_req).await?;

            // Parse transport response to A2A response
            let response =
                Self::parse_transport_response(transport_resp, codec.as_ref(), &req.operation)?;

            Ok(response)
        })
    }
}

impl<T> Clone for A2AProtocolService<T>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            codec: self.codec.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::{
        codec::JsonCodec,
        protocol::{message::Message, task::Task},
        service::RequestContext,
        transport::{mock::MockTransport, TransportResponse},
    };

    use super::*;

    #[tokio::test]
    async fn test_service_send_message() {
        // Create a mock transport that returns a task
        let transport = MockTransport::new(|_req| {
            let task = Task::new("task-123", Message::user("Test"));
            let json = serde_json::to_vec(&task).unwrap();

            TransportResponse::new(200).body(Bytes::from(json))
        });

        let codec = Arc::new(JsonCodec);
        let mut service = A2AProtocolService::new(transport, codec);

        let operation = A2AOperation::SendMessage {
            message: Message::user("Hello"),
            stream: false,
            context_id: None,
            task_id: None,
        };

        let request = A2ARequest::new(operation, RequestContext::default());

        let response = service.call(request).await.unwrap();

        match response {
            A2AResponse::Task(task) => {
                assert_eq!(task.id, "task-123");
            }
            _ => panic!("Expected Task response"),
        }
    }

    #[tokio::test]
    async fn test_service_error_handling() {
        // Create a mock transport that returns an error
        let transport = MockTransport::new(|_req| {
            let error_json = r#"{"message": "Unauthorized"}"#;
            TransportResponse::new(401).body(Bytes::from(error_json))
        });

        let codec = Arc::new(JsonCodec);
        let mut service = A2AProtocolService::new(transport, codec);

        let operation = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };

        let request = A2ARequest::new(operation, RequestContext::default());

        let result = service.call(request).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), A2AError::Auth(_)));
    }
}
