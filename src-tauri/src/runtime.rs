use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use ohmywu_domain::{chrono_now, AgentMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeThread {
    pub id: String,
    pub session_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub last_turn_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeTurn {
    pub id: String,
    pub thread_id: String,
    pub session_id: String,
    pub parent_turn_id: Option<String>,
    pub agent_name: Option<String>,
    #[serde(default)]
    pub delegated: bool,
    pub status: String,
    pub agent_mode: AgentMode,
    pub user_content: String,
    pub assistant_content: Option<String>,
    pub execution_count: usize,
    pub checklist_count: usize,
    pub started_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistItem {
    pub text: String,
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecklistSnapshot {
    pub turn_id: String,
    pub session_id: String,
    pub title: Option<String>,
    pub items: Vec<ChecklistItem>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeEvent {
    pub id: String,
    pub session_id: String,
    pub thread_id: String,
    pub turn_id: Option<String>,
    pub kind: String,
    pub summary: String,
    pub payload: serde_json::Value,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeThreadView {
    pub thread: RuntimeThread,
    pub turns: Vec<RuntimeTurn>,
    pub events: Vec<RuntimeEvent>,
}

pub struct RuntimeStore {
    root: PathBuf,
    write_lock: Mutex<()>,
}

impl RuntimeStore {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        for sub in ["threads", "turns", "events", "checklists"] {
            fs::create_dir_all(root.join(sub))
                .map_err(|e| format!("Create runtime dir {}: {}", sub, e))?;
        }
        Ok(Self {
            root,
            write_lock: Mutex::new(()),
        })
    }

    pub fn start_turn(
        &self,
        session_id: &str,
        agent_mode: AgentMode,
        user_content: &str,
    ) -> Result<RuntimeTurn, String> {
        let mut thread = self.ensure_thread(session_id)?;
        let now = chrono_now();
        let turn = RuntimeTurn {
            id: unique_id("turn"),
            thread_id: thread.id.clone(),
            session_id: session_id.to_string(),
            parent_turn_id: None,
            agent_name: None,
            delegated: false,
            status: "running".into(),
            agent_mode,
            user_content: user_content.to_string(),
            assistant_content: None,
            execution_count: 0,
            checklist_count: 0,
            started_at: now.clone(),
            finished_at: None,
        };
        self.write_turn(&turn)?;

        thread.updated_at = now;
        thread.last_turn_id = Some(turn.id.clone());
        self.write_thread(&thread)?;

        let _ = self.record_event(
            session_id,
            Some(&turn.id),
            "turn.started",
            "开始新回合",
            serde_json::json!({
                "agentMode": turn.agent_mode,
                "userContent": turn.user_content,
            }),
        )?;

        Ok(turn)
    }

    pub fn start_delegated_turn(
        &self,
        session_id: &str,
        parent_turn_id: &str,
        turn_id: &str,
        agent_mode: AgentMode,
        user_content: &str,
        agent_name: &str,
    ) -> Result<RuntimeTurn, String> {
        let mut thread = self.ensure_thread(session_id)?;
        let now = chrono_now();
        let turn = RuntimeTurn {
            id: turn_id.to_string(),
            thread_id: thread.id.clone(),
            session_id: session_id.to_string(),
            parent_turn_id: Some(parent_turn_id.to_string()),
            agent_name: Some(agent_name.to_string()),
            delegated: true,
            status: "running".into(),
            agent_mode,
            user_content: user_content.to_string(),
            assistant_content: None,
            execution_count: 0,
            checklist_count: 0,
            started_at: now.clone(),
            finished_at: None,
        };
        self.write_turn(&turn)?;

        thread.updated_at = now;
        thread.last_turn_id = Some(turn.id.clone());
        self.write_thread(&thread)?;

        let _ = self.record_event(
            session_id,
            Some(&turn.id),
            "turn.started",
            &format!("子 Agent 开始：{}", agent_name),
            serde_json::json!({
                "agentMode": turn.agent_mode,
                "userContent": turn.user_content,
                "agentName": agent_name,
                "parentTurnId": parent_turn_id,
                "delegated": true,
            }),
        )?;

        Ok(turn)
    }

    pub fn finish_turn_completed(
        &self,
        session_id: &str,
        turn_id: &str,
        assistant_content: &str,
        execution_count: usize,
    ) -> Result<RuntimeTurn, String> {
        self.finish_turn_with_status(
            session_id,
            turn_id,
            "completed",
            "turn.completed",
            &format!("回合完成，执行 {} 个工具", execution_count),
            assistant_content,
            execution_count,
        )
    }

    pub fn finish_turn_cancelled(
        &self,
        session_id: &str,
        turn_id: &str,
        assistant_content: &str,
        execution_count: usize,
    ) -> Result<RuntimeTurn, String> {
        self.finish_turn_with_status(
            session_id,
            turn_id,
            "cancelled",
            "turn.cancelled",
            &format!("回合已中断，已执行 {} 个工具", execution_count),
            assistant_content,
            execution_count,
        )
    }

    pub fn finish_turn_failed(
        &self,
        session_id: &str,
        turn_id: &str,
        assistant_content: &str,
        execution_count: usize,
    ) -> Result<RuntimeTurn, String> {
        self.finish_turn_with_status(
            session_id,
            turn_id,
            "failed",
            "turn.failed",
            &format!("回合失败，执行 {} 个工具", execution_count),
            assistant_content,
            execution_count,
        )
    }

    fn finish_turn_with_status(
        &self,
        session_id: &str,
        turn_id: &str,
        status: &str,
        event_kind: &str,
        event_summary: &str,
        assistant_content: &str,
        execution_count: usize,
    ) -> Result<RuntimeTurn, String> {
        let mut turn = self
            .read_turn(turn_id)?
            .ok_or_else(|| format!("Runtime turn not found: {}", turn_id))?;
        turn.status = status.into();
        turn.assistant_content = Some(assistant_content.to_string());
        turn.execution_count = execution_count;
        turn.finished_at = Some(chrono_now());
        self.write_turn(&turn)?;

        let mut thread = self.ensure_thread(session_id)?;
        thread.updated_at = chrono_now();
        thread.last_turn_id = Some(turn.id.clone());
        self.write_thread(&thread)?;

        let _ = self.record_event(
            session_id,
            Some(&turn.id),
            event_kind,
            event_summary,
            serde_json::json!({
                "status": status,
                "assistantContent": assistant_content,
                "executionCount": execution_count,
            }),
        )?;

        Ok(turn)
    }

    pub fn write_checklist(
        &self,
        session_id: &str,
        turn_id: &str,
        title: Option<String>,
        items: Vec<ChecklistItem>,
    ) -> Result<ChecklistSnapshot, String> {
        let mut turn = self
            .read_turn(turn_id)?
            .ok_or_else(|| format!("Runtime turn not found: {}", turn_id))?;
        let snapshot = ChecklistSnapshot {
            turn_id: turn_id.to_string(),
            session_id: session_id.to_string(),
            title,
            items,
            updated_at: chrono_now(),
        };
        let path = self.checklist_path(turn_id);
        write_json(&path, &snapshot)?;

        turn.checklist_count = snapshot.items.len();
        self.write_turn(&turn)?;

        let _ = self.record_event(
            session_id,
            Some(turn_id),
            "checklist.updated",
            &format!("更新 checklist（{} 项）", snapshot.items.len()),
            serde_json::to_value(&snapshot).map_err(|e| format!("Checklist value: {}", e))?,
        )?;

        Ok(snapshot)
    }

    pub fn load_thread_view(&self, session_id: &str) -> Result<Option<RuntimeThreadView>, String> {
        let Some(thread) = self.read_thread(session_id)? else {
            return Ok(None);
        };

        let mut turns = Vec::new();
        for entry in fs::read_dir(self.turns_dir()).map_err(|e| format!("Read turns dir: {}", e))? {
            let entry = entry.map_err(|e| format!("Read turn entry: {}", e))?;
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let turn: RuntimeTurn = read_json(&path)?;
            if turn.session_id == session_id {
                turns.push(turn);
            }
        }
        turns.sort_by(|a, b| a.started_at.cmp(&b.started_at));

        let events = self.read_events(&thread.id)?;
        Ok(Some(RuntimeThreadView { thread, turns, events }))
    }

    pub fn delete_thread(&self, session_id: &str) -> Result<(), String> {
        let Some(thread) = self.read_thread(session_id)? else {
            return Ok(());
        };

        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let _ = fs::remove_file(self.thread_path(session_id));
        let _ = fs::remove_file(self.event_path(&thread.id));

        for entry in fs::read_dir(self.turns_dir()).map_err(|e| format!("Read turns dir: {}", e))? {
            let entry = entry.map_err(|e| format!("Read turn entry: {}", e))?;
            let path = entry.path();
            if path.extension().is_none_or(|ext| ext != "json") {
                continue;
            }
            let turn: RuntimeTurn = read_json(&path)?;
            if turn.session_id == session_id {
                let _ = fs::remove_file(&path);
                let _ = fs::remove_file(self.checklist_path(&turn.id));
            }
        }

        Ok(())
    }

    pub fn record_event(
        &self,
        session_id: &str,
        turn_id: Option<&str>,
        kind: &str,
        summary: &str,
        payload: serde_json::Value,
    ) -> Result<RuntimeEvent, String> {
        let thread = self.ensure_thread(session_id)?;
        let event = RuntimeEvent {
            id: unique_id("evt"),
            session_id: session_id.to_string(),
            thread_id: thread.id,
            turn_id: turn_id.map(|id| id.to_string()),
            kind: kind.to_string(),
            summary: summary.to_string(),
            payload,
            timestamp: chrono_now(),
        };
        self.append_event(event.clone())?;
        Ok(event)
    }

    fn ensure_thread(&self, session_id: &str) -> Result<RuntimeThread, String> {
        if let Some(thread) = self.read_thread(session_id)? {
            return Ok(thread);
        }
        let now = chrono_now();
        let thread = RuntimeThread {
            id: format!("thread-{}", session_id),
            session_id: session_id.to_string(),
            created_at: now.clone(),
            updated_at: now,
            last_turn_id: None,
        };
        self.write_thread(&thread)?;
        Ok(thread)
    }

    fn append_event(&self, event: RuntimeEvent) -> Result<(), String> {
        let _guard = self.write_lock.lock().map_err(|e| format!("Lock: {}", e))?;
        let path = self.event_path(&event.thread_id);
        let line = serde_json::to_string(&event).map_err(|e| format!("Serialize runtime event: {}", e))?;
        let mut file = OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .map_err(|e| format!("Open event log {}: {}", path.display(), e))?;
        writeln!(file, "{}", line).map_err(|e| format!("Write runtime event: {}", e))?;
        Ok(())
    }

    fn read_events(&self, thread_id: &str) -> Result<Vec<RuntimeEvent>, String> {
        let path = self.event_path(thread_id);
        if !path.exists() {
            return Ok(Vec::new());
        }
        let file = OpenOptions::new()
            .read(true)
            .open(&path)
            .map_err(|e| format!("Open runtime events {}: {}", path.display(), e))?;
        let reader = BufReader::new(file);
        let mut events = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line = line.map_err(|e| format!("Read runtime event line {}: {}", index + 1, e))?;
            if line.trim().is_empty() {
                continue;
            }
            let event: RuntimeEvent =
                serde_json::from_str(&line).map_err(|e| format!("Parse runtime event line {}: {}", index + 1, e))?;
            events.push(event);
        }
        Ok(events)
    }

    fn read_thread(&self, session_id: &str) -> Result<Option<RuntimeThread>, String> {
        let path = self.thread_path(session_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }

    fn write_thread(&self, thread: &RuntimeThread) -> Result<(), String> {
        write_json(&self.thread_path(&thread.session_id), thread)
    }

    fn read_turn(&self, turn_id: &str) -> Result<Option<RuntimeTurn>, String> {
        let path = self.turn_path(turn_id);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(read_json(&path)?))
    }

    fn write_turn(&self, turn: &RuntimeTurn) -> Result<(), String> {
        write_json(&self.turn_path(&turn.id), turn)
    }

    fn threads_dir(&self) -> PathBuf {
        self.root.join("threads")
    }

    fn turns_dir(&self) -> PathBuf {
        self.root.join("turns")
    }

    fn events_dir(&self) -> PathBuf {
        self.root.join("events")
    }

    fn checklists_dir(&self) -> PathBuf {
        self.root.join("checklists")
    }

    fn thread_path(&self, session_id: &str) -> PathBuf {
        self.threads_dir().join(format!("{}.json", session_id))
    }

    fn turn_path(&self, turn_id: &str) -> PathBuf {
        self.turns_dir().join(format!("{}.json", turn_id))
    }

    fn event_path(&self, thread_id: &str) -> PathBuf {
        self.events_dir().join(format!("{}.jsonl", thread_id))
    }

    fn checklist_path(&self, turn_id: &str) -> PathBuf {
        self.checklists_dir().join(format!("{}.json", turn_id))
    }
}

fn unique_id(prefix: &str) -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    format!("{}-{}", prefix, millis)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Read json {}: {}", path.display(), e))?;
    serde_json::from_str(&content).map_err(|e| format!("Parse json {}: {}", path.display(), e))
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| format!("Serialize json {}: {}", path.display(), e))?;
    fs::write(path, json).map_err(|e| format!("Write json {}: {}", path.display(), e))
}
