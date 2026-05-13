use std::collections::HashMap;
use std::sync::RwLock;

use ohmywu_domain::{self, Task, TaskStatus};

/// Task engine — lifecycle management for tracked executions.
pub struct TaskEngine {
    tasks: RwLock<HashMap<String, Task>>,
    counter: RwLock<u64>,
}

impl TaskEngine {
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            counter: RwLock::new(0),
        }
    }

    pub fn create(&self, name: &str, target: &str) -> Task {
        let mut counter = self.counter.write().unwrap();
        *counter += 1;
        let id = format!("task-{}", *counter);

        let now = ohmywu_domain::chrono_now();
        let task = Task {
            id: id.clone(),
            name: name.to_string(),
            target: target.to_string(),
            status: TaskStatus::Running,
            detail: None,
            created_at: now,
        };

        let mut tasks = self.tasks.write().unwrap();
        tasks.insert(id, task.clone());
        task
    }

    pub fn complete(&self, id: &str, detail: &str) -> Option<Task> {
        let mut tasks = self.tasks.write().unwrap();
        tasks.get_mut(id).map(|task| {
            task.status = TaskStatus::Completed;
            task.detail = Some(detail.to_string());
            task.clone()
        })
    }

    pub fn fail(&self, id: &str, detail: &str) -> Option<Task> {
        let mut tasks = self.tasks.write().unwrap();
        tasks.get_mut(id).map(|task| {
            task.status = TaskStatus::Failed;
            task.detail = Some(detail.to_string());
            task.clone()
        })
    }

    pub fn get(&self, id: &str) -> Option<Task> {
        let tasks = self.tasks.read().unwrap();
        tasks.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Task> {
        let tasks = self.tasks.read().unwrap();
        tasks.values().cloned().collect()
    }
}

impl Default for TaskEngine {
    fn default() -> Self {
        Self::new()
    }
}
