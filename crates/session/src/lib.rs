use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// One message in a session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub executions: Option<Vec<ExecutionRecord>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    pub timestamp: String,
}

/// Execution detail for a single capability invocation within a message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub capability: String,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub status: String,
    pub duration_ms: u64,
}

/// Lightweight summary of a session (no message content).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub message_count: usize,
    pub created_at: String,
    pub updated_at: String,
}

/// Manages session JSONL files on disk.
pub struct SessionManager {
    sessions_dir: PathBuf,
    /// Prevents interleaved writes from concurrent Tauri commands.
    write_lock: Mutex<()>,
}

impl SessionManager {
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self {
            sessions_dir,
            write_lock: Mutex::new(()),
        }
    }

    // ── file path helper ──────────────────────────────────────────

    fn file_path(&self, session_id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.jsonl", session_id))
    }

    // ── session CRUD ──────────────────────────────────────────────

    pub fn create_session(&self, name: &str) -> Result<SessionSummary, String> {
        let now = ohmywu_domain::chrono_now();
        let date = &now[..10].replace('-', "");
        let counter = self.next_counter_for_date(date)?;
        let id = format!("session-{}-{:03}", date, counter);
        let path = self.file_path(&id);

        File::create(&path)
            .map_err(|e| format!("Create session file {}: {}", path.display(), e))?;

        let summary = SessionSummary {
            id,
            name: name.to_string(),
            message_count: 0,
            created_at: now.clone(),
            updated_at: now,
        };

        Ok(summary)
    }

    pub fn append_message(
        &self,
        session_id: &str,
        msg: &SessionMessage,
    ) -> Result<(), String> {
        let line = serde_json::to_string(msg).map_err(|e| format!("Serialize msg: {}", e))?;
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let path = self.file_path(session_id);
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| format!("Open session {}: {}", path.display(), e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Write msg: {}", e))?;
        Ok(())
    }

    pub fn load_session(&self, session_id: &str) -> Result<Vec<SessionMessage>, String> {
        let path = self.file_path(session_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = File::open(&path)
            .map_err(|e| format!("Open session {}: {}", path.display(), e))?;
        let reader = BufReader::new(file);
        let mut messages = Vec::new();
        for (i, line_res) in reader.lines().enumerate() {
            let line = line_res.map_err(|e| format!("Read line {}: {}", i + 1, e))?;
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let msg: SessionMessage =
                serde_json::from_str(trimmed).map_err(|e| format!("Parse line {}: {}", i + 1, e))?;
            messages.push(msg);
        }
        Ok(messages)
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionSummary>, String> {
        let mut summaries = Vec::new();
        let entries = fs::read_dir(&self.sessions_dir)
            .map_err(|e| format!("Read sessions dir: {}", e))?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "jsonl") {
                continue;
            }

            let stem = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown");
            let name = stem.to_string();

            let (msg_count, created_at, updated_at) = self.scan_summary(&name)?;

            summaries.push(SessionSummary {
                id: name.clone(),
                name,
                message_count: msg_count,
                created_at,
                updated_at,
            });
        }

        summaries.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
        Ok(summaries)
    }

    /// Scan only first and last line of a session file for metadata.
    fn scan_summary(&self, session_id: &str) -> Result<(usize, String, String), String> {
        let path = self.file_path(session_id);
        if !path.exists() {
            return Ok((0, ohmywu_domain::chrono_now(), ohmywu_domain::chrono_now()));
        }
        let file = File::open(&path).map_err(|e| format!("Open: {}", e))?;
        let reader = BufReader::new(file);
        let mut count = 0usize;
        let mut first_ts: Option<String> = None;
        let mut last_ts: Option<String> = None;

        for line_res in reader.lines() {
            let line = line_res.map_err(|e| format!("Read: {}", e))?;
            if line.trim().is_empty() {
                continue;
            }
            count += 1;
            // extract just the timestamp field with a simple string search
            if let Some(ts) = Self::extract_timestamp(&line) {
                if first_ts.is_none() {
                    first_ts = Some(ts.clone());
                }
                last_ts = Some(ts);
            }
        }

        let now = ohmywu_domain::chrono_now();
        Ok((
            count,
            first_ts.unwrap_or_else(|| now.clone()),
            last_ts.unwrap_or(now),
        ))
    }

    fn extract_timestamp(line: &str) -> Option<String> {
        // quick parse of the "timestamp":"..." field without full serde
        let marker = "\"timestamp\":\"";
        let start = line.find(marker)?;
        let value_start = start + marker.len();
        let end = line[value_start..].find('"')?;
        Some(line[value_start..value_start + end].to_string())
    }

    pub fn delete_session(&self, session_id: &str) -> Result<(), String> {
        let path = self.file_path(session_id);
        if path.exists() {
            fs::remove_file(&path)
                .map_err(|e| format!("Delete session {}: {}", path.display(), e))?;
        }
        Ok(())
    }

    // ── helpers ───────────────────────────────────────────────────

    fn next_counter_for_date(&self, date: &str) -> Result<u32, String> {
        let prefix = format!("session-{}-", date);
        let mut max_counter: u32 = 0;
        let entries = fs::read_dir(&self.sessions_dir)
            .map_err(|e| format!("Read sessions dir: {}", e))?;

        for entry in entries {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let fname = entry.file_name();
            let name = fname.to_string_lossy();
            if name.starts_with(&prefix) && name.ends_with(".jsonl") {
                let num_str = &name[prefix.len()..name.len() - 5];
                if let Ok(n) = num_str.parse::<u32>()
                    && n > max_counter
                {
                    max_counter = n;
                }
            }
        }

        Ok(max_counter + 1)
    }
}
