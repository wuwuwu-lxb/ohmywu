use chrono::Utc;
use ohmywu_domain::{AuditEvent, RiskLevel};
use std::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct AuditLog {
    events: RwLock<Vec<AuditEvent>>,
}

impl AuditLog {
    pub fn record(&self, actor: &str, action: &str, target: &str, risk_level: RiskLevel, result: &str, detail: Option<&str>) -> AuditEvent {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            risk_level,
            result: result.to_string(),
            detail: detail.map(String::from),
        };
        {
            let mut events = self.events.write().unwrap();
            events.push(event.clone());
        }
        event
    }

    pub fn list(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events.iter().rev().take(limit).cloned().collect()
    }
}
