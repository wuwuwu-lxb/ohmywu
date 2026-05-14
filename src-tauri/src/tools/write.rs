use std::path::Path;

use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'path' parameter for write".to_string())?;
    let content = params
        .get("content")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'content' parameter for write".to_string())?;

    let p = Path::new(path);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create parent directories: {}", e))?;
    }

    std::fs::write(p, content)
        .map_err(|e| format!("Write failed: {}", e))?;

    Ok(ExecOutput {
        output: Some(format!("Written {} bytes to {}", content.len(), path)),
        stderr: None,
        exit_code: 0,
    })
}
