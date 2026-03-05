//! Authentication types, request context, and auth Tower layer

use std::{
    collections::HashMap,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};

use base64::{engine::general_purpose, Engine as _};
use tower_layer::Layer;
use tower_service::Service;
use url::Url;

use crate::common::error::A2AError;

/// Authentication credentials
#[derive(Debug, Clone)]
pub enum AuthCredentials {
    /// Bearer token authentication
    Bearer(String),

    /// API key authentication
    ApiKey { key: String, header: String },

    /// Basic HTTP authentication
    Basic { username: String, password: String },
}

impl AuthCredentials {
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    pub fn api_key(key: impl Into<String>, header: impl Into<String>) -> Self {
        Self::ApiKey {
            key: key.into(),
            header: header.into(),
        }
    }

    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Get the (header-name, header-value) pair for this credential
    pub fn to_header(&self) -> (String, String) {
        match self {
            AuthCredentials::Bearer(token) => {
                ("Authorization".to_string(), format!("Bearer {token}"))
            }
            AuthCredentials::ApiKey { key, header } => (header.clone(), key.clone()),
            AuthCredentials::Basic { username, password } => {
                let credentials = format!("{username}:{password}");
                let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
                ("Authorization".to_string(), format!("Basic {encoded}"))
            }
        }
    }
}

/// Request context containing metadata and configuration
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// Base URL of the target agent
    pub agent_url: Url,

    /// Authentication credentials (injected by AuthLayer)
    pub auth: Option<AuthCredentials>,

    /// Request timeout
    pub timeout: Option<Duration>,

    /// Additional metadata headers
    pub metadata: HashMap<String, String>,
}

impl RequestContext {
    pub fn new(agent_url: Url) -> Self {
        Self {
            agent_url,
            auth: None,
            timeout: Some(Duration::from_secs(30)),
            metadata: HashMap::new(),
        }
    }

    pub fn with_auth(mut self, auth: AuthCredentials) -> Self {
        self.auth = Some(auth);
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl Default for RequestContext {
    fn default() -> Self {
        Self::new(Url::parse("http://localhost:8080").unwrap())
    }
}

/// Generic A2A request wrapping any operation type
#[derive(Debug, Clone)]
pub struct A2ARequest<Op> {
    pub operation: Op,
    pub context: RequestContext,
}

impl<Op> A2ARequest<Op> {
    pub fn new(operation: Op, context: RequestContext) -> Self {
        Self { operation, context }
    }
}

/// Authentication layer — injects credentials into request context
#[derive(Clone)]
pub struct AuthLayer {
    credentials: AuthCredentials,
}

impl AuthLayer {
    pub fn new(credentials: AuthCredentials) -> Self {
        Self { credentials }
    }

    pub fn bearer(token: impl Into<String>) -> Self {
        Self::new(AuthCredentials::bearer(token))
    }

    pub fn api_key(key: impl Into<String>, header: impl Into<String>) -> Self {
        Self::new(AuthCredentials::api_key(key, header))
    }
}

impl<S> Layer<S> for AuthLayer {
    type Service = AuthService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthService {
            inner,
            credentials: self.credentials.clone(),
        }
    }
}

/// Authentication service wrapping an inner service
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    credentials: AuthCredentials,
}

impl<S, Op, Resp> Service<A2ARequest<Op>> for AuthService<S>
where
    S: Service<A2ARequest<Op>, Response = Resp, Error = A2AError> + Clone + Send + 'static,
    S::Future: Send,
    Op: Send + 'static,
    Resp: Send + 'static,
{
    type Response = Resp;
    type Error = A2AError;
    type Future = Pin<Box<dyn Future<Output = Result<Resp, A2AError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: A2ARequest<Op>) -> Self::Future {
        req.context.auth = Some(self.credentials.clone());
        let mut inner = self.inner.clone();
        Box::pin(async move { inner.call(req).await })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bearer_credentials() {
        let creds = AuthCredentials::bearer("test-token");
        let (header, value) = creds.to_header();
        assert_eq!(header, "Authorization");
        assert_eq!(value, "Bearer test-token");
    }

    #[test]
    fn test_api_key_credentials() {
        let creds = AuthCredentials::api_key("secret-key", "X-API-Key");
        let (header, value) = creds.to_header();
        assert_eq!(header, "X-API-Key");
        assert_eq!(value, "secret-key");
    }

    #[test]
    fn test_basic_credentials() {
        let creds = AuthCredentials::basic("user", "pass");
        let (header, value) = creds.to_header();
        assert_eq!(header, "Authorization");
        assert!(value.starts_with("Basic "));
    }

    #[test]
    fn test_request_context() {
        let ctx = RequestContext::new("https://example.com".parse().unwrap())
            .with_timeout(Duration::from_secs(60))
            .with_metadata("key", "value");
        assert_eq!(ctx.timeout, Some(Duration::from_secs(60)));
        assert_eq!(ctx.metadata.get("key"), Some(&"value".to_string()));
    }
}
