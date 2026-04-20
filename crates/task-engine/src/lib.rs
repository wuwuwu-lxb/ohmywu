use chrono::Utc;
use ohmywu_domain::{Task, TaskStatus};
use std::collections::HashMap;
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct TaskEngine {
    tasks: RwLock<HashMap<String, Task>>,
}

impl TaskEngine {
    pub fn new_task(&self, action: &str, target: &str) -> String {
        let id = Uuid::new_v4().to_string();
        let task = Task {
            id: Uuid::new_v4(),
            action: action.to_string(),
            target: target.to_string(),
            status: TaskStatus::Pending,
            created_at: Utc::now(),
            completed_at: None,
            result: None,
        };
        {
            let mut tasks = self.tasks.write().unwrap();
            tasks.insert(id.clone(), task);
        }
        id
    }

    pub fn complete(&self, id: &str, result: &str) {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Completed;
            task.completed_at = Some(Utc::now());
            task.result = Some(result.to_string());
        }
    }

    pub fn fail(&self, id: &str, error: &str) {
        let mut tasks = self.tasks.write().unwrap();
        if let Some(task) = tasks.get_mut(id) {
            task.status = TaskStatus::Failed;
            task.completed_at = Some(Utc::now());
            task.result = Some(error.to_string());
        }
    }

    pub fn get_task(&self, id: &str) -> Option<Task> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).cloned()
    }

    pub fn list_tasks(&self) -> Vec<Task> {
        let tasks = self.tasks.read().unwrap();
        tasks.values().cloned().collect()
    }
}
