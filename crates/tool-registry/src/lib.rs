use std::collections::BTreeMap;

use ohmywu_domain::ToolSpec;

#[derive(Debug, Clone, Default)]
pub struct ToolRegistry {
    tools: BTreeMap<String, ToolSpec>,
}

impl ToolRegistry {
    pub fn from_specs(specs: Vec<ToolSpec>) -> Self {
        let mut tools = BTreeMap::new();
        for spec in specs {
            tools.insert(spec.name.clone(), spec);
        }
        Self { tools }
    }

    pub fn list(&self) -> Vec<ToolSpec> {
        self.tools.values().cloned().collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}
