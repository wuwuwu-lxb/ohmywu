use std::sync::RwLock;

use ohmywu_domain::{self, AuditEvent, RiskLevel};

/// Audit log — immutable record of all significant operations.
pub struct AuditLog {
    events: RwLock<Vec<AuditEvent>>,
}

impl AuditLog {
    pub fn new() -> Self {
        Self {
            events: RwLock::new(Vec::new()),
        }
    }

    const MAX_EVENTS: usize = 10_000;

    pub fn record(
        &self,
        actor: &str,
        action: &str,
        target: &str,
        risk_level: RiskLevel,
        status: &str,
        detail: Option<&str>,
    ) {
        let mut events = self.events.write().unwrap();
        let now = ohmywu_domain::chrono_now();
        events.push(AuditEvent {
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            risk_level,
            status: status.to_string(),
            detail: detail.map(|s| s.to_string()),
            timestamp: now,
        });
        // cap at MAX_EVENTS, keep newest
        if events.len() > Self::MAX_EVENTS {
            let excess = events.len() - Self::MAX_EVENTS;
            events.drain(..excess);
        }
    }

    pub fn list(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::new()
    }
}
