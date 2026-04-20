use ohmywu_domain::{
    ActionSpec, ActionTarget, AgentLifecycleState, AgentTemplate, RiskLevel, ToolSpec,
    WorkflowSpec,
};

pub fn system_management_action_specs() -> Vec<ActionSpec> {
    vec![
        ActionSpec {
            name: "system_overview".to_string(),
            title: "System Overview".to_string(),
            summary: "Inspect the current Linux machine overview.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ReadOnly,
            target: ActionTarget::Tool("get_system_overview".to_string()),
        },
        ActionSpec {
            name: "list_processes".to_string(),
            title: "List Processes".to_string(),
            summary: "Read the current process list.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ReadOnly,
            target: ActionTarget::Tool("list_processes".to_string()),
        },
        ActionSpec {
            name: "scan_storage".to_string(),
            title: "Scan Storage".to_string(),
            summary: "Run a storage scan workflow and return the summary.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ReadOnly,
            target: ActionTarget::Workflow("scan_storage".to_string()),
        },
        ActionSpec {
            name: "list_services".to_string(),
            title: "List Services".to_string(),
            summary: "Read the current list of systemd services.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ReadOnly,
            target: ActionTarget::Tool("list_services".to_string()),
        },
        ActionSpec {
            name: "read_journal".to_string(),
            title: "Read Journal".to_string(),
            summary: "Read recent systemd journal entries.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ReadOnly,
            target: ActionTarget::Tool("read_journal".to_string()),
        },
        // ── M3: Write Operations ───────────────────────────────────────────────
        ActionSpec {
            name: "kill_process".to_string(),
            title: "Kill Process".to_string(),
            summary: "Terminate a process by PID.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::HighRisk,
            target: ActionTarget::Tool("kill_process".to_string()),
        },
        ActionSpec {
            name: "start_service".to_string(),
            title: "Start Service".to_string(),
            summary: "Start a systemd service.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ControlledWrite,
            target: ActionTarget::Tool("start_service".to_string()),
        },
        ActionSpec {
            name: "stop_service".to_string(),
            title: "Stop Service".to_string(),
            summary: "Stop a systemd service.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ControlledWrite,
            target: ActionTarget::Tool("stop_service".to_string()),
        },
        ActionSpec {
            name: "restart_service".to_string(),
            title: "Restart Service".to_string(),
            summary: "Restart a systemd service.".to_string(),
            domain: "system-management".to_string(),
            risk_level: RiskLevel::ControlledWrite,
            target: ActionTarget::Tool("restart_service".to_string()),
        },
    ]
}

pub fn system_management_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "get_system_overview".to_string(),
            title: "Get System Overview".to_string(),
            summary: "Read CPU, memory, disk and uptime summaries.".to_string(),
            risk_level: RiskLevel::ReadOnly,
        },
        ToolSpec {
            name: "list_processes".to_string(),
            title: "List Processes".to_string(),
            summary: "Read the process table.".to_string(),
            risk_level: RiskLevel::ReadOnly,
        },
        ToolSpec {
            name: "list_services".to_string(),
            title: "List Services".to_string(),
            summary: "Read systemd service list.".to_string(),
            risk_level: RiskLevel::ReadOnly,
        },
        ToolSpec {
            name: "scan_storage".to_string(),
            title: "Scan Storage".to_string(),
            summary: "Scan directory sizes using du.".to_string(),
            risk_level: RiskLevel::ReadOnly,
        },
        ToolSpec {
            name: "read_journal".to_string(),
            title: "Read Journal".to_string(),
            summary: "Read systemd journal entries.".to_string(),
            risk_level: RiskLevel::ReadOnly,
        },
        // ── M3: Write Operations ───────────────────────────────────────────────
        ToolSpec {
            name: "kill_process".to_string(),
            title: "Kill Process".to_string(),
            summary: "Terminate a process by PID using kill.".to_string(),
            risk_level: RiskLevel::HighRisk,
        },
        ToolSpec {
            name: "start_service".to_string(),
            title: "Start Service".to_string(),
            summary: "Start a systemd service via systemctl.".to_string(),
            risk_level: RiskLevel::ControlledWrite,
        },
        ToolSpec {
            name: "stop_service".to_string(),
            title: "Stop Service".to_string(),
            summary: "Stop a systemd service via systemctl.".to_string(),
            risk_level: RiskLevel::ControlledWrite,
        },
        ToolSpec {
            name: "restart_service".to_string(),
            title: "Restart Service".to_string(),
            summary: "Restart a systemd service via systemctl.".to_string(),
            risk_level: RiskLevel::ControlledWrite,
        },
    ]
}

pub fn system_management_workflows() -> Vec<WorkflowSpec> {
    vec![WorkflowSpec {
        name: "scan_storage".to_string(),
        title: "Scan Storage".to_string(),
        summary: "Collect directory size and large-file hints for later cleanup flows.".to_string(),
        action_names: vec!["system_overview".to_string(), "list_processes".to_string()],
    }]
}

pub fn system_management_agent_templates() -> Vec<AgentTemplate> {
    vec![
        AgentTemplate {
            name: "system-ops".to_string(),
            title: "System Ops".to_string(),
            summary: "Coordinates system-management actions for Linux diagnostics.".to_string(),
            default_actions: vec![
                "system_overview".to_string(),
                "list_processes".to_string(),
                "scan_storage".to_string(),
            ],
            lifecycle_state: AgentLifecycleState::Active,
        },
        AgentTemplate {
            name: "cleanup-reviewer".to_string(),
            title: "Cleanup Reviewer".to_string(),
            summary: "Reviews cleanup plans before write actions are enabled.".to_string(),
            default_actions: vec!["scan_storage".to_string()],
            lifecycle_state: AgentLifecycleState::Draft,
        },
    ]
}
