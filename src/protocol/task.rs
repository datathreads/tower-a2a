//! A2A task types and lifecycle management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{error::TaskError, message::Message};

/// A task in the A2A protocol
///
/// Tasks represent asynchronous operations performed by agents.
/// They have a lifecycle from submitted to completion, with various intermediate states.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    /// Unique identifier for the task
    pub id: String,

    /// Current status of the task
    pub status: TaskStatus,

    /// Input message that created this task
    pub input: Message,

    /// Output message (present when task is completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Message>,

    /// Error information (present if task failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<TaskError>,

    /// When the task was created
    #[serde(rename = "createdAt")]
    pub created_at: DateTime<Utc>,

    /// When the task was last updated
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<DateTime<Utc>>,

    /// Optional context ID for grouping related tasks/messages
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,
}

impl Task {
    /// Create a new task
    pub fn new(id: impl Into<String>, input: Message) -> Self {
        Self {
            id: id.into(),
            status: TaskStatus::Submitted,
            input,
            output: None,
            error: None,
            created_at: Utc::now(),
            updated_at: None,
            context_id: None,
        }
    }

    /// Check if the task is in a terminal state
    pub fn is_terminal(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Rejected
        )
    }

    /// Check if the task is still processing
    pub fn is_processing(&self) -> bool {
        matches!(self.status, TaskStatus::Submitted | TaskStatus::Working)
    }

    /// Check if the task requires input
    pub fn requires_input(&self) -> bool {
        matches!(
            self.status,
            TaskStatus::InputRequired | TaskStatus::AuthRequired
        )
    }

    /// Update the task status
    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self.updated_at = Some(Utc::now());
        self
    }

    /// Set the task output
    pub fn with_output(mut self, output: Message) -> Self {
        self.output = Some(output);
        self.updated_at = Some(Utc::now());
        self
    }

    /// Set the task error
    pub fn with_error(mut self, error: TaskError) -> Self {
        self.error = Some(error);
        self.updated_at = Some(Utc::now());
        self
    }

    /// Set the context ID
    pub fn with_context_id(mut self, context_id: impl Into<String>) -> Self {
        self.context_id = Some(context_id.into());
        self
    }
}

/// Task status in the A2A protocol lifecycle
///
/// Task lifecycle: submitted → working → completed/failed/cancelled/rejected
/// Non-terminal states: input-required, auth-required (awaiting client input)
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatus {
    /// Task has been received and is queued for processing
    Submitted,

    /// Task is currently being processed
    Working,

    /// Task requires additional input from the client
    InputRequired,

    /// Task requires authentication or authorization
    AuthRequired,

    /// Task completed successfully
    Completed,

    /// Task failed with an error
    Failed,

    /// Task was cancelled by the client
    Cancelled,

    /// Task was rejected by the agent (e.g., invalid request)
    Rejected,
}

impl TaskStatus {
    /// Check if this is a terminal status
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            TaskStatus::Completed
                | TaskStatus::Failed
                | TaskStatus::Cancelled
                | TaskStatus::Rejected
        )
    }

    /// Check if this status requires client action
    pub fn requires_action(&self) -> bool {
        matches!(self, TaskStatus::InputRequired | TaskStatus::AuthRequired)
    }
}

/// Request to send a message to an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendMessageRequest {
    /// The message to send
    pub message: Message,

    /// Whether to stream the response
    #[serde(default)]
    pub stream: bool,

    /// Optional context ID for multi-turn conversations
    #[serde(rename = "contextId", skip_serializing_if = "Option::is_none")]
    pub context_id: Option<String>,

    /// Optional task ID to continue from
    #[serde(rename = "taskId", skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
}

/// Response from listing tasks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResponse {
    /// List of tasks
    pub tasks: Vec<Task>,

    /// Total number of tasks matching the query
    pub total: usize,

    /// Optional continuation token for pagination
    #[serde(rename = "nextToken", skip_serializing_if = "Option::is_none")]
    pub next_token: Option<String>,
}

#[cfg(test)]
mod tests {
    use crate::protocol::message::Message;

    use super::*;

    #[test]
    fn test_task_creation() {
        let msg = Message::user("Test");
        let task = Task::new("task-123", msg);

        assert_eq!(task.id, "task-123");
        assert_eq!(task.status, TaskStatus::Submitted);
        assert!(!task.is_terminal());
        assert!(task.is_processing());
    }

    #[test]
    fn test_task_lifecycle() {
        let msg = Message::user("Test");
        let task = Task::new("task-123", msg);

        let task = task.with_status(TaskStatus::Working);
        assert_eq!(task.status, TaskStatus::Working);
        assert!(task.is_processing());

        let task = task.with_status(TaskStatus::Completed);
        assert!(task.is_terminal());
        assert!(!task.is_processing());
    }

    #[test]
    fn test_task_status() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(!TaskStatus::Working.is_terminal());

        assert!(TaskStatus::InputRequired.requires_action());
        assert!(TaskStatus::AuthRequired.requires_action());
        assert!(!TaskStatus::Working.requires_action());
    }

    #[test]
    fn test_task_serialization() {
        let msg = Message::user("Test");
        let task = Task::new("task-123", msg);

        let json = serde_json::to_string(&task).unwrap();
        assert!(json.contains("\"id\":\"task-123\""));
        assert!(json.contains("\"status\":\"submitted\""));

        let deserialized: Task = serde_json::from_str(&json).unwrap();
        assert_eq!(task.id, deserialized.id);
        assert_eq!(task.status, deserialized.status);
    }
}
