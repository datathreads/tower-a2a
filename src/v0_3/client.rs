//! v0.3.0 client builder and agent client

use std::{collections::HashMap, sync::Arc, time::Duration};

use futures::Stream;
use tower_service::Service;
use url::Url;

use crate::common::{
    auth::{A2ARequest, AuthCredentials, RequestContext},
    error::A2AError,
    transport::{HttpTransport, Transport},
};

use super::{
    codec::{Codec, JsonRpcCodec, SseEvent},
    service::A2AProtocolService,
    types::{A2AResponse, AgentCard, Message, Operation, Task},
};

/// Configuration for the v0.3.0 A2A client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub agent_url: Url,
    pub timeout: Duration,
    pub max_retries: u32,
    pub auth: Option<AuthCredentials>,
    pub extra_headers: HashMap<String, String>,
}

impl ClientConfig {
    pub fn new(agent_url: Url) -> Self {
        Self {
            agent_url,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            auth: None,
            extra_headers: HashMap::new(),
        }
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_auth(mut self, auth: AuthCredentials) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }
}

/// High-level A2A v0.3.0 client
pub struct AgentClient<S> {
    service: S,
    config: ClientConfig,
}

impl<S> AgentClient<S>
where
    S: Service<A2ARequest<Operation>, Response = A2AResponse, Error = A2AError>,
{
    pub fn new(service: S, config: ClientConfig) -> Self {
        Self { service, config }
    }

    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    fn context(&self) -> RequestContext {
        let mut ctx =
            RequestContext::new(self.config.agent_url.clone()).with_timeout(self.config.timeout);
        if let Some(ref auth) = self.config.auth {
            ctx = ctx.with_auth(auth.clone());
        }
        for (k, v) in &self.config.extra_headers {
            ctx = ctx.with_metadata(k.clone(), v.clone());
        }
        ctx
    }

    /// Fetch the agent card
    pub async fn discover(&mut self) -> Result<AgentCard, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(Operation::DiscoverAgent, self.context()))
            .await?;
        resp.into_agent_card()
            .ok_or_else(|| A2AError::Protocol("Expected AgentCard response".into()))
    }

    /// Send a message to the agent
    pub async fn send_message(&mut self, message: Message) -> Result<Task, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(
                Operation::SendMessage {
                    message,
                    task_id: None,
                    skill_id: None,
                    configuration: None,
                },
                self.context(),
            ))
            .await?;
        resp.into_task()
            .ok_or_else(|| A2AError::Protocol("Expected Task response".into()))
    }

    /// Get a task by ID
    pub async fn get_task(&mut self, id: String) -> Result<Task, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(Operation::GetTask { id }, self.context()))
            .await?;
        resp.into_task()
            .ok_or_else(|| A2AError::Protocol("Expected Task response".into()))
    }

    /// Cancel a task by ID
    pub async fn cancel_task(&mut self, id: String) -> Result<Task, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(
                Operation::CancelTask { id },
                self.context(),
            ))
            .await?;
        resp.into_task()
            .ok_or_else(|| A2AError::Protocol("Expected Task response".into()))
    }

    /// Poll a task until it reaches a terminal state
    pub async fn poll_until_complete(
        &mut self,
        id: String,
        poll_interval_ms: u64,
        max_attempts: usize,
    ) -> Result<Task, A2AError> {
        let mut attempts = 0;

        loop {
            let task = self.get_task(id.clone()).await?;

            if task.is_terminal() {
                return Ok(task);
            }

            attempts += 1;
            if max_attempts > 0 && attempts >= max_attempts {
                return Err(A2AError::Timeout);
            }

            tokio::time::sleep(tokio::time::Duration::from_millis(poll_interval_ms)).await;
        }
    }
}

impl AgentClient<A2AProtocolService<HttpTransport>> {
    /// Stream a message via SSE (`message/stream` method).
    pub async fn stream_message(
        &mut self,
        message: Message,
    ) -> Result<impl Stream<Item = Result<SseEvent, A2AError>>, A2AError> {
        let context = self.context();
        self.service
            .stream_operation(A2ARequest::new(
                Operation::StreamMessage {
                    message,
                    task_id: None,
                    skill_id: None,
                    configuration: None,
                },
                context,
            ))
            .await
    }
}

/// Builder for constructing v0.3.0 A2A clients
pub struct A2AClientBuilder<T: Transport> {
    agent_url: Url,
    transport: Option<T>,
    codec: Option<Arc<dyn Codec>>,
    auth: Option<AuthCredentials>,
    timeout: Option<Duration>,
    max_retries: u32,
    extra_headers: HashMap<String, String>,
}

impl<T: Transport> A2AClientBuilder<T> {
    pub fn new(agent_url: Url) -> Self {
        Self {
            agent_url,
            transport: None,
            codec: None,
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
            extra_headers: HashMap::new(),
        }
    }

    pub fn with_transport(mut self, transport: T) -> Self {
        self.transport = Some(transport);
        self
    }

    pub fn with_codec(mut self, codec: Arc<dyn Codec>) -> Self {
        self.codec = Some(codec);
        self
    }

    pub fn with_bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(AuthCredentials::bearer(token));
        self
    }

    pub fn with_auth(mut self, credentials: AuthCredentials) -> Self {
        self.auth = Some(credentials);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    pub fn with_extra_header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra_headers.insert(key.into(), value.into());
        self
    }

    pub fn build(self) -> Result<AgentClient<A2AProtocolService<T>>, A2AError> {
        let transport = self
            .transport
            .ok_or_else(|| A2AError::Protocol("Transport not configured".into()))?;
        let codec = self.codec.unwrap_or_else(|| Arc::new(JsonRpcCodec));
        let service = A2AProtocolService::new(transport, codec);

        let mut config = ClientConfig::new(self.agent_url).with_max_retries(self.max_retries);
        if let Some(timeout) = self.timeout {
            config = config.with_timeout(timeout);
        }
        if let Some(auth) = self.auth {
            config = config.with_auth(auth);
        }
        for (k, v) in self.extra_headers {
            config = config.with_extra_header(k, v);
        }

        Ok(AgentClient::new(service, config))
    }
}

impl A2AClientBuilder<HttpTransport> {
    pub fn new_http(agent_url: Url) -> Self {
        let transport = HttpTransport::new(agent_url.clone());
        Self {
            agent_url,
            transport: Some(transport),
            codec: Some(Arc::new(JsonRpcCodec)),
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
            extra_headers: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_new_http() {
        let client = A2AClientBuilder::new_http("https://example.com".parse().unwrap()).build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_auth() {
        let client = A2AClientBuilder::new_http("https://example.com".parse().unwrap())
            .with_bearer_auth("test-token")
            .build()
            .unwrap();
        assert!(client.config().auth.is_some());
    }

    #[test]
    fn test_builder_with_extra_header() {
        let client = A2AClientBuilder::new_http("https://example.com".parse().unwrap())
            .with_extra_header("traceparent", "00-abc-def-01")
            .build()
            .unwrap();
        assert_eq!(
            client.config().extra_headers.get("traceparent"),
            Some(&"00-abc-def-01".to_string())
        );
    }
}
