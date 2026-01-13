//! Validation layer for A2A protocol requests and responses

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tower_layer::Layer;
use tower_service::Service;

use crate::{
    prelude::{MessagePart, TaskStatus},
    protocol::{error::A2AError, operation::A2AOperation},
    service::{A2ARequest, A2AResponse},
};

/// Layer that validates A2A protocol requests and responses
#[derive(Clone, Debug, Default)]
pub struct A2AValidationLayer;

impl A2AValidationLayer {
    /// Create a new validation layer
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for A2AValidationLayer {
    type Service = A2AValidationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        A2AValidationService { inner }
    }
}

/// Validation service that wraps an inner service
#[derive(Clone)]
pub struct A2AValidationService<S> {
    inner: S,
}

impl<S> A2AValidationService<S> {
    /// Validate an A2A request
    fn validate_request(req: &A2ARequest) -> Result<(), A2AError> {
        match &req.operation {
            A2AOperation::SendMessage { message, .. } => {
                // Message must have at least one part
                if message.parts.is_empty() {
                    return Err(A2AError::Validation(
                        "Message must have at least one part".into(),
                    ));
                }

                // Validate each part (basic checks)
                for part in &message.parts {
                    match part {
                        MessagePart::Text { text } => {
                            if text.is_empty() {
                                return Err(A2AError::Validation(
                                    "Text part cannot be empty".into(),
                                ));
                            }
                        }
                        MessagePart::File { file } => {
                            if file.name.is_empty() {
                                return Err(A2AError::Validation(
                                    "File name cannot be empty".into(),
                                ));
                            }
                            if file.file_with_uri.is_none() && file.file_with_bytes.is_none() {
                                return Err(A2AError::Validation(
                                    "File must have either URI or bytes content".into(),
                                ));
                            }
                        }
                        MessagePart::Data { .. } => {
                            // Data validation could be more specific
                        }
                    }
                }
            }
            A2AOperation::GetTask { task_id } => {
                if task_id.is_empty() {
                    return Err(A2AError::Validation("Task ID cannot be empty".into()));
                }
            }
            A2AOperation::CancelTask { task_id } => {
                if task_id.is_empty() {
                    return Err(A2AError::Validation("Task ID cannot be empty".into()));
                }
            }
            A2AOperation::ListTasks { limit, offset, .. } => {
                if let Some(limit_val) = limit {
                    if *limit_val == 0 {
                        return Err(A2AError::Validation("Limit must be greater than 0".into()));
                    }
                    if *limit_val > 1000 {
                        return Err(A2AError::Validation("Limit cannot exceed 1000".into()));
                    }
                }

                if let Some(offset_val) = offset {
                    if *offset_val > 1000000 {
                        return Err(A2AError::Validation("Offset is too large".into()));
                    }
                }
            }
            A2AOperation::RegisterWebhook { url, events, .. } => {
                if url.is_empty() {
                    return Err(A2AError::Validation("Webhook URL cannot be empty".into()));
                }
                if events.is_empty() {
                    return Err(A2AError::Validation(
                        "Webhook must subscribe to at least one event".into(),
                    ));
                }
            }
            _ => {}
        }

        // Validate agent URL
        if req.context.agent_url.is_empty() {
            return Err(A2AError::Validation("Agent URL cannot be empty".into()));
        }

        Ok(())
    }

    /// Validate an A2A response
    fn validate_response(resp: &A2AResponse) -> Result<(), A2AError> {
        match resp {
            A2AResponse::Task(task) => {
                if task.id.is_empty() {
                    return Err(A2AError::Validation("Task ID cannot be empty".into()));
                }

                // Validate task has input
                if task.input.parts.is_empty() {
                    return Err(A2AError::Validation(
                        "Task input must have at least one part".into(),
                    ));
                }

                // If task is completed, it should have artifacts or error
                if task.status == TaskStatus::Completed
                    && task.artifacts.is_empty()
                    && task.error.is_none()
                {
                    return Err(A2AError::Validation(
                        "Completed task must have artifacts or error".into(),
                    ));
                }

                // If task is failed, it should have an error
                if task.status == TaskStatus::Failed && task.error.is_none() {
                    return Err(A2AError::Validation(
                        "Failed task must have an error".into(),
                    ));
                }
            }
            A2AResponse::AgentCard(card) => {
                if card.name.is_empty() {
                    return Err(A2AError::Validation("Agent name cannot be empty".into()));
                }
                if card.endpoints.is_empty() {
                    return Err(A2AError::Validation(
                        "Agent card must have at least one endpoint".into(),
                    ));
                }
            }
            _ => {}
        }

        Ok(())
    }
}

impl<S> Service<A2ARequest> for A2AValidationService<S>
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

    fn call(&mut self, req: A2ARequest) -> Self::Future {
        // Validate request before passing to inner service
        if let Err(e) = Self::validate_request(&req) {
            return Box::pin(async move { Err(e) });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move {
            let response = inner.call(req).await?;

            // Validate response
            Self::validate_response(&response)?;

            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        protocol::{message::Message, task::Task},
        service::RequestContext,
    };

    use super::*;

    #[test]
    fn test_validate_send_message() {
        let operation = A2AOperation::SendMessage {
            message: Message::user("Hello"),
            stream: false,
            context_id: None,
            task_id: None,
        };

        let context = RequestContext::new("https://example.com");
        let request = A2ARequest::new(operation, context);

        assert!(A2AValidationService::<()>::validate_request(&request).is_ok());
    }

    #[test]
    fn test_validate_empty_message() {
        let mut message = Message::user("Test");
        message.parts.clear();

        let operation = A2AOperation::SendMessage {
            message,
            stream: false,
            context_id: None,
            task_id: None,
        };

        let context = RequestContext::new("https://example.com");
        let request = A2ARequest::new(operation, context);

        assert!(A2AValidationService::<()>::validate_request(&request).is_err());
    }

    #[test]
    fn test_validate_task_response() {
        let task = Task::new("task-123", Message::user("Test"));
        let response = A2AResponse::Task(Box::new(task));

        assert!(A2AValidationService::<()>::validate_response(&response).is_ok());
    }
}
