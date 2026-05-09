use std::sync::Arc;

use ohmywu_action_registry::ActionRegistry;
use ohmywu_audit::AuditLog;
use ohmywu_capability_registry::CapabilityRegistry;
use ohmywu_domain::*;
use ohmywu_policy_engine::PolicyEngine;
use ohmywu_task_engine::TaskEngine;
use tauri::Manager;

pub struct AppState {
    pub capabilities: Arc<CapabilityRegistry>,
    pub actions: Arc<ActionRegistry>,
    pub policy: Arc<PolicyEngine>,
    pub tasks: Arc<TaskEngine>,
    pub audit: Arc<AuditLog>,
}

impl AppState {
    pub fn new() -> Self {
        let capabilities = Arc::new(CapabilityRegistry::new());
        let actions = Arc::new(ActionRegistry::new());
        let policy = Arc::new(PolicyEngine::new());
        let tasks = Arc::new(TaskEngine::new());
        let audit = Arc::new(AuditLog::new());

        // register initial capabilities
        register_capabilities(&capabilities);
        // register initial actions
        register_actions(&actions);

        Self {
            capabilities,
            actions,
            policy,
            tasks,
            audit,
        }
    }
}

fn register_capabilities(registry: &CapabilityRegistry) {
    registry.register(Capability::new(
        "bash",
        "Execute a shell command. Subject to policy control.",
        RiskLevel::HighRisk,
    ));
    registry.register(Capability::new(
        "read",
        "Read file contents from the filesystem.",
        RiskLevel::ReadOnly,
    ));
}

fn register_actions(registry: &ActionRegistry) {
    registry.register(Action::new(
        "shell.exec",
        "Execute a shell command",
    ));
    registry.register(Action::new(
        "fs.read",
        "Read a file",
    ));
    registry.register(Action::new(
        "system.info",
        "Get system information",
    ));
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tauri Commands ──────────────────────────────────────────────────

#[tauri::command]
fn get_capabilities(state: tauri::State<AppState>) -> Vec<Capability> {
    state.capabilities.list()
}

#[tauri::command]
fn get_actions(state: tauri::State<AppState>) -> Vec<Action> {
    state.actions.list()
}

#[tauri::command]
fn get_policy_mode(state: tauri::State<AppState>) -> PolicyMode {
    state.policy.current_mode()
}

#[tauri::command]
fn get_tasks(state: tauri::State<AppState>) -> Vec<Task> {
    state.tasks.list()
}

#[tauri::command]
fn get_audits(state: tauri::State<AppState>) -> Vec<AuditEvent> {
    state.audit.list(100)
}

// ── Tauri App Entry ──────────────────────────────────────────────────

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let state = AppState::new();
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_capabilities,
            get_actions,
            get_policy_mode,
            get_tasks,
            get_audits,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
