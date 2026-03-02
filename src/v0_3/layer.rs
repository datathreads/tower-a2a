//! v0.3.0 Tower validation layer

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use tower_layer::Layer;
use tower_service::Service;

use crate::common::{auth::A2ARequest, error::A2AError};

use super::types::{A2AResponse, Operation, Part};

/// Validation layer for v0.3.0 protocol requests and responses
#[derive(Clone, Debug, Default)]
pub struct ValidationLayer;

impl ValidationLayer {
    pub fn new() -> Self {
        Self
    }
}

impl<S> Layer<S> for ValidationLayer {
    type Service = ValidationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ValidationService { inner }
    }
}

/// Validation service wrapping an inner service
#[derive(Clone)]
pub struct ValidationService<S> {
    inner: S,
}

impl<S> ValidationService<S> {
    fn validate_request(req: &A2ARequest<Operation>) -> Result<(), A2AError> {
        match &req.operation {
            Operation::SendMessage { message, .. } | Operation::StreamMessage { message, .. } => {
                if message.parts.is_empty() {
                    return Err(A2AError::Validation(
                        "Message must have at least one part".into(),
                    ));
                }
                for part in &message.parts {
                    validate_part(part)?;
                }
            }
            Operation::GetTask { id } | Operation::CancelTask { id } => {
                if id.is_empty() {
                    return Err(A2AError::Validation("Task ID cannot be empty".into()));
                }
            }
            _ => {}
        }

        if !req.context.agent_url.has_host() {
            return Err(A2AError::Validation("Agent URL must have a host".into()));
        }

        Ok(())
    }

    fn validate_response(resp: &A2AResponse) -> Result<(), A2AError> {
        match resp {
            A2AResponse::Task(task) => {
                if task.id.is_empty() {
                    return Err(A2AError::Validation("Task ID cannot be empty".into()));
                }
            }
            A2AResponse::AgentCard(card) => {
                if card.name.is_empty() {
                    return Err(A2AError::Validation("Agent name cannot be empty".into()));
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn validate_part(part: &Part) -> Result<(), A2AError> {
    match part {
        Part::Text { text } if text.is_empty() => {
            Err(A2AError::Validation("Text part cannot be empty".into()))
        }
        _ => Ok(()),
    }
}

impl<S> Service<A2ARequest<Operation>> for ValidationService<S>
where
    S: Service<A2ARequest<Operation>, Response = A2AResponse, Error = A2AError>
        + Clone
        + Send
        + 'static,
    S::Future: Send,
{
    type Response = A2AResponse;
    type Error = A2AError;
    type Future = Pin<Box<dyn Future<Output = Result<A2AResponse, A2AError>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: A2ARequest<Operation>) -> Self::Future {
        if let Err(e) = Self::validate_request(&req) {
            return Box::pin(async move { Err(e) });
        }

        let mut inner = self.inner.clone();
        Box::pin(async move {
            let response = inner.call(req).await?;
            Self::validate_response(&response)?;
            Ok(response)
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::common::auth::RequestContext;
    use crate::v0_3::types::Message;

    use super::*;

    #[test]
    fn test_validate_send_message() {
        let req = A2ARequest::new(
            Operation::SendMessage {
                message: Message::user("Hello"),
                task_id: None,
                skill_id: None,
                configuration: None,
            },
            RequestContext::new("https://example.com".parse().unwrap()),
        );
        assert!(ValidationService::<()>::validate_request(&req).is_ok());
    }
}
