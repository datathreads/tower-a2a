//! Common infrastructure shared between v1 and v0_3 modules

pub mod auth;
pub mod error;
pub mod security;
pub mod transport;

pub use auth::{A2ARequest, AuthCredentials, AuthLayer, AuthService, RequestContext};
pub use error::{A2AError, A2AResult, TaskError};
pub use security::{AgentCardSignature, OAuthFlows, SecurityScheme};
pub use transport::{HttpTransport, Transport, TransportRequest, TransportResponse, WebSocketTransport};
