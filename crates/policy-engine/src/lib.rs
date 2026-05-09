use std::sync::RwLock;

use ohmywu_domain::{PolicyMode, RiskLevel};

/// Policy engine — gatekeeps capability execution by risk level.
pub struct PolicyEngine {
    mode: RwLock<PolicyMode>,
}

impl PolicyEngine {
    pub fn new() -> Self {
        Self {
            mode: RwLock::new(PolicyMode::Sandbox),
        }
    }

    pub fn set_mode(&self, mode: PolicyMode) {
        let mut current = self.mode.write().unwrap();
        *current = mode;
    }

    pub fn current_mode(&self) -> PolicyMode {
        *self.mode.read().unwrap()
    }

    /// Check whether a capability with the given risk level is allowed.
    /// In Sandbox mode, only ReadOnly capabilities pass.
    /// In Danger mode, all capabilities pass but HighRisk should be audited.
    pub fn check(&self, risk_level: RiskLevel) -> PolicyDecision {
        let mode = self.current_mode();
        let allowed = match mode {
            PolicyMode::Sandbox => risk_level == RiskLevel::ReadOnly,
            PolicyMode::Danger => true,
        };
        PolicyDecision {
            allowed,
            mode,
            risk_level,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyDecision {
    pub allowed: bool,
    pub mode: PolicyMode,
    pub risk_level: RiskLevel,
}

impl Default for PolicyEngine {
    fn default() -> Self {
        Self::new()
    }
}
