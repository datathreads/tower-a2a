//! v1 Tower service implementation

use std::{
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use tower_service::Service;

use crate::common::{
    auth::A2ARequest,
    error::A2AError,
    transport::{Transport, TransportRequest},
};

use super::{
    codec::Codec,
    types::{A2AResponse, Operation},
};

/// Type alias for v1 A2A requests
pub type A2ARequestV1 = A2ARequest<Operation>;

/// Core v1 A2A protocol service wrapping a transport
pub struct A2AProtocolService<T> {
    transport: T,
    codec: Arc<dyn Codec>,
}

impl<T: Transport> A2AProtocolService<T> {
    pub fn new(transport: T, codec: Arc<dyn Codec>) -> Self {
        Self { transport, codec }
    }

    fn build_transport_request(
        req: &A2ARequest<Operation>,
        codec: &dyn Codec,
    ) -> Result<TransportRequest, A2AError> {
        // Agent card discovery uses a plain GET, everything else is a JSON-RPC POST
        let (endpoint, method): (&str, &str) = match &req.operation {
            Operation::DiscoverAgent => ("/.well-known/agent-card.json", "GET"),
            _ => ("/", "POST"),
        };

        let mut transport_req = TransportRequest::new(endpoint, method);

        let accept = match &req.operation {
            Operation::SendStreamingMessage { .. } | Operation::SubscribeToTask { .. } => {
                "text/event-stream"
            }
            _ => codec.content_type(),
        };
        transport_req = transport_req.header("Content-Type", codec.content_type());
        transport_req = transport_req.header("Accept", accept);
        transport_req = transport_req.header("A2A-Version", "1.0");

        if let Some(auth) = &req.context.auth {
            let (header, value) = auth.to_header();
            transport_req = transport_req.header(header, value);
        }

        for (key, value) in &req.context.metadata {
            transport_req = transport_req.header(key.clone(), value.clone());
        }

        if method == "POST" {
            let body = codec.encode_request(&req.operation)?;
            transport_req = transport_req.body(body);
        }

        Ok(transport_req)
    }

    fn parse_response(
        transport_resp: crate::common::transport::TransportResponse,
        codec: &dyn Codec,
        operation: &Operation,
    ) -> Result<A2AResponse, A2AError> {
        if !transport_resp.is_success() {
            return Err(Self::error_from_response(&transport_resp));
        }
        codec.decode_response(&transport_resp.body, operation)
    }

    fn error_from_response(resp: &crate::common::transport::TransportResponse) -> A2AError {
        if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&resp.body) {
            if let Some(message) = json.get("message").and_then(|v| v.as_str()) {
                return match resp.status {
                    401 | 403 => A2AError::Auth(message.to_string()),
                    404 => A2AError::Protocol(format!("Not found: {}", message)),
                    429 => A2AError::RateLimitExceeded,
                    _ => A2AError::Transport(format!("HTTP {}: {}", resp.status, message)),
                };
            }
        }
        A2AError::Transport(format!("HTTP error: {}", resp.status))
    }
}

impl<T: Transport + Clone> Service<A2ARequest<Operation>> for A2AProtocolService<T> {
    type Response = A2AResponse;
    type Error = A2AError;
    type Future = Pin<Box<dyn Future<Output = Result<A2AResponse, A2AError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.transport.poll_ready(cx)
    }

    fn call(&mut self, req: A2ARequest<Operation>) -> Self::Future {
        let transport = self.transport.clone();
        let codec = self.codec.clone();

        Box::pin(async move {
            let transport_req = Self::build_transport_request(&req, codec.as_ref())?;
            let transport_resp = transport.execute(transport_req).await?;
            Self::parse_response(transport_resp, codec.as_ref(), &req.operation)
        })
    }
}

impl<T: Clone> Clone for A2AProtocolService<T> {
    fn clone(&self) -> Self {
        Self {
            transport: self.transport.clone(),
            codec: self.codec.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        common::{auth::RequestContext, transport::mock::MockTransport},
        v1::{codec::JsonRpcCodec, types::Message},
    };

    use super::*;

    fn make_service() -> A2AProtocolService<MockTransport> {
        let transport = MockTransport::ok();
        A2AProtocolService::new(transport, Arc::new(JsonRpcCodec))
    }

    #[test]
    fn test_build_transport_request_discover() {
        let service = make_service();
        let ctx = RequestContext::new("https://example.com".parse().unwrap());
        let req = A2ARequest::new(Operation::DiscoverAgent, ctx);
        let tr = A2AProtocolService::<MockTransport>::build_transport_request(
            &req,
            service.codec.as_ref(),
        )
        .unwrap();
        assert_eq!(tr.endpoint, "/.well-known/agent-card.json");
        assert_eq!(tr.method, "GET");
        assert!(tr.body.is_empty());
    }

    #[test]
    fn test_build_transport_request_send_message() {
        let service = make_service();
        let ctx = RequestContext::new("https://example.com".parse().unwrap());
        let req = A2ARequest::new(
            Operation::SendMessage {
                message: Message::user("Hello"),
                configuration: None,
            },
            ctx,
        );
        let tr = A2AProtocolService::<MockTransport>::build_transport_request(
            &req,
            service.codec.as_ref(),
        )
        .unwrap();
        assert_eq!(tr.endpoint, "/");
        assert_eq!(tr.method, "POST");
        assert!(!tr.body.is_empty());
        assert_eq!(tr.headers.get("Accept").map(|s| s.as_str()), Some("application/json"));
    }

    #[test]
    fn test_build_transport_request_streaming_accept() {
        let service = make_service();
        let ctx = RequestContext::new("https://example.com".parse().unwrap());
        let req = A2ARequest::new(
            Operation::SendStreamingMessage {
                message: Message::user("Hello"),
                configuration: None,
            },
            ctx,
        );
        let tr = A2AProtocolService::<MockTransport>::build_transport_request(
            &req,
            service.codec.as_ref(),
        )
        .unwrap();
        assert_eq!(tr.headers.get("Accept").map(|s| s.as_str()), Some("text/event-stream"));
    }

    #[test]
    fn test_build_transport_request_subscribe_accept() {
        let service = make_service();
        let ctx = RequestContext::new("https://example.com".parse().unwrap());
        let req = A2ARequest::new(
            Operation::SubscribeToTask { id: "t-1".to_string() },
            ctx,
        );
        let tr = A2AProtocolService::<MockTransport>::build_transport_request(
            &req,
            service.codec.as_ref(),
        )
        .unwrap();
        assert_eq!(tr.headers.get("Accept").map(|s| s.as_str()), Some("text/event-stream"));
    }
}
