use std::sync::RwLock;

use ohmywu_domain::{AuditEvent, RiskLevel};

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
        let now = chrono_now();
        events.push(AuditEvent {
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            risk_level,
            status: status.to_string(),
            detail: detail.map(|s| s.to_string()),
            timestamp: now,
        });
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

fn chrono_now() -> String {
    use std::time::SystemTime;
    let ts = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let secs = ts.as_secs();
    let rem = secs % 86400;
    let hours = rem / 3600;
    let minutes = (rem % 3600) / 60;
    let seconds = rem % 60;
    format!("2026-05-10T{:02}:{:02}:{:02}Z", hours, minutes, seconds)
}
