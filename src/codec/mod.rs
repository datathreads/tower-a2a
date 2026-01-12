//! Serialization codecs for different protocol bindings

pub mod json;
pub mod jsonrpc;
pub mod sse;

pub use json::JsonCodec;
pub use jsonrpc::JsonRpcCodec;
pub use sse::{SseCodec, SseEvent};

use crate::{
    protocol::{error::A2AError, operation::A2AOperation},
    service::response::A2AResponse,
};
use bytes::Bytes;

/// Codec trait for encoding and decoding A2A protocol messages
///
/// Different codecs implement different protocol bindings (HTTP+JSON, gRPC+Protobuf, etc.)
pub trait Codec: Send + Sync {
    /// Serialize an A2A operation to bytes for transport
    ///
    /// # Arguments
    ///
    /// * `operation` - The A2A operation to encode
    ///
    /// # Returns
    ///
    /// The serialized bytes or an error
    fn encode_request(&self, operation: &A2AOperation) -> Result<Bytes, A2AError>;

    /// Deserialize transport response bytes to an A2A response
    ///
    /// # Arguments
    ///
    /// * `body` - The response body bytes
    /// * `operation` - The original operation (for context)
    ///
    /// # Returns
    ///
    /// The deserialized A2A response or an error
    fn decode_response(
        &self,
        body: &[u8],
        operation: &A2AOperation,
    ) -> Result<A2AResponse, A2AError>;

    /// Get the content type for this codec
    ///
    /// # Returns
    ///
    /// The MIME type (e.g., "application/json", "application/protobuf")
    fn content_type(&self) -> &str;
}
