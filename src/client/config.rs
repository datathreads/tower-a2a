//! Client configuration

use std::time::Duration;

/// Configuration for an A2A client
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// Base URL of the agent
    pub agent_url: String,

    /// Default request timeout
    pub timeout: Duration,

    /// Maximum number of retry attempts
    pub max_retries: u32,

    /// Enable response validation
    pub validate_responses: bool,
}

impl ClientConfig {
    /// Create a new client configuration
    pub fn new(agent_url: impl Into<String>) -> Self {
        Self {
            agent_url: agent_url.into(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            validate_responses: true,
        }
    }

    /// Set the timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Enable or disable response validation
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validate_responses = enabled;
        self
    }
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self::new("")
    }
}
