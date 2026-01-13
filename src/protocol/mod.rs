//! Core A2A protocol types and definitions

use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod agent;
pub mod error;
pub mod message;
pub mod operation;
pub mod task;

pub use agent::{AgentCapabilities, AgentCard};
pub use error::{A2AError, TaskError};
pub use message::{Message, MessagePart, Role};
pub use operation::A2AOperation;
pub use task::{Task, TaskStatus};

/// Artifacts represent task outputs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Artifact {
    /// Unique identifier of the Artifact
    pub artifact_id: String,

    /// A human readable name for the Artifact
    pub name: Option<String>,

    /// A human readable description of the Artifact
    pub description: Option<String>,

    /// Contents of the Artifact. Must contain at least one part
    pub parts: Vec<MessagePart>,

    pub metadata: Option<Value>,

    /// The URIs of extensions that are present or contributed to this Artifact
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub extensions: Vec<String>,
}
