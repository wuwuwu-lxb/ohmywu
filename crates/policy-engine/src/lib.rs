use ohmywu_domain::{PolicyMode, RiskLevel};

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    mode: PolicyMode,
}

impl PolicyEngine {
    pub fn new(mode: PolicyMode) -> Self {
        Self { mode }
    }

    pub fn allows(&self, risk: &RiskLevel) -> bool {
        match (&self.mode, risk) {
            (PolicyMode::Sandbox, RiskLevel::ReadOnly) => true,
            (PolicyMode::Sandbox, _) => false,
            (PolicyMode::Danger, _) => true,
        }
    }
}
