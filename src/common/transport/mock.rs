//! Mock transport for testing

use std::{
    sync::Arc,
    task::{Context, Poll},
};

use async_trait::async_trait;
use url::Url;

use crate::common::error::A2AError;

use super::{Transport, TransportRequest, TransportResponse};

/// Mock transport for unit tests
#[derive(Clone)]
pub struct MockTransport {
    handler: Arc<dyn Fn(TransportRequest) -> TransportResponse + Send + Sync>,
    base_url: Url,
}

impl MockTransport {
    pub fn new<F>(handler: F) -> Self
    where
        F: Fn(TransportRequest) -> TransportResponse + Send + Sync + 'static,
    {
        Self {
            handler: Arc::new(handler),
            base_url: Url::parse("mock://localhost").unwrap(),
        }
    }

    pub fn ok() -> Self {
        Self::new(|_| TransportResponse::new(200))
    }
}

#[async_trait]
impl Transport for MockTransport {
    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), A2AError>> {
        Poll::Ready(Ok(()))
    }

    async fn execute(&self, request: TransportRequest) -> Result<TransportResponse, A2AError> {
        Ok((self.handler)(request))
    }

    fn base_url(&self) -> &Url {
        &self.base_url
    }
}

impl std::fmt::Debug for MockTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MockTransport").finish()
    }
}
