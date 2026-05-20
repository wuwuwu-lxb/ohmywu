use std::collections::HashMap;
use std::sync::RwLock;

use ohmywu_domain::Capability;

/// Registry for atomic capabilities.
pub struct CapabilityRegistry {
    capabilities: RwLock<HashMap<String, Capability>>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self {
            capabilities: RwLock::new(HashMap::new()),
        }
    }

    pub fn register(&self, cap: Capability) {
        let mut caps = self.capabilities.write().unwrap();
        caps.insert(cap.name.clone(), cap);
    }

    pub fn replace_all(&self, items: Vec<Capability>) {
        let mut caps = self.capabilities.write().unwrap();
        caps.clear();
        for cap in items {
            caps.insert(cap.name.clone(), cap);
        }
    }

    pub fn get(&self, name: &str) -> Option<Capability> {
        let caps = self.capabilities.read().unwrap();
        caps.get(name).cloned()
    }

    pub fn list(&self) -> Vec<Capability> {
        let caps = self.capabilities.read().unwrap();
        caps.values().cloned().collect()
    }

    pub fn contains(&self, name: &str) -> bool {
        let caps = self.capabilities.read().unwrap();
        caps.contains_key(name)
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}
