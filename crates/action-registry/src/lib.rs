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

    pub fn register_many(&self, items: Vec<Action>) {
        let mut actions = self.actions.write().unwrap();
        for action in items {
            actions.insert(action.id.clone(), action);
        }
    }

    pub fn replace_all(&self, items: Vec<Action>) {
        let mut actions = self.actions.write().unwrap();
        actions.clear();
        for action in items {
            actions.insert(action.id.clone(), action);
        }
    }

    pub fn get(&self, id: &str) -> Option<Action> {
        let actions = self.actions.read().unwrap();
        actions.get(id).cloned()
    }

    pub fn list(&self) -> Vec<Action> {
        let actions = self.actions.read().unwrap();
        let mut items: Vec<Action> = actions.values().cloned().collect();
        items.sort_by_key(|action| action.sort_key());
        items
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
