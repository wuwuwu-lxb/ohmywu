use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::State,
    http::Method,
    routing::{get, post},
    Json, Router,
};
use ohmywu_action_registry::ActionRegistry;
use ohmywu_agent_kernel::AgentKernel;
use ohmywu_domain::{
    ActionSpec, AgentTemplate, AppSettings, HealthResponse, JournalQuery,
    ReferenceResolutionRequest, ReferenceResolutionResponse, StorageScanQuery, WorkflowSpec,
};
use ohmywu_linux_adapter::LinuxAdapter;
use ohmywu_reference_resolver::resolve_references;
use ohmywu_store::InMemoryStore;
use ohmywu_toolkit_system_management::{
    system_management_action_specs, system_management_agent_templates,
    system_management_workflows,
};
use ohmywu_workflow_registry::WorkflowRegistry;
use tower_http::cors::{Any, CorsLayer};

#[derive(Clone)]
struct AppState {
    actions: Arc<ActionRegistry>,
    workflows: Arc<WorkflowRegistry>,
    agents: Arc<AgentKernel>,
    store: Arc<InMemoryStore>,
    linux_adapter: Arc<LinuxAdapter>,
}

#[tokio::main]
async fn main() {
    let actions = Arc::new(ActionRegistry::from_specs(system_management_action_specs()));
    let workflows = Arc::new(WorkflowRegistry::from_specs(system_management_workflows()));
    let agents = Arc::new(AgentKernel::from_templates(system_management_agent_templates()));
    let store = Arc::new(InMemoryStore::new(AppSettings::default()));
    let linux_adapter = Arc::new(LinuxAdapter::default());

    let app_state = AppState {
        actions,
        workflows,
        agents,
        store,
        linux_adapter,
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
        .and_then(|s| s.split_whitespace().next().and_then(|v| v.parse::<f64>().ok()))
        .unwrap_or(0.0);
    let uptime_days = uptime_secs / 86400.0;
    let uptime_hours = (uptime_secs % 86400.0) / 3600.0;
    let uptime = format!("{:.1} 天 · {:.1} 小时", uptime_days, uptime_hours);

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
