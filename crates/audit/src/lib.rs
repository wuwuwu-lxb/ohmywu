use chrono::Utc;
use ohmywu_domain::AuditEvent;
use uuid::Uuid;

#[derive(Debug, Clone, Default)]
pub struct AuditLog;

impl AuditLog {
    pub fn record(&self, actor: impl Into<String>, summary: impl Into<String>) -> AuditEvent {
        AuditEvent {
            id: Uuid::new_v4(),
            actor: actor.into(),
            summary: summary.into(),
            created_at: Utc::now(),
        }
    }
}
