use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'path' parameter for read".to_string())?;

    let content = std::fs::read_to_string(path).map_err(|e| format!("Read failed: {}", e))?;

    Ok(ExecOutput {
        output: Some(content),
        stderr: None,
        exit_code: 0,
    })
}
