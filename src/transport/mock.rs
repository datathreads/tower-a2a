use std::{
    sync::Arc,
    task::{Context, Poll},
};

use async_trait::async_trait;
use url::Url;

use crate::transport::{Transport, TransportRequest, TransportResponse};

/// Mock transport for internal testing
///
/// This transport is used for unit tests to mock agent responses without
/// requiring a real network connection or a mock HTTP server.
#[derive(Clone)]
pub(crate) struct MockTransport {
    handler: Arc<dyn Fn(TransportRequest) -> TransportResponse + Send + Sync>,
    base_url: Url,
}

impl MockTransport {
    /// Create a new mock transport with a custom request handler
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(TransportRequest) -> TransportResponse + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
            base_url: Url::parse("mock://").unwrap(),
        }
    }

    /// Create a mock transport that always returns 200 OK
    #[cfg(test)]
    pub fn ok() -> Self {
        Self::new(|_| TransportResponse::new(200))
    }
}

#[async_trait]
impl Transport for MockTransport {
    fn poll_ready(
        &mut self,
        _cx: &mut Context<'_>,
    ) -> Poll<Result<(), crate::protocol::error::A2AError>> {
        Poll::Ready(Ok(()))
    }

    async fn execute(
        &self,
        request: TransportRequest,
    ) -> Result<TransportResponse, crate::protocol::error::A2AError> {
        Ok((self.handler)(request))
    }

    fn base_url(&self) -> &Url {
        &self.base_url
    }

    fn supports_streaming(&self) -> bool {
        false
    }

    fn clone_box(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}

impl std::fmt::Debug for MockTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockTransport").finish()
    }
}
