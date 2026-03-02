//! Shared security scheme types (used by both v1 and v0_3)

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use url::Url;

/// API Key security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApiKeySecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "in")]
    pub location: String,
    pub name: String,
}

/// HTTP authentication security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scheme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
}

/// Authorization Code OAuth flow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCodeOAuthFlow {
    pub authorization_url: Url,
    pub token_url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<Url>,
    pub scopes: HashMap<String, String>,
}

/// Client Credentials OAuth flow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentialsOAuthFlow {
    pub token_url: Url,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<Url>,
    pub scopes: HashMap<String, String>,
}

/// Device Code OAuth flow
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeOAuthFlow {
    pub token_url: Url,
    pub device_authorization_url: Url,
    pub scopes: HashMap<String, String>,
}

/// OAuth flows configuration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OAuthFlows {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authorization_code: Option<AuthorizationCodeOAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_credentials: Option<ClientCredentialsOAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_code: Option<DeviceCodeOAuthFlow>,
}

/// OAuth2 security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuth2SecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub flows: OAuthFlows,
}

/// OpenID Connect security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub open_id_connect_url: Url,
}

/// Mutual TLS security scheme
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MutualTlsSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Security scheme discriminated union
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    #[serde(rename = "apiKey")]
    ApiKey(ApiKeySecurityScheme),
    #[serde(rename = "http")]
    Http(HttpAuthSecurityScheme),
    #[serde(rename = "oauth2")]
    OAuth2(Box<OAuth2SecurityScheme>),
    #[serde(rename = "openIdConnect")]
    OpenIdConnect(OpenIdConnectSecurityScheme),
    #[serde(rename = "mutualTLS")]
    MutualTls(MutualTlsSecurityScheme),
}

/// JWS signature for agent card verification (RFC 7515)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentCardSignature {
    /// Base64url-encoded JSON object (protected header)
    pub protected: String,

    /// Computed signature, Base64url-encoded
    pub signature: String,

    /// Unprotected JWS header values
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<serde_json::Value>,
}
