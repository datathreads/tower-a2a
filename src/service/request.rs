//! A2A service request types

use std::{collections::HashMap, time::Duration};

use crate::{layer::auth::AuthCredentials, protocol::operation::A2AOperation};

/// A request to the A2A service
///
/// This wraps an A2A operation with additional context needed for execution
#[derive(Debug, Clone)]
pub struct A2ARequest {
    /// The A2A operation to execute
    pub operation: A2AOperation,

    /// Request context (auth, timeouts, metadata)
    pub context: RequestContext,
}

impl A2ARequest {
    /// Create a new A2A request
    pub fn new(operation: A2AOperation, context: RequestContext) -> Self {
        Self { operation, context }
    }
}

/// Request context containing metadata and configuration
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Base URL of the target agent
    pub agent_url: String,

    /// Authentication credentials (if any)
    pub auth: Option<AuthCredentials>,

    /// Request timeout
    pub timeout: Option<Duration>,

    /// Additional metadata headers
    pub metadata: HashMap<String, String>,
}

impl RequestContext {
    /// Create a new request context
    pub fn new(agent_url: impl Into<String>) -> Self {
        Self {
            agent_url: agent_url.into(),
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            metadata: HashMap::new(),
        }
    }

    /// Set authentication credentials
    pub fn with_auth(mut self, auth: AuthCredentials) -> Self {
        self.auth = Some(auth);
        self
    }

    /// Set request timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Add a metadata header
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self {
            agent_url: String::new(),
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            metadata: HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::message::Message;

    #[test]
    fn test_request_context_creation() {
        let context = RequestContext::new("https://example.com")
            .with_timeout(Duration::from_secs(60))
            .with_metadata("key", "value");

        assert_eq!(context.agent_url, "https://example.com");
        assert_eq!(context.timeout, Some(Duration::from_secs(60)));
        assert_eq!(context.metadata.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_request_creation() {
        let operation = A2AOperation::SendMessage {
            message: Message::user("Test"),
            stream: false,
            context_id: None,
            task_id: None,
        };

        let context = RequestContext::new("https://example.com");
        let request = A2ARequest::new(operation, context);

        assert_eq!(request.context.agent_url, "https://example.com");
    }
}
