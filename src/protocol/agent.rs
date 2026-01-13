//! Agent discovery and capability types

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use url::Url;
use uuid::Uuid;

/// Agent scope for granular access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentScope {
    pub scope_type: String,
    pub scope_id: Uuid,
}

/// Agent Card for agent discovery
///
/// The Agent Card is published at `/.well-known/agent-card.json` and describes
/// the agent's capabilities, supported interfaces, and authentication requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCard {
    /// Database ID (not part of protocol, used for database storage)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    /// Organization ID (not part of protocol, used for multi-tenancy)
    #[serde(
        rename = "organizationId",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub organization_id: Option<Uuid>,

    /// Name of the agent
    pub name: String,

    /// Human-readable description of the agent
    pub description: String,

    /// Agent capabilities
    pub capabilities: AgentCapabilities,

    /// Supported authentication schemes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authentication: Option<Vec<SecurityScheme>>,

    /// Endpoint configurations for different bindings
    pub endpoints: HashMap<String, EndpointConfig>,

    /// Agent version
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,

    /// URL to agent documentation
    #[serde(rename = "documentationUrl", skip_serializing_if = "Option::is_none")]
    pub documentation_url: Option<String>,

    /// Granular access control scopes (not part of protocol, used for authorization)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<AgentScope>>,
}

impl AgentCard {
    /// Create a new agent card
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        capabilities: AgentCapabilities,
    ) -> Self {
        Self {
            id: None,
            organization_id: None,
            name: name.into(),
            description: description.into(),
            capabilities,
            authentication: None,
            endpoints: HashMap::new(),
            version: None,
            documentation_url: None,
            scopes: None,
        }
    }

    /// Add an endpoint to the agent card
    pub fn with_endpoint(mut self, name: impl Into<String>, config: EndpointConfig) -> Self {
        self.endpoints.insert(name.into(), config);
        self
    }

    /// Add authentication schemes
    pub fn with_authentication(mut self, schemes: Vec<SecurityScheme>) -> Self {
        self.authentication = Some(schemes);
        self
    }

    /// Set the agent version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

/// Agent capabilities
#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCapabilities {
    /// Supports streaming responses
    #[serde(default)]
    pub streaming: bool,

    /// Supports push notifications via webhooks
    #[serde(rename = "pushNotifications", default)]
    pub push_notifications: bool,

    /// Supports task management (get, list, cancel)
    #[serde(rename = "taskManagement", default)]
    pub task_management: bool,

    /// Supports multi-turn conversations with context
    #[serde(rename = "multiTurn", default)]
    pub multi_turn: bool,

    /// Supported message part types
    #[serde(rename = "supportedPartTypes", skip_serializing_if = "Option::is_none")]
    pub supported_part_types: Option<Vec<String>>,
}

impl AgentCapabilities {
    /// Create capabilities with default values (all false)
    pub fn new() -> Self {
        Self::default()
    }

    /// Enable streaming
    pub fn with_streaming(mut self) -> Self {
        self.streaming = true;
        self
    }

    /// Enable push notifications
    pub fn with_push_notifications(mut self) -> Self {
        self.push_notifications = true;
        self
    }

    /// Enable task management
    pub fn with_task_management(mut self) -> Self {
        self.task_management = true;
        self
    }

    /// Enable multi-turn conversations
    pub fn with_multi_turn(mut self) -> Self {
        self.multi_turn = true;
        self
    }
}

/// API Key security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    pub description: Option<String>,
    #[serde(rename = "in")]
    pub location: String,
    pub name: String,
}

/// HTTP authentication security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    pub description: Option<String>,
    pub scheme: String,
    pub bearer_format: Option<String>,
}

/// OAuth flow configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlow {
    pub authorization_url: Option<Url>,
    pub token_url: Option<Url>,
    pub refresh_url: Option<Url>,
    pub scopes: HashMap<String, String>,
}

/// OAuth flows configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlows {
    pub authorization_code: Option<OAuthFlow>,
    pub client_credentials: Option<OAuthFlow>,
    pub implicit: Option<OAuthFlow>,
    pub password: Option<OAuthFlow>,
}

/// OAuth2 security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2SecurityScheme {
    pub description: Option<String>,
    pub flows: OAuthFlows,
    pub oauth2_metadata_url: Option<Url>,
}

/// OpenID Connect security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    pub description: Option<String>,
    pub open_id_connect_url: Url,
}

/// Security scheme for authentication
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SecurityScheme {
    #[serde(rename = "apiKeySecurityScheme")]
    ApiKey(ApiKeySecurityScheme),
    #[serde(rename = "httpAuthSecurityScheme")]
    HttpAuth(HttpAuthSecurityScheme),
    #[serde(rename = "oauth2SecurityScheme")]
    OAuth2(Box<OAuth2SecurityScheme>),
    #[serde(rename = "openIdConnectSecurityScheme")]
    OpenIdConnect(OpenIdConnectSecurityScheme),
}

/// Endpoint configuration for a specific binding
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EndpointConfig {
    /// Base URL for the endpoint
    pub url: String,

    /// Protocol/binding type (e.g., "http+json", "grpc", "json-rpc")
    #[serde(rename = "type")]
    pub endpoint_type: String,

    /// Whether this endpoint is preferred
    #[serde(default)]
    pub preferred: bool,
}

impl EndpointConfig {
    /// Create a new endpoint configuration
    pub fn new(url: impl Into<String>, endpoint_type: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            endpoint_type: endpoint_type.into(),
            preferred: false,
        }
    }

    /// Mark this endpoint as preferred
    pub fn preferred(mut self) -> Self {
        self.preferred = true;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_agent_card_creation() {
        let capabilities = AgentCapabilities::new()
            .with_streaming()
            .with_task_management();

        let card = AgentCard::new("Test Agent", "A test agent", capabilities)
            .with_version("1.0.0")
            .with_endpoint(
                "http",
                EndpointConfig::new("https://example.com", "http+json").preferred(),
            );

        assert_eq!(card.name, "Test Agent");
        assert!(card.capabilities.streaming);
        assert!(card.capabilities.task_management);
        assert_eq!(card.version, Some("1.0.0".to_string()));
        assert_eq!(card.endpoints.len(), 1);
    }

    #[test]
    fn test_agent_capabilities() {
        let mut caps = AgentCapabilities::default();
        assert!(!caps.streaming);
        assert!(!caps.task_management);

        caps = caps.with_streaming().with_multi_turn();
        assert!(caps.streaming);
        assert!(caps.multi_turn);
    }

    #[test]
    fn test_security_schemes() {
        let http_auth = SecurityScheme::HttpAuth(HttpAuthSecurityScheme {
            description: None,
            scheme: "bearer".to_string(),
            bearer_format: None,
        });
        let api_key = SecurityScheme::ApiKey(ApiKeySecurityScheme {
            description: None,
            location: "header".to_string(),
            name: "X-API-Key".to_string(),
        });

        assert!(matches!(http_auth, SecurityScheme::HttpAuth(_)));
        assert!(matches!(api_key, SecurityScheme::ApiKey(_)));
    }

    #[test]
    fn test_agent_card_serialization() {
        let capabilities = AgentCapabilities::new().with_streaming();
        let card = AgentCard::new("Test", "Description", capabilities);

        let json = serde_json::to_string(&card).unwrap();
        assert!(json.contains("\"name\":\"Test\""));

        let deserialized: AgentCard = serde_json::from_str(&json).unwrap();
        assert_eq!(card, deserialized);
    }
}
