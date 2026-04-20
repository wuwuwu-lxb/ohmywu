use std::collections::BTreeMap;

use ohmywu_domain::WorkflowSpec;

#[derive(Debug, Clone, Default)]
pub struct WorkflowRegistry {
    workflows: BTreeMap<String, WorkflowSpec>,
}

impl WorkflowRegistry {
    pub fn from_specs(specs: Vec<WorkflowSpec>) -> Self {
        let mut workflows = BTreeMap::new();
        for spec in specs {
            workflows.insert(spec.name.clone(), spec);
        }
        Self { workflows }
    }

    pub fn list(&self) -> Vec<WorkflowSpec> {
        self.workflows.values().cloned().collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.workflows.contains_key(name)
    }
}
