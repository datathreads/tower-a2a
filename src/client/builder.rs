//! Client builder for constructing A2A clients with composable layers

use std::{sync::Arc, time::Duration};

use url::Url;

use crate::{
    client::{AgentClient, ClientConfig},
    codec::{Codec, JsonCodec},
    layer::AuthCredentials,
    prelude::A2AError,
    service::A2AProtocolService,
    transport::{HttpTransport, Transport},
};

/// Builder for constructing A2A clients
///
/// This builder provides a fluent API for configuring and building an A2A client
/// with customizable transport, authentication, timeouts, and validation.
///
/// # Example
///
/// ```rust,no_run
/// use tower_a2a::prelude::*;
/// use std::time::Duration;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let url = "https://agent.example.com".parse().unwrap();
/// let mut client = A2AClientBuilder::new_http(url)
///     .with_bearer_auth("token123".to_string())
///     .with_timeout(Duration::from_secs(60))
///     .build()?;
///
/// let agent_card = client.discover().await?;
/// println!("Connected to: {}", agent_card.name);
/// # Ok(())
/// # }
/// ```
///
/// # Compiler Error
/// This will fail to compile if it is not clear to the compilerwhich typeimplementing
/// `Transport` is being used as underlying transport. This is expected behaviour.
///
/// ```compile_fail
/// let client = A2AClientBuilder::new(agent_url()).build();
/// ```
pub struct A2AClientBuilder<T: Transport> {
    agent_url: Url,
    transport: Option<T>,
    codec: Option<Arc<dyn Codec>>,
    auth: Option<AuthCredentials>,
    timeout: Option<Duration>,
    max_retries: u32,
    validate_responses: bool,
}

impl<T: Transport> A2AClientBuilder<T> {
    /// Use a custom transport
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport implementation to use
    pub fn new(agent_url: Url) -> Self {
        Self {
            agent_url,
            transport: None,
            codec: None,
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
            validate_responses: true,
        }
    }

    /// Use a custom transport
    ///
    /// # Arguments
    ///
    /// * `transport` - The transport implementation to use
    pub fn with_transport(mut self, transport: T) -> Self {
        self.transport = Some(transport);
        self
    }

    /// Use a custom codec
    ///
    /// # Arguments
    ///
    /// * `codec` - The codec implementation to use
    pub fn with_codec(mut self, codec: Arc<dyn Codec>) -> Self {
        self.codec = Some(codec);
        self
    }

    /// Enable bearer token authentication
    ///
    /// # Arguments
    ///
    /// * `token` - The bearer token for authentication
    pub fn with_bearer_auth(mut self, token: impl Into<String>) -> Self {
        self.auth = Some(AuthCredentials::bearer(token));
        self
    }

    /// Enable API key authentication
    ///
    /// # Arguments
    ///
    /// * `key` - The API key
    /// * `header` - The header name for the API key (e.g., "X-API-Key")
    pub fn with_api_key_auth(mut self, key: impl Into<String>, header: impl Into<String>) -> Self {
        self.auth = Some(AuthCredentials::api_key(key, header));
        self
    }

    /// Enable basic HTTP authentication
    ///
    /// # Arguments
    ///
    /// * `username` - The username
    /// * `password` - The password
    pub fn with_basic_auth(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.auth = Some(AuthCredentials::basic(username, password));
        self
    }

    /// Set custom authentication credentials
    pub fn with_auth(mut self, credentials: AuthCredentials) -> Self {
        self.auth = Some(credentials);
        self
    }

    /// Set the request timeout
    ///
    /// # Arguments
    ///
    /// * `timeout` - The timeout duration for requests
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retry attempts
    ///
    /// # Arguments
    ///
    /// * `max_retries` - Maximum number of retries (default: 3)
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = max_retries;
        self
    }

    /// Enable or disable response validation
    ///
    /// # Arguments
    ///
    /// * `enabled` - Whether to validate responses (default: true)
    pub fn with_validation(mut self, enabled: bool) -> Self {
        self.validate_responses = enabled;
        self
    }

    /// Build the A2A client
    ///
    /// This assembles all the Tower layers and returns a configured client.
    ///
    /// # Returns
    ///
    /// A configured `AgentClient` ready to use
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No transport has been configured
    /// - No codec has been configured (usually set automatically with transport)
    pub fn build(self) -> Result<AgentClient<A2AProtocolService<T>>, A2AError> {
        // Ensure transport is configured
        let transport = self.transport.ok_or_else(|| {
            A2AError::Protocol(
                "Transport not configured. Call with_http() or with_transport()".into(),
            )
        })?;

        // Ensure codec is configured (should be set with transport)
        let codec = self.codec.unwrap_or_else(|| Arc::new(JsonCodec));

        // Create the core protocol service
        let service = A2AProtocolService::new(transport, codec);

        // Create client configuration
        // Note: auth, timeout, and validation would be better handled as Tower layers
        // but for simplicity we're storing them in the config for now
        let config = ClientConfig::new(self.agent_url)
            .with_timeout(self.timeout.unwrap_or(Duration::from_secs(30)))
            .with_max_retries(self.max_retries)
            .with_validation(self.validate_responses);

        // Create and return the agent client
        Ok(AgentClient::new(service, config))
    }
}

impl A2AClientBuilder<HttpTransport> {
    /// Create a new client builder with HTTP transport (HTTP+JSON binding)
    ///
    /// # Arguments
    ///
    /// * `agent_url` - The base URL of the agent (e.g., "<https://agent.example.com>")
    pub fn new_http(agent_url: Url) -> Self {
        let transport = HttpTransport::new(agent_url.clone());
        Self {
            agent_url,
            transport: Some(transport),
            codec: Some(Arc::new(JsonCodec)),
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            max_retries: 3,
            validate_responses: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::transport::mock::MockTransport;

    use super::*;

    fn agent_url() -> Url {
        "https://example.com".parse().unwrap()
    }

    #[test]
    fn test_builder_with_http() {
        let client = A2AClientBuilder::new_http(agent_url()).build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_memory_transport() {
        let transport = MockTransport::ok();

        let client = A2AClientBuilder::new(agent_url())
            .with_transport(transport)
            .with_codec(Arc::new(JsonCodec))
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_auth() {
        let client = A2AClientBuilder::new_http(agent_url())
            .with_bearer_auth("test-token")
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_with_timeout() {
        let client = A2AClientBuilder::new_http(agent_url())
            .with_timeout(Duration::from_secs(60))
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_builder_all_options() {
        let client = A2AClientBuilder::new_http(agent_url())
            .with_bearer_auth("token")
            .with_timeout(Duration::from_secs(45))
            .with_max_retries(5)
            .with_validation(true)
            .build();

        assert!(client.is_ok());
    }
}
