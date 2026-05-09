use std::collections::HashMap;
use std::sync::RwLock;

use ohmywu_domain::Action;

/// Registry for stable actions.
pub struct ActionRegistry {
    actions: RwLock<HashMap<String, Action>>,
}

impl ActionRegistry {
    pub fn new() -> Self {
        Self {
            actions: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, action: Action) {
        let mut actions = self.actions.write().unwrap();
        actions.insert(action.id.clone(), action);
    }

    pub fn get(&self, id: &str) -> Option<Action> {
        let actions = self.actions.read().unwrap();
        actions.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Action> {
        let actions = self.actions.read().unwrap();
        actions.values().cloned().collect()
    }

    pub fn contains(&self, id: &str) -> bool {
        let actions = self.actions.read().unwrap();
        actions.contains_key(id)
    }
}

impl Default for ActionRegistry {
    fn default() -> Self {
        Self::new()
    }
}
