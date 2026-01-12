//! A2A protocol operations

use super::{message::Message, task::TaskStatus};

/// A2A protocol operations
///
/// This enum represents all the abstract operations defined in the A2A protocol spec.
/// Each operation is binding-independent and can be implemented over HTTP, gRPC, etc.
#[derive(Debug, Clone)]
pub enum A2AOperation {
    /// Send a message to an agent
    SendMessage {
        /// The message to send
        message: Message,

        /// Whether to stream the response
        stream: bool,

        /// Optional context ID for multi-turn conversations
        context_id: Option<String>,

        /// Optional task ID to continue from
        task_id: Option<String>,
    },

    /// Get a task by ID
    GetTask {
        /// The task ID to retrieve
        task_id: String,
    },

    /// List tasks with optional filtering
    ListTasks {
        /// Filter by task status
        status: Option<TaskStatus>,

        /// Maximum number of tasks to return
        limit: Option<u32>,

        /// Offset for pagination
        offset: Option<u32>,

        /// Continuation token for pagination
        next_token: Option<String>,
    },

    /// Cancel a task
    CancelTask {
        /// The task ID to cancel
        task_id: String,
    },

    /// Discover agent capabilities (fetch Agent Card)
    DiscoverAgent,

    /// Subscribe to task updates (streaming)
    SubscribeTask {
        /// The task ID to subscribe to
        task_id: String,
    },

    /// Register a webhook for push notifications
    RegisterWebhook {
        /// The webhook URL
        url: String,

        /// Events to subscribe to
        events: Vec<String>,

        /// Optional authentication for webhook calls
        auth: Option<String>,
    },
}

impl A2AOperation {
    /// Get the HTTP endpoint path for this operation
    pub fn endpoint(&self) -> String {
        match self {
            A2AOperation::SendMessage { task_id, .. } => {
                if let Some(id) = task_id {
                    format!("/tasks/{}", id)
                } else {
                    "/tasks".to_string()
                }
            }
            A2AOperation::GetTask { task_id } => format!("/tasks/{}", task_id),
            A2AOperation::ListTasks { .. } => "/tasks".to_string(),
            A2AOperation::CancelTask { task_id } => format!("/tasks/{}/cancel", task_id),
            A2AOperation::DiscoverAgent => "/.well-known/agent-card.json".to_string(),
            A2AOperation::SubscribeTask { task_id } => format!("/tasks/{}/stream", task_id),
            A2AOperation::RegisterWebhook { .. } => "/webhooks".to_string(),
        }
    }

    /// Get the HTTP method for this operation
    pub fn method(&self) -> &'static str {
        match self {
            A2AOperation::SendMessage { task_id, .. } => {
                if task_id.is_some() {
                    "PUT"
                } else {
                    "POST"
                }
            }
            A2AOperation::GetTask { .. } => "GET",
            A2AOperation::ListTasks { .. } => "GET",
            A2AOperation::CancelTask { .. } => "POST",
            A2AOperation::DiscoverAgent => "GET",
            A2AOperation::SubscribeTask { .. } => "GET",
            A2AOperation::RegisterWebhook { .. } => "POST",
        }
    }

    /// Check if this operation expects a streaming response
    pub fn is_streaming(&self) -> bool {
        matches!(
            self,
            A2AOperation::SendMessage { stream: true, .. } | A2AOperation::SubscribeTask { .. }
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::message::Message;

    use super::*;

    #[test]
    fn test_operation_endpoints() {
        let op = A2AOperation::SendMessage {
            message: Message::user("test"),
            stream: false,
            context_id: None,
            task_id: None,
        };
        assert_eq!(op.endpoint(), "/tasks");
        assert_eq!(op.method(), "POST");

        let op = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };
        assert_eq!(op.endpoint(), "/tasks/task-123");
        assert_eq!(op.method(), "GET");

        let op = A2AOperation::DiscoverAgent;
        assert_eq!(op.endpoint(), "/.well-known/agent-card.json");
        assert_eq!(op.method(), "GET");
    }

    #[test]
    fn test_operation_streaming() {
        let op = A2AOperation::SendMessage {
            message: Message::user("test"),
            stream: true,
            context_id: None,
            task_id: None,
        };
        assert!(op.is_streaming());

        let op = A2AOperation::GetTask {
            task_id: "task-123".to_string(),
        };
        assert!(!op.is_streaming());
    }
}
