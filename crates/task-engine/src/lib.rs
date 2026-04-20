use chrono::Utc;
use ohmywu_domain::{Task, TaskStatus};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct TaskEngine;

impl TaskEngine {
    pub fn create_task(&self, title: impl Into<String>) -> Task {
        Task {
            id: Uuid::new_v4(),
            title: title.into(),
            status: TaskStatus::Draft,
            created_at: Utc::now(),
        }
    }

    pub fn transition(&self, mut task: Task, status: TaskStatus) -> Task {
        task.status = status;
        task
    }
}
