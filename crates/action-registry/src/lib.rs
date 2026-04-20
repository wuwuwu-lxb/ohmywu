use std::collections::BTreeMap;

use ohmywu_domain::ActionSpec;

#[derive(Debug, Clone, Default)]
pub struct ActionRegistry {
    actions: BTreeMap<String, ActionSpec>,
}

impl ActionRegistry {
    pub fn from_specs(specs: Vec<ActionSpec>) -> Self {
        let mut actions = BTreeMap::new();
        for spec in specs {
            actions.insert(spec.name.clone(), spec);
        }
        Self { actions }
    }

    pub fn list(&self) -> Vec<ActionSpec> {
        self.actions.values().cloned().collect()
    }

    pub fn get(&self, name: &str) -> Option<ActionSpec> {
        self.actions.get(name).cloned()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.actions.contains_key(name)
    }
}
