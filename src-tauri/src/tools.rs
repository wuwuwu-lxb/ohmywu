use ohmywu_llm_adapter::types::{FunctionDef, ToolDef};

use crate::AppState;

/// Convert registered capabilities into LLM tool definitions.
pub fn capabilities_as_tools(state: &AppState) -> Vec<ToolDef> {
    let mut tools = Vec::new();

    // bash capability
    if state.capabilities.contains("bash") {
        tools.push(ToolDef {
            tool_type: "function".into(),
            function: FunctionDef {
                name: "bash".into(),
                description: "执行一个 shell 命令。在执行可能有破坏性的操作前，向用户确认。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "command": {
                            "type": "string",
                            "description": "要执行的 shell 命令"
                        }
                    },
                    "required": ["command"]
                }),
            },
        });
    }

    // read capability
    if state.capabilities.contains("read") {
        tools.push(ToolDef {
            tool_type: "function".into(),
            function: FunctionDef {
                name: "read".into(),
                description: "从文件系统读取文件内容。不会修改任何文件。".into(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": {
                            "type": "string",
                            "description": "要读取的文件路径"
                        }
                    },
                    "required": ["path"]
                }),
            },
        });
    }

    // Note: Actions are not yet exposed as tools.
    // The Action pipeline (Phase 3) will add proper action→tool conversion
    // once each Action has a README.md + optional script. For now, only
    // the two atomic capabilities (bash + read) are exposed.

    tools
}
