use std::collections::BTreeMap;

use ohmywu_domain::{AgentInstance, AgentLifecycleState, AgentTemplate};
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct AgentKernel {
    templates: BTreeMap<String, AgentTemplate>,
}

impl AgentKernel {
    pub fn from_templates(templates: Vec<AgentTemplate>) -> Self {
        let mut map = BTreeMap::new();
        for template in templates {
            map.insert(template.name.clone(), template);
        }
        Self { templates: map }
    }

    pub fn list_templates(&self) -> Vec<AgentTemplate> {
        self.templates.values().cloned().collect()
    }

    pub fn contains_template(&self, name: &str) -> bool {
        self.templates.contains_key(name)
    }

    pub fn spawn_instance(&self, template_name: &str) -> Option<AgentInstance> {
        self.templates.get(template_name).map(|template| AgentInstance {
            id: Uuid::new_v4(),
            template_name: template.name.clone(),
            lifecycle_state: AgentLifecycleState::Trial,
        })
    }
}
