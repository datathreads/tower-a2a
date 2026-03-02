//! v1 client builder and agent client

use std::{sync::Arc, time::Duration};

use tower_service::Service;
use url::Url;

use crate::common::{
    auth::{A2ARequest, AuthCredentials, RequestContext},
    error::A2AError,
    transport::{HttpTransport, Transport},
};

use super::{
    codec::{Codec, JsonRpcCodec},
    service::A2AProtocolService,
    types::{AgentCard, A2AResponse, ListTasksParams, Message, Operation, Task},
};

/// Configuration for the v1 A2A client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub agent_url: Url,
    pub timeout: Duration,
    pub max_retries: u32,
}

impl ClientConfig {
    pub fn new(agent_url: Url) -> Self {
        Self {
            agent_url,
            timeout: Duration::from_secs(30),
            max_retries: 3,
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
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("http://.".parse().unwrap())
    }
}

/// High-level A2A v1 client
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
        RequestContext::new(self.config.agent_url.clone())
            .with_timeout(self.config.timeout)
    }

    /// Fetch the agent card from `/.well-known/agent-card.json`
    pub async fn discover(&mut self) -> Result<AgentCard, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(Operation::DiscoverAgent, self.context()))
            .await?;
        resp.into_agent_card()
            .ok_or_else(|| A2AError::Protocol("Expected AgentCard response".into()))
    }

    /// Send a message to the agent, expecting a Task response
    pub async fn send_message(&mut self, message: Message) -> Result<Task, A2AError> {
        let resp = self.send_message_raw(message).await?;
        resp.into_task()
            .ok_or_else(|| A2AError::Protocol("Expected Task response".into()))
    }

    /// Send a message to the agent, returning the raw response (Task or Message)
    pub async fn send_message_raw(&mut self, message: Message) -> Result<A2AResponse, A2AError> {
        self.service
            .call(A2ARequest::new(
                Operation::SendMessage {
                    message,
                    configuration: None,
                },
                self.context(),
            ))
            .await
    }

    /// Get a task by ID
    pub async fn get_task(&mut self, id: String) -> Result<Task, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(
                Operation::GetTask {
                    id,
                    history_length: None,
                },
                self.context(),
            ))
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

    /// List tasks with optional filtering
    pub async fn list_tasks(
        &mut self,
        params: ListTasksParams,
    ) -> Result<Vec<Task>, A2AError> {
        let resp = self
            .service
            .call(A2ARequest::new(
                Operation::ListTasks(params),
                self.context(),
            ))
            .await?;
        resp.into_task_list()
            .ok_or_else(|| A2AError::Protocol("Expected TaskList response".into()))
    }

    /// List all tasks
    pub async fn list_all_tasks(&mut self) -> Result<Vec<Task>, A2AError> {
        self.list_tasks(ListTasksParams::default()).await
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

/// Builder for constructing v1 A2A clients
pub struct A2AClientBuilder<T: Transport> {
    agent_url: Url,
    transport: Option<T>,
    codec: Option<Arc<dyn Codec>>,
    auth: Option<AuthCredentials>,
    timeout: Option<Duration>,
    max_retries: u32,
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

    pub fn with_api_key_auth(
        mut self,
        key: impl Into<String>,
        header: impl Into<String>,
    ) -> Self {
        self.auth = Some(AuthCredentials::api_key(key, header));
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

    pub fn build(self) -> Result<AgentClient<A2AProtocolService<T>>, A2AError> {
        let transport = self.transport.ok_or_else(|| {
            A2AError::Protocol("Transport not configured".into())
        })?;
        let codec = self.codec.unwrap_or_else(|| Arc::new(JsonRpcCodec));
        let service = A2AProtocolService::new(transport, codec);

        let mut config = ClientConfig::new(self.agent_url)
            .with_max_retries(self.max_retries);

        if let Some(timeout) = self.timeout {
            config = config.with_timeout(timeout);
        }

        Ok(AgentClient::new(service, config))
    }
}

impl A2AClientBuilder<HttpTransport> {
    /// Create a builder with HTTP transport
    pub fn new_http(agent_url: Url) -> Self {
        let transport = HttpTransport::new(agent_url.clone());
        Self {
            agent_url,
            transport: Some(transport),
            codec: Some(Arc::new(JsonRpcCodec)),
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use bytes::Bytes;

    use crate::common::transport::{mock::MockTransport, TransportResponse};
    use crate::v1::types::{TaskState, TaskStatus};

    use super::*;

    fn agent_url() -> Url {
        "https://example.com".parse().unwrap()
    }

    fn make_task(id: &str) -> Task {
        Task {
            id: id.to_string(),
            context_id: "ctx-1".to_string(),
            status: TaskStatus {
                state: TaskState::Created,
                timestamp: None,
            },
            state: TaskState::Created,
            messages: None,
            artifacts: None,
            created_time: None,
            updated_time: None,
        }
    }

    #[tokio::test]
    async fn test_send_message() {
        let transport = MockTransport::new(|_| {
            let task = make_task("task-123");
            let json_body = serde_json::json!({
                "jsonrpc": "2.0",
                "result": task,
                "id": "1"
            });
            TransportResponse::new(200).body(Bytes::from(serde_json::to_vec(&json_body).unwrap()))
        });

        let client_builder = A2AClientBuilder::new(agent_url())
            .with_transport(transport)
            .with_codec(Arc::new(JsonRpcCodec));
        let mut client = client_builder.build().unwrap();

        let task = client.send_message(Message::user("Hello")).await.unwrap();
        assert_eq!(task.id, "task-123");
    }

    #[test]
    fn test_builder_new_http() {
        let client = A2AClientBuilder::new_http(agent_url()).build();
        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_auth() {
        let client = A2AClientBuilder::new_http(agent_url())
            .with_bearer_auth("token")
            .build();
        assert!(client.is_ok());
    }
}
