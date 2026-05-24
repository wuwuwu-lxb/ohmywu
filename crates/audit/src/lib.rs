use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::RwLock;

use ohmywu_domain::{self, AuditEvent, RiskLevel};

/// Audit log — immutable record of all significant operations.
pub struct AuditLog {
    events: RwLock<Vec<AuditEvent>>,
    file_path: PathBuf,
}

impl AuditLog {
    pub fn load(base_dir: &Path) -> Result<Self, String> {
        fs::create_dir_all(base_dir)
            .map_err(|e| format!("Create audit dir {}: {}", base_dir.display(), e))?;
        let file_path = base_dir.join("events.jsonl");
        let events = if file_path.exists() {
            read_events(&file_path)?
        } else {
            Vec::new()
        };
        Ok(Self {
            events: RwLock::new(events),
            file_path,
        })
    }

    const MAX_EVENTS: usize = 10_000;

    pub fn record(
        &self,
        session_id: Option<&str>,
        turn_id: Option<&str>,
        actor: &str,
        action: &str,
        target: &str,
        risk_level: RiskLevel,
        status: &str,
        detail: Option<&str>,
    ) {
        let now = ohmywu_domain::chrono_now();
        let event = AuditEvent {
            session_id: session_id.map(|item| item.to_string()),
            turn_id: turn_id.map(|item| item.to_string()),
            actor: actor.to_string(),
            action: action.to_string(),
            target: target.to_string(),
            risk_level,
            status: status.to_string(),
            detail: detail.map(|s| s.to_string()),
            timestamp: now,
        };

        let mut events = self.events.write().unwrap();
        events.push(event.clone());
        if events.len() > Self::MAX_EVENTS {
            let excess = events.len() - Self::MAX_EVENTS;
            events.drain(..excess);
        }
        let snapshot = events.clone();
        drop(events);

        let _ = append_event(&self.file_path, &event);
        let _ = rewrite_if_needed(&self.file_path, &snapshot);
    }

    pub fn list(&self, limit: usize) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        let start = events.len().saturating_sub(limit);
        events[start..].to_vec()
    }

    pub fn clear(&self) -> Result<(), String> {
        let mut events = self.events.write().unwrap();
        events.clear();
        drop(events);
        fs::write(&self.file_path, "").map_err(|e| format!("Clear audit log {}: {}", self.file_path.display(), e))
    }

    pub fn list_by_session(&self, session_id: Option<&str>) -> Vec<AuditEvent> {
        let events = self.events.read().unwrap();
        events
            .iter()
            .filter(|event| event.session_id.as_deref() == session_id)
            .cloned()
            .collect()
    }

    pub fn list_all(&self) -> Vec<AuditEvent> {
        self.events.read().unwrap().clone()
    }
}

fn read_events(path: &Path) -> Result<Vec<AuditEvent>, String> {
    let file = OpenOptions::new()
        .read(true)
        .open(path)
        .map_err(|e| format!("Open audit log {}: {}", path.display(), e))?;
    let reader = BufReader::new(file);
    let mut events = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line = line.map_err(|e| format!("Read audit line {}: {}", index + 1, e))?;
        if line.trim().is_empty() {
            continue;
        }
        let event: AuditEvent = serde_json::from_str(&line)
            .map_err(|e| format!("Parse audit line {}: {}", index + 1, e))?;
        events.push(event);
    }
    if events.len() > AuditLog::MAX_EVENTS {
        let start = events.len() - AuditLog::MAX_EVENTS;
        events = events[start..].to_vec();
    }
    Ok(events)
}

fn append_event(path: &Path, event: &AuditEvent) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("Open audit log {}: {}", path.display(), e))?;
    let line = serde_json::to_string(event).map_err(|e| format!("Serialize audit event: {}", e))?;
    writeln!(file, "{}", line).map_err(|e| format!("Write audit log {}: {}", path.display(), e))?;
    Ok(())
}

fn rewrite_if_needed(path: &Path, events: &[AuditEvent]) -> Result<(), String> {
    if events.len() < AuditLog::MAX_EVENTS {
        return Ok(());
    }
    let tmp_path = path.with_extension("jsonl.tmp");
    let mut file = OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&tmp_path)
        .map_err(|e| format!("Open audit tmp {}: {}", tmp_path.display(), e))?;
    for event in events {
        let line = serde_json::to_string(event).map_err(|e| format!("Serialize audit event: {}", e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Write audit tmp {}: {}", tmp_path.display(), e))?;
    }
    fs::rename(&tmp_path, path)
        .map_err(|e| format!("Rename audit tmp {}: {}", tmp_path.display(), e))
}

impl Default for AuditLog {
    fn default() -> Self {
        Self::load(Path::new(".")).expect("failed to initialize default audit log")
    }
}
