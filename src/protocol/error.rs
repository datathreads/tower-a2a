//! Error types for A2A protocol operations

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Main error type for A2A protocol operations
#[derive(Debug, Error)]
pub enum A2AError {
    /// Transport-level error (network, connection, etc.)
    #[error("Transport error: {0}")]
    Transport(String),

    /// Protocol-level error (invalid message format, unsupported operation, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Validation error (invalid request or response)
    #[error("Validation error: {0}")]
    Validation(String),

    /// Authentication or authorization error
    #[error("Authentication error: {0}")]
    Auth(String),

    /// Task-specific error
    #[error("Task error: {source}")]
    Task {
        #[from]
        source: TaskError,
    },

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Request timeout error
    #[error("Request timeout")]
    Timeout,

    /// Task not found error
    #[error("Task not found: {task_id}")]
    TaskNotFound { task_id: String },

    /// Agent not found or unreachable
    #[error("Agent not found or unreachable: {agent_url}")]
    AgentNotFound { agent_url: url::Url },

    /// Rate limit exceeded
    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    /// Generic error with custom message
    #[error("{0}")]
    Other(String),
}

/// Task-specific error with structured information
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Error)]
#[error("{message}")]
pub struct TaskError {
    /// Error code (e.g., "INVALID_INPUT", "PROCESSING_FAILED")
    pub code: String,

    /// Human-readable error message
    pub message: String,

    /// Additional error details as structured data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl TaskError {
    /// Create a new task error
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
        }
    }

    /// Add details to the task error
    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }
}

/// Result type alias for A2A operations
pub type A2AResult<T> = Result<T, A2AError>;

impl From<reqwest::Error> for A2AError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            A2AError::Timeout
        } else if err.is_connect() {
            A2AError::Transport(format!("Connection error: {}", err))
        } else {
            A2AError::Transport(err.to_string())
        }
    }
}

impl From<&str> for A2AError {
    fn from(s: &str) -> Self {
        A2AError::Other(s.to_string())
    }
}

impl From<String> for A2AError {
    fn from(s: String) -> Self {
        A2AError::Other(s)
    }
}
