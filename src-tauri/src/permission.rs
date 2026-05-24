use serde::{Deserialize, Serialize};

use ohmywu_domain::AgentMode;

use crate::tools::ToolKind;

/// Permission configuration — Claude Code style allow/deny rules.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionConfig {
    /// Rules ordered: deny wins over allow regardless of order.
    #[serde(default)]
    pub rules: Vec<PermissionRule>,
}

/// A single allow/deny rule with optional parameter pattern.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRule {
    /// "allow" or "deny"
    pub effect: String,
    /// Tool pattern: e.g. "bash", "bash(rm *)", "write(/etc/*)"
    pub tool: String,
}

/// Result of a permission check.
#[derive(Debug)]
pub enum PermissionCheck {
    /// Allowed to execute.
    Allowed,
    /// Explicitly denied by a rule.
    Denied(String),
    /// Needs user confirmation (HighRisk in default mode).
    NeedsConfirm(String),
}

/// Check if a tool call is allowed by the permission rules.
pub fn check_permission(
    config: &PermissionConfig,
    tool_name: &str,
    params: &serde_json::Value,
    tool_kind: Option<ToolKind>,
    agent_mode: AgentMode,
) -> PermissionCheck {
    // Collect params into a flat string for pattern matching
    let param_str = params_as_string(tool_name, params);
    let param_str = param_str.as_str();

    // Phase 1: check deny rules — deny always wins
    for rule in &config.rules {
        if rule.effect != "deny" {
            continue;
        }
        if rule_matches(&rule.tool, tool_name, param_str) {
            return PermissionCheck::Denied(format!(
                "权限拒绝: {} {} (匹配规则 '{}')",
                tool_name, param_str, rule.tool
            ));
        }
    }

    // Phase 2: check allow rules — do any allow this?
    let has_allow_rules = config.rules.iter().any(|r| r.effect == "allow");

    if has_allow_rules {
        // There are explicit allow rules — tool must match one
        let allowed = config.rules.iter().any(|r| {
            r.effect == "allow" && rule_matches(&r.tool, tool_name, param_str)
        });

        if !allowed {
            return PermissionCheck::Denied(format!(
                "权限拒绝: {} {} 不在允许列表中", tool_name, param_str
            ));
        }
    }

    // Phase 3: risk-based check (no rules, or allowed by rules)
    match (agent_mode, tool_kind) {
        (AgentMode::Auto, Some(ToolKind::HighRisk)) => PermissionCheck::Allowed,
        (_, Some(ToolKind::HighRisk)) => PermissionCheck::NeedsConfirm(format!(
            "需要确认: 执行 {} {}?", tool_name, param_str
        )),
        _ => PermissionCheck::Allowed,
    }
}

/// Extract the primary parameter value for pattern matching.
/// e.g. bash → "rm -rf /", write → "/etc/passwd"
fn params_as_string(tool_name: &str, params: &serde_json::Value) -> String {
    match tool_name {
        "bash" => params.get("command").and_then(|v| v.as_str()),
        "write" => params.get("path").and_then(|v| v.as_str()),
        "edit" => params.get("file_path").and_then(|v| v.as_str()),
        "read" => params.get("path").and_then(|v| v.as_str()),
        "artifact_read" => params
            .get("artifactId")
            .and_then(|v| v.as_str())
            .or_else(|| params.get("path").and_then(|v| v.as_str())),
        "glob" => params.get("pattern").and_then(|v| v.as_str()),
        "grep" => params.get("pattern").and_then(|v| v.as_str()),
        "web_fetch" => params.get("url").and_then(|v| v.as_str()),
        "capability_register" => params.get("name").and_then(|v| v.as_str()),
        "action_register" => params.get("id").and_then(|v| v.as_str()),
        "agent_delegate" => params.get("targetAgentId").and_then(|v| v.as_str()),
        "agent_register" => params.get("id").and_then(|v| v.as_str()),
        _ => None,
    }
    .unwrap_or("")
    .to_string()
}

/// Check if a rule pattern matches a tool call.
/// Pattern formats:
///   "bash"                — matches any bash call
///   "bash(rm *)"          — matches bash where param starts with "rm"
///   "write(/etc/*)"       — matches write where path starts with "/etc"
fn rule_matches(pattern: &str, tool_name: &str, param_str: &str) -> bool {
    // Extract tool name and optional param pattern: "bash(rm *)" → ("bash", Some("rm *"))
    let (rule_tool, param_pattern) = match pattern.split_once('(') {
        Some((name, rest)) => {
            let rest = rest.trim_end_matches(')');
            (name, Some(rest))
        }
        None => (pattern, None),
    };

    // Tool name must match first
    if !simple_match(rule_tool, tool_name) {
        return false;
    }

    // If no param pattern, any call to this tool matches
    let Some(pp) = param_pattern else {
        return true;
    };

    // Check if param string matches the pattern
    simple_match(pp, param_str)
}

/// Simple glob-like matching: `*` matches any sequence, `?` matches single char.
fn simple_match(pattern: &str, text: &str) -> bool {
    if pattern == "*" {
        return true;
    }

    let p_chars: Vec<char> = pattern.chars().collect();
    let t_chars: Vec<char> = text.chars().collect();
    let (pl, tl) = (p_chars.len(), t_chars.len());

    // DP table for wildcard matching
    let mut dp = vec![vec![false; tl + 1]; pl + 1];
    dp[0][0] = true;

    for i in 1..=pl {
        if p_chars[i - 1] == '*' {
            dp[i][0] = dp[i - 1][0];
        }
    }

    for i in 1..=pl {
        for j in 1..=tl {
            if p_chars[i - 1] == '*' {
                dp[i][j] = dp[i - 1][j] || dp[i][j - 1];
            } else if p_chars[i - 1] == '?' || p_chars[i - 1] == t_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1];
            }
        }
    }

    dp[pl][tl]
}

#[cfg(test)]
mod tests {
    use super::*;
    use ohmywu_domain::AgentMode;

    #[test]
    fn test_simple_match() {
        assert!(simple_match("bash", "bash"));
        assert!(simple_match("bash*", "bash"));
        assert!(simple_match("rm *", "rm -rf /"));
        assert!(!simple_match("rm *", "ls"));
        assert!(simple_match("/etc/*", "/etc/nginx.conf"));
        assert!(!simple_match("/etc/*", "/home/user/file.txt"));
        assert!(simple_match("*", "anything"));
        assert!(simple_match("ls", "ls"));
    }

    #[test]
    fn test_rule_matches() {
        assert!(rule_matches("bash", "bash", "ls -la"));
        assert!(rule_matches("bash(rm *)", "bash", "rm -rf /tmp"));
        assert!(!rule_matches("bash(rm *)", "bash", "ls -la"));
        assert!(rule_matches("write(/etc/*)", "write", "/etc/passwd"));
        assert!(!rule_matches("write(/etc/*)", "write", "/home/x.txt"));
    }

    #[test]
    fn test_check_permission_deny_wins() {
        let config = PermissionConfig {
            rules: vec![PermissionRule {
                effect: "deny".into(),
                tool: "bash(rm *)".into(),
            }],
        };
        let params = serde_json::json!({"command": "rm -rf /"});
        let result = check_permission(&config, "bash", &params, Some(ToolKind::HighRisk), AgentMode::Agent);
        assert!(matches!(result, PermissionCheck::Denied(_)));
    }

    #[test]
    fn test_check_permission_allow_with_rules() {
        let config = PermissionConfig {
            rules: vec![
                PermissionRule {
                    effect: "allow".into(),
                    tool: "bash(ls *)".into(),
                },
                PermissionRule {
                    effect: "deny".into(),
                    tool: "bash(rm *)".into(),
                },
            ],
        };
        let params = serde_json::json!({"command": "ls -la"});
        let result = check_permission(&config, "bash", &params, Some(ToolKind::HighRisk), AgentMode::Agent);
        // ls is allowed by allow rules → confirm (needs user ok since it's HighRisk)
        assert!(matches!(result, PermissionCheck::NeedsConfirm(_)));
    }

    #[test]
    fn test_check_permission_readonly_allowed() {
        let config = PermissionConfig::default();
        let params = serde_json::json!({"path": "/etc/hosts"});
        let result = check_permission(&config, "read", &params, Some(ToolKind::ReadOnly), AgentMode::Agent);
        assert!(matches!(result, PermissionCheck::Allowed));
    }

    #[test]
    fn test_empty_allow_deny_all_by_default() {
        let config = PermissionConfig::default();
        // No rules → default behavior: ReadOnly allowed, HighRisk needs confirm
        let r_params = serde_json::json!({"path": "/tmp"});
        assert!(matches!(
            check_permission(&config, "read", &r_params, Some(ToolKind::ReadOnly), AgentMode::Agent),
            PermissionCheck::Allowed
        ));
        let b_params = serde_json::json!({"command": "ls"});
        assert!(matches!(
            check_permission(&config, "bash", &b_params, Some(ToolKind::HighRisk), AgentMode::Agent),
            PermissionCheck::NeedsConfirm(_)
        ));
    }
}
