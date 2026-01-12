//! A2A service response types

use crate::protocol::{agent::AgentCard, task::Task};

/// Response from an A2A service operation
#[derive(Debug, Clone)]
pub enum A2AResponse {
    /// Task response (from SendMessage, GetTask, CancelTask)
    Task(Box<Task>),

    /// Task list response (from ListTasks)
    TaskList {
        /// The tasks matching the query
        tasks: Vec<Task>,

        /// Total number of tasks
        total: usize,

        /// Optional continuation token for pagination
        next_token: Option<String>,
    },

    /// Agent card response (from DiscoverAgent)
    AgentCard(Box<AgentCard>),

    /// Empty response (for operations with no return value)
    Empty,
}

impl A2AResponse {
    /// Extract a task from the response, if present
    pub fn into_task(self) -> Option<Task> {
        match self {
            A2AResponse::Task(task) => Some(*task),
            _ => None,
        }
    }

    /// Extract a task list from the response, if present
    pub fn into_task_list(self) -> Option<Vec<Task>> {
        match self {
            A2AResponse::TaskList { tasks, .. } => Some(tasks),
            _ => None,
        }
    }

    /// Extract an agent card from the response, if present
    pub fn into_agent_card(self) -> Option<AgentCard> {
        match self {
            A2AResponse::AgentCard(card) => Some(*card),
            _ => None,
        }
    }

    /// Check if the response is empty
    pub fn is_empty(&self) -> bool {
        matches!(self, A2AResponse::Empty)
    }
}

#[cfg(test)]
mod tests {
    use crate::protocol::message::Message;

    use super::*;

    #[test]
    fn test_response_task() {
        let task = Task::new("task-123", Message::user("Test"));
        let response = A2AResponse::Task(Box::new(task));

        assert!(matches!(response, A2AResponse::Task(_)));

        let extracted = response.into_task();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().id, "task-123");
    }

    #[test]
    fn test_response_task_list() {
        let task1 = Task::new("task-1", Message::user("Test 1"));
        let task2 = Task::new("task-2", Message::user("Test 2"));

        let response = A2AResponse::TaskList {
            tasks: vec![task1, task2],
            total: 2,
            next_token: None,
        };

        let extracted = response.into_task_list();
        assert!(extracted.is_some());
        assert_eq!(extracted.unwrap().len(), 2);
    }

    #[test]
    fn test_response_empty() {
        let response = A2AResponse::Empty;
        assert!(response.is_empty());
    }
}
