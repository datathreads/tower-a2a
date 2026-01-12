//! High-level A2A agent client

use crate::{
    client::config::ClientConfig,
    prelude::A2AError,
    protocol::{A2AOperation, AgentCard, Message, Task, TaskStatus},
    service::{A2ARequest, A2AResponse, RequestContext},
};
use tower_service::Service;

/// High-level A2A client for interacting with agents
///
/// This client wraps a Tower service and provides convenient methods for common A2A operations.
/// The service is generic over any implementation that satisfies the Service trait bounds.
///
/// # Example
///
/// ```rust,no_run
/// use tower_a2a::prelude::*;
///
/// # async fn example() -> Result<(), A2AError> {
/// let url = "https://agent.example.com".parse().unwrap();
/// let mut client = A2AClientBuilder::new(url)
///     .with_http()
///     .build()?;
///
/// let message = Message::user("Hello, agent!");
/// let task = client.send_message(message).await?;
/// println!("Task created: {}", task.id);
/// # Ok(())
/// # }
/// ```
pub struct AgentClient<S> {
    service: S,
    config: ClientConfig,
}

impl<S> AgentClient<S>
where
    S: Service<A2ARequest, Response = A2AResponse, Error = A2AError>,
{
    /// Create a new agent client
    ///
    /// # Arguments
    ///
    /// * `service` - The Tower service that handles requests
    /// * `config` - Client configuration
    pub fn new(service: S, config: ClientConfig) -> Self {
        Self { service, config }
    }

    /// Get the client configuration
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// Build a request context from the client configuration
    fn build_context(&self) -> RequestContext {
        RequestContext {
            agent_url: self.config.agent_url.clone(),
            auth: None, // Set by AuthLayer
            timeout: Some(self.config.timeout),
            metadata: Default::default(),
        }
    }

    /// Send a message to the agent and get a task
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send to the agent
    ///
    /// # Returns
    ///
    /// A task representing the agent's processing of the message
    ///
    /// # Errors
    ///
    /// Returns an error if the message fails to send or the response is invalid
    pub async fn send_message(&mut self, message: Message) -> Result<Task, A2AError> {
        let operation = A2AOperation::SendMessage {
            message,
            stream: false,
            context_id: None,
            task_id: None,
        };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::Task(task) => Ok(*task),
            _ => Err(A2AError::Protocol(
                "Expected task response from send_message".into(),
            )),
        }
    }

    /// Send a message with streaming enabled
    ///
    /// Note: Streaming is not yet fully implemented
    pub async fn send_message_streaming(&mut self, message: Message) -> Result<Task, A2AError> {
        let operation = A2AOperation::SendMessage {
            message,
            stream: true,
            context_id: None,
            task_id: None,
        };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::Task(task) => Ok(*task),
            _ => Err(A2AError::Protocol(
                "Expected task response from send_message_streaming".into(),
            )),
        }
    }

    /// Send a message in a specific context for multi-turn conversations
    ///
    /// # Arguments
    ///
    /// * `message` - The message to send
    /// * `context_id` - The context ID for grouping related messages
    pub async fn send_message_in_context(
        &mut self,
        message: Message,
        context_id: String,
    ) -> Result<Task, A2AError> {
        let operation = A2AOperation::SendMessage {
            message,
            stream: false,
            context_id: Some(context_id),
            task_id: None,
        };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::Task(task) => Ok(*task),
            _ => Err(A2AError::Protocol(
                "Expected task response from send_message_in_context".into(),
            )),
        }
    }

    /// Get a task by ID
    ///
    /// # Arguments
    ///
    /// * `task_id` - The unique identifier of the task to retrieve
    ///
    /// # Returns
    ///
    /// The task with the specified ID
    ///
    /// # Errors
    ///
    /// Returns `A2AError::TaskNotFound` if the task doesn't exist
    pub async fn get_task(&mut self, task_id: String) -> Result<Task, A2AError> {
        let operation = A2AOperation::GetTask { task_id };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::Task(task) => Ok(*task),
            _ => Err(A2AError::Protocol(
                "Expected task response from get_task".into(),
            )),
        }
    }

    /// List tasks with optional filtering
    ///
    /// # Arguments
    ///
    /// * `status` - Optional filter by task status
    /// * `limit` - Maximum number of tasks to return (default: 100)
    ///
    /// # Returns
    ///
    /// A vector of tasks matching the query
    pub async fn list_tasks(
        &mut self,
        status: Option<TaskStatus>,
        limit: Option<u32>,
    ) -> Result<Vec<Task>, A2AError> {
        let operation = A2AOperation::ListTasks {
            status,
            limit,
            offset: None,
            next_token: None,
        };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::TaskList { tasks, .. } => Ok(tasks),
            _ => Err(A2AError::Protocol(
                "Expected task list response from list_tasks".into(),
            )),
        }
    }

    /// List all tasks without filtering
    pub async fn list_all_tasks(&mut self) -> Result<Vec<Task>, A2AError> {
        self.list_tasks(None, None).await
    }

    /// List tasks with a specific status
    pub async fn list_tasks_by_status(
        &mut self,
        status: TaskStatus,
    ) -> Result<Vec<Task>, A2AError> {
        self.list_tasks(Some(status), None).await
    }

    /// Cancel a task by ID
    ///
    /// # Arguments
    ///
    /// * `task_id` - The unique identifier of the task to cancel
    ///
    /// # Returns
    ///
    /// The updated task with cancelled status
    pub async fn cancel_task(&mut self, task_id: String) -> Result<Task, A2AError> {
        let operation = A2AOperation::CancelTask { task_id };

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::Task(task) => Ok(*task),
            _ => Err(A2AError::Protocol(
                "Expected task response from cancel_task".into(),
            )),
        }
    }

    /// Discover agent capabilities by fetching the Agent Card
    ///
    /// This retrieves the agent's metadata from `/.well-known/agent-card.json`
    ///
    /// # Returns
    ///
    /// The agent's capability card
    pub async fn discover(&mut self) -> Result<AgentCard, A2AError> {
        let operation = A2AOperation::DiscoverAgent;

        let request = A2ARequest::new(operation, self.build_context());
        let response = self.service.call(request).await?;

        match response {
            A2AResponse::AgentCard(card) => Ok(*card),
            _ => Err(A2AError::Protocol(
                "Expected agent card response from discover".into(),
            )),
        }
    }

    /// Poll a task until it reaches a terminal state
    ///
    /// This is a convenience method that repeatedly calls get_task until
    /// the task is completed, failed, cancelled, or rejected.
    ///
    /// # Arguments
    ///
    /// * `task_id` - The task ID to poll
    /// * `poll_interval` - How often to poll (in milliseconds)
    /// * `max_attempts` - Maximum number of polling attempts (0 = unlimited)
    ///
    /// # Returns
    ///
    /// The final task state
    pub async fn poll_until_complete(
        &mut self,
        task_id: String,
        poll_interval_ms: u64,
        max_attempts: usize,
    ) -> Result<Task, A2AError> {
        let mut attempts = 0;

        loop {
            let task = self.get_task(task_id.clone()).await?;

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        codec::JsonCodec,
        protocol::message::Message,
        service::A2AProtocolService,
        transport::{mock::MockTransport, TransportResponse},
    };
    use bytes::Bytes;

    use super::*;

    #[tokio::test]
    async fn test_send_message() {
        let transport = MockTransport::new(|_req| {
            let task = Task::new("task-123", Message::user("Test"));
            let json = serde_json::to_vec(&task).unwrap();
            TransportResponse::new(200).body(Bytes::from(json))
        });

        let codec = Arc::new(JsonCodec::new());
        let service = A2AProtocolService::new(transport, codec);
        let config = ClientConfig::new("https://example.com");
        let mut client = AgentClient::new(service, config);

        let message = Message::user("Hello");
        let task = client.send_message(message).await.unwrap();

        assert_eq!(task.id, "task-123");
    }

    #[tokio::test]
    async fn test_get_task() {
        let transport = MockTransport::new(|_req| {
            let task = Task::new("task-456", Message::user("Test"));
            let json = serde_json::to_vec(&task).unwrap();
            TransportResponse::new(200).body(Bytes::from(json))
        });

        let codec = Arc::new(JsonCodec::new());
        let service = A2AProtocolService::new(transport, codec);
        let config = ClientConfig::new("https://example.com");
        let mut client = AgentClient::new(service, config);

        let task = client.get_task("task-456".to_string()).await.unwrap();

        assert_eq!(task.id, "task-456");
    }

    #[tokio::test]
    async fn test_discover() {
        use crate::protocol::agent::{AgentCapabilities, AgentCard};

        let transport = MockTransport::new(|_req| {
            let card = AgentCard::new("Test Agent", "A test agent", AgentCapabilities::default());
            let json = serde_json::to_vec(&card).unwrap();
            TransportResponse::new(200).body(Bytes::from(json))
        });

        let codec = Arc::new(JsonCodec::new());
        let service = A2AProtocolService::new(transport, codec);
        let config = ClientConfig::new("https://example.com");
        let mut client = AgentClient::new(service, config);

        let card = client.discover().await.unwrap();

        assert_eq!(card.name, "Test Agent");
    }
}
