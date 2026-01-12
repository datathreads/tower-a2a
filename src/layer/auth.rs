//! Authentication layer for A2A protocol

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use base64::{engine::general_purpose, Engine as _};
use tower_layer::Layer;
use tower_service::Service;

use crate::{
    protocol::error::A2AError,
    service::{A2ARequest, A2AResponse},
};

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
    /// Create bearer token credentials
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::Bearer(token.into())
    }

    /// Create API key credentials
    pub fn api_key(key: impl Into<String>, header: impl Into<String>) -> Self {
        Self::ApiKey {
            key: key.into(),
            header: header.into(),
        }
    }

    /// Create basic auth credentials
    pub fn basic(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::Basic {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Get the header name and value for this credential
    pub fn to_header(&self) -> (String, String) {
        match self {
            AuthCredentials::Bearer(token) => {
                ("Authorization".to_string(), format!("Bearer {}", token))
            }
            AuthCredentials::ApiKey { key, header } => (header.clone(), key.clone()),
            AuthCredentials::Basic { username, password } => {
                let credentials = format!("{}:{}", username, password);
                let encoded = general_purpose::STANDARD.encode(credentials.as_bytes());
                ("Authorization".to_string(), format!("Basic {}", encoded))
            }
        }
    }
}

/// Authentication layer
#[derive(Clone)]
pub struct AuthLayer {
    credentials: AuthCredentials,
}

impl AuthLayer {
    /// Create a new authentication layer
    pub fn new(credentials: AuthCredentials) -> Self {
        Self { credentials }
    }

    /// Create a bearer authentication layer
    pub fn bearer(token: impl Into<String>) -> Self {
        Self::new(AuthCredentials::bearer(token))
    }

    /// Create an API key authentication layer
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

/// Authentication service
#[derive(Clone)]
pub struct AuthService<S> {
    inner: S,
    credentials: AuthCredentials,
}

impl<S> Service<A2ARequest> for AuthService<S>
where
    S: Service<A2ARequest, Response = A2AResponse, Error = A2AError> + Clone + Send + 'static,
    S::Future: Send,
{
    type Response = A2AResponse;
    type Error = A2AError;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: A2ARequest) -> Self::Future {
        // Inject credentials into request context
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
}
