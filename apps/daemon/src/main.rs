use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::Method,
    routing::{get, post},
    Json, Router,
};
use ohmywu_action_registry::ActionRegistry;
use ohmywu_agent_kernel::AgentKernel;
use ohmywu_audit::AuditLog;
use ohmywu_domain::{
    ActionSpec, AgentTemplate, AppSettings, CleanupExecuteQuery, CleanupPreviewQuery,
    CleanupPreviewResult, CleanupScanPathQuery, CleanupTreeResult, HealthResponse, JournalQuery,
    ReferenceResolutionRequest, ReferenceResolutionResponse, RiskLevel, StorageScanQuery,
    WorkflowSpec,
};
use ohmywu_linux_adapter::LinuxAdapter;
use ohmywu_reference_resolver::resolve_references;
use ohmywu_store::InMemoryStore;
use ohmywu_task_engine::TaskEngine;
use ohmywu_toolkit_system_management::{
    system_management_action_specs, system_management_agent_templates, system_management_workflows,
};
use ohmywu_workflow_registry::WorkflowRegistry;
use serde::Deserialize;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    actions: Arc<ActionRegistry>,
    workflows: Arc<WorkflowRegistry>,
    agents: Arc<AgentKernel>,
    store: Arc<InMemoryStore>,
    linux_adapter: Arc<LinuxAdapter>,
    task_engine: Arc<TaskEngine>,
    audit_log: Arc<AuditLog>,
}

#[tokio::main]
async fn main() {
    let actions = Arc::new(ActionRegistry::from_specs(system_management_action_specs()));
    let workflows = Arc::new(WorkflowRegistry::from_specs(system_management_workflows()));
    let agents = Arc::new(AgentKernel::from_templates(
        system_management_agent_templates(),
    ));
    let store = Arc::new(InMemoryStore::new(AppSettings::default()));
    let linux_adapter = Arc::new(LinuxAdapter::default());
    let task_engine = Arc::new(TaskEngine::default());
    let audit_log = Arc::new(AuditLog::default());

    let app_state = AppState {
        actions,
        workflows,
        agents,
        store,
        linux_adapter,
        task_engine,
        audit_log,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST, Method::PUT])
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/health", get(health))
        .route("/api/actions", get(list_actions))
        .route("/api/workflows", get(list_workflows))
        .route("/api/agents/templates", get(list_agent_templates))
        .route("/api/references/resolve", post(resolve_reference_input))
        .route("/api/settings", get(get_settings))
        .route("/api/system/overview", get(system_overview))
        .route("/api/processes", get(list_processes))
        .route("/api/services", get(list_services))
        .route("/api/storage/scans", post(scan_storage))
        .route("/api/logs/journal", post(read_journal))
        .route("/api/tasks", get(list_tasks))
        .route("/api/tasks/:id", get(get_task))
        .route("/api/audits", get(list_audits))
        // ── M3: Write Operation Routes ───────────────────────────────────────
        .route("/api/processes/:pid/kill", post(kill_process))
        .route("/api/services/:name/start", post(start_service))
        .route("/api/services/:name/stop", post(stop_service))
        .route("/api/services/:name/restart", post(restart_service))
        // ── M3: Storage Cleanup Routes ────────────────────────────────────────
        .route("/api/cleanup/preview", post(cleanup_preview))
        .route("/api/cleanup/tree", post(cleanup_tree))
        .route("/api/cleanup/scan-path", post(cleanup_scan_path))
        .route("/api/cleanup/execute", post(cleanup_execute))
        .with_state(app_state)
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("ohmywu daemon listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("failed to bind daemon port");

    axum::serve(listener, app)
        .await
        .expect("failed to run daemon");
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        app: "ohmywu-daemon".to_string(),
        status: "ok".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

async fn list_actions(State(state): State<AppState>) -> Json<Vec<ActionSpec>> {
    Json(state.actions.list())
}

async fn list_workflows(State(state): State<AppState>) -> Json<Vec<WorkflowSpec>> {
    Json(state.workflows.list())
}

async fn list_agent_templates(State(state): State<AppState>) -> Json<Vec<AgentTemplate>> {
    Json(state.agents.list_templates())
}

async fn resolve_reference_input(
    State(state): State<AppState>,
    Json(request): Json<ReferenceResolutionRequest>,
) -> Json<ReferenceResolutionResponse> {
    let mut references = resolve_references(&request.input);

    for reference in &mut references {
        reference.exists = match reference.kind.as_str() {
            "action" => state.actions.contains(&reference.name),
            "agent" => state.agents.contains_template(&reference.name),
            _ => false,
        };
    }

    Json(ReferenceResolutionResponse {
        input: request.input,
        references,
    })
}

async fn get_settings(State(state): State<AppState>) -> Json<AppSettings> {
    Json(state.store.settings())
}

async fn system_overview() -> Json<serde_json::Value> {
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let uptime_secs = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| {
            s.split_whitespace()
                .next()
                .and_then(|v| v.parse::<f64>().ok())
        })
        .unwrap_or(0.0);
    let uptime_days = uptime_secs / 86400.0;
    let uptime_hours = (uptime_secs % 86400.0) / 3600.0;
    let uptime = format!("{:.1} days . {:.1} hours", uptime_days, uptime_hours);

    let cpu_model = std::fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|content| {
            content
                .lines()
                .find(|l| l.starts_with("model name"))
                .map(|l| l.split(':').nth(1).map(|s| s.trim().to_string()))
                .flatten()
        })
        .unwrap_or_else(|| "unknown".to_string());

    let cpu_cores = std::thread::available_parallelism()
        .map(|n| n.get().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let loadavg = std::fs::read_to_string("/proc/loadavg")
        .ok()
        .map(|s| s.split_whitespace().take(3).collect::<Vec<_>>().join(" / "))
        .unwrap_or_else(|| "N/A".to_string());

    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    let mem_total = meminfo
        .lines()
        .find(|l| l.starts_with("MemTotal:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v / 1024)
        .unwrap_or(0);
    let mem_available = meminfo
        .lines()
        .find(|l| l.starts_with("MemAvailable:"))
        .and_then(|l| l.split_whitespace().nth(1))
        .and_then(|v| v.parse::<u64>().ok())
        .map(|v| v / 1024)
        .unwrap_or(0);
    let mem_used = mem_total.saturating_sub(mem_available);
    let memory_total = format!("{} MB", mem_total);
    let memory = format!("{} MB", mem_used);
    let memory_detail = format!("{}/{}", mem_used, mem_total);

    let os = std::fs::read_to_string("/etc/os-release")
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("PRETTY_NAME="))
                .map(|l| l.split('=').nth(1).map(|v| v.trim_matches('"').to_string()))
                .flatten()
        })
        .unwrap_or_else(|| "Linux".to_string());

    let kernel = std::fs::read_to_string("/proc/sys/kernel/osrelease")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    Json(serde_json::json!({
        "hostname": hostname,
        "os": os,
        "kernel": kernel,
        "uptime": uptime,
        "cpu_model": cpu_model,
        "cpu_cores": cpu_cores,
        "loadavg": loadavg,
        "memory_total": memory_total,
        "memory": memory,
        "memory_detail": memory_detail,
        "disk": "N/A",
    }))
}

async fn list_processes(State(state): State<AppState>) -> Json<Vec<ohmywu_domain::ProcessInfo>> {
    Json(state.linux_adapter.list_processes())
}

async fn list_services(State(state): State<AppState>) -> Json<Vec<ohmywu_domain::ServiceInfo>> {
    Json(state.linux_adapter.list_services().await)
}

async fn scan_storage(
    State(state): State<AppState>,
    Json(query): Json<StorageScanQuery>,
) -> Json<ohmywu_domain::StorageScanResult> {
    Json(state.linux_adapter.scan_storage(query).await)
}

async fn read_journal(
    State(state): State<AppState>,
    Json(query): Json<JournalQuery>,
) -> Json<Vec<ohmywu_domain::JournalEntry>> {
    Json(state.linux_adapter.read_journal(query).await)
}

async fn list_tasks(State(state): State<AppState>) -> Json<Vec<ohmywu_domain::Task>> {
    Json(state.task_engine.list_tasks())
}

async fn get_task(
    axum::extract::Path(id): axum::extract::Path<String>,
    State(state): State<AppState>,
) -> Json<Option<ohmywu_domain::Task>> {
    Json(state.task_engine.get_task(&id))
}

async fn list_audits(
    axum::extract::Query(limit): axum::extract::Query<AuditQuery>,
    State(state): State<AppState>,
) -> Json<Vec<ohmywu_domain::AuditEvent>> {
    let limit = limit.limit.unwrap_or(100);
    Json(state.audit_log.list(limit))
}

#[derive(Debug, Deserialize)]
struct AuditQuery {
    limit: Option<usize>,
}

// ── M3: Write Operation Handlers ─────────────────────────────────────────────

async fn kill_process(
    State(state): State<AppState>,
    axum::extract::Path(pid): axum::extract::Path<u64>,
) -> Result<Json<serde_json::Value>, String> {
    let task_id = state.task_engine.new_task("kill_process", &pid.to_string());
    let result = state.linux_adapter.kill_process(pid).await;
    match result {
        Ok(()) => {
            state.task_engine.complete(&task_id, "success");
            state.audit_log.record(
                "user",
                "kill_process",
                &pid.to_string(),
                RiskLevel::HighRisk,
                "success",
                None,
            );
            Ok(Json(serde_json::json!({ "success": true, "pid": pid })))
        }
        Err(e) => {
            state.task_engine.fail(&task_id, &e);
            state.audit_log.record(
                "user",
                "kill_process",
                &pid.to_string(),
                RiskLevel::HighRisk,
                "failure",
                Some(&e),
            );
            Err(e)
        }
    }
}

async fn start_service(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, String> {
    let task_id = state.task_engine.new_task("start_service", &name);
    let result = state.linux_adapter.start_service(&name).await;
    match result {
        Ok(()) => {
            state.task_engine.complete(&task_id, "success");
            state.audit_log.record(
                "user",
                "start_service",
                &name,
                RiskLevel::ControlledWrite,
                "success",
                None,
            );
            Ok(Json(
                serde_json::json!({ "success": true, "service": name }),
            ))
        }
        Err(e) => {
            state.task_engine.fail(&task_id, &e);
            state.audit_log.record(
                "user",
                "start_service",
                &name,
                RiskLevel::ControlledWrite,
                "failure",
                Some(&e),
            );
            Err(e)
        }
    }
}

async fn stop_service(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, String> {
    let task_id = state.task_engine.new_task("stop_service", &name);
    let result = state.linux_adapter.stop_service(&name).await;
    match result {
        Ok(()) => {
            state.task_engine.complete(&task_id, "success");
            state.audit_log.record(
                "user",
                "stop_service",
                &name,
                RiskLevel::ControlledWrite,
                "success",
                None,
            );
            Ok(Json(
                serde_json::json!({ "success": true, "service": name }),
            ))
        }
        Err(e) => {
            state.task_engine.fail(&task_id, &e);
            state.audit_log.record(
                "user",
                "stop_service",
                &name,
                RiskLevel::ControlledWrite,
                "failure",
                Some(&e),
            );
            Err(e)
        }
    }
}

async fn restart_service(
    State(state): State<AppState>,
    axum::extract::Path(name): axum::extract::Path<String>,
) -> Result<Json<serde_json::Value>, String> {
    let task_id = state.task_engine.new_task("restart_service", &name);
    let result = state.linux_adapter.restart_service(&name).await;
    match result {
        Ok(()) => {
            state.task_engine.complete(&task_id, "success");
            state.audit_log.record(
                "user",
                "restart_service",
                &name,
                RiskLevel::ControlledWrite,
                "success",
                None,
            );
            Ok(Json(
                serde_json::json!({ "success": true, "service": name }),
            ))
        }
        Err(e) => {
            state.task_engine.fail(&task_id, &e);
            state.audit_log.record(
                "user",
                "restart_service",
                &name,
                RiskLevel::ControlledWrite,
                "failure",
                Some(&e),
            );
            Err(e)
        }
    }
}

// ── M3: Storage Cleanup Handlers ─────────────────────────────────────────────

async fn cleanup_preview(
    State(state): State<AppState>,
    Json(query): Json<CleanupPreviewQuery>,
) -> Json<CleanupPreviewResult> {
    let task_id = state
        .task_engine
        .new_task("cleanup_preview", query.path.as_deref().unwrap_or("common"));
    let result = state.linux_adapter.cleanup_preview(query).await;
    state.task_engine.complete(&task_id, "success");
    state.audit_log.record(
        "user",
        "cleanup_preview",
        result.scanned_path.as_str(),
        RiskLevel::ReadOnly,
        "success",
        None,
    );
    Json(result)
}

async fn cleanup_tree(State(state): State<AppState>) -> Json<CleanupTreeResult> {
    let task_id = state.task_engine.new_task("cleanup_tree", "main");
    let result = state.linux_adapter.cleanup_tree().await;
    state.task_engine.complete(&task_id, "success");
    state.audit_log.record(
        "user",
        "cleanup_tree",
        "/",
        RiskLevel::ReadOnly,
        "success",
        None,
    );
    Json(result)
}

async fn cleanup_scan_path(
    State(state): State<AppState>,
    Json(query): Json<CleanupScanPathQuery>,
) -> Json<CleanupTreeResult> {
    let task_id = state.task_engine.new_task("cleanup_scan_path", "main");
    let result = state.linux_adapter.cleanup_scan_path(&query.path).await;
    state.task_engine.complete(&task_id, "success");
    state.audit_log.record(
        "user",
        "cleanup_scan_path",
        query.path.as_str(),
        RiskLevel::ReadOnly,
        "success",
        None,
    );
    Json(result)
}

async fn cleanup_execute(
    State(state): State<AppState>,
    Json(query): Json<CleanupExecuteQuery>,
) -> Json<ohmywu_domain::CleanupExecuteResult> {
    let target = format!("{} paths", query.paths.len());
    let task_id = state.task_engine.new_task("cleanup_execute", &target);
    let result = state.linux_adapter.cleanup_execute(query).await;
    let detail = format!(
        "freed {} bytes, deleted {}, rejected {}",
        result.freed_bytes,
        result.deleted.len(),
        result.rejected.len()
    );
    let status = if result.deleted.is_empty() && !result.rejected.is_empty() {
        "failure"
    } else if result.rejected.is_empty() {
        "success"
    } else {
        "partial_success"
    };

    if status == "failure" {
        state.task_engine.fail(&task_id, &detail);
    } else {
        state.task_engine.complete(&task_id, &detail);
    }
    state.audit_log.record(
        "user",
        "cleanup_execute",
        &target,
        RiskLevel::HighRisk,
        status,
        Some(&detail),
    );
    Json(result)
}
