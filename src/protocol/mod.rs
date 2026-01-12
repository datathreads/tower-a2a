//! Core A2A protocol types and definitions

pub mod message;
pub mod task;
pub mod operation;
pub mod agent;
pub mod error;

pub use message::{Message, MessagePart, Role};
pub use task::{Task, TaskStatus};
pub use operation::A2AOperation;
pub use agent::{AgentCard, AgentCapabilities};
pub use error::{A2AError, TaskError};
