use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let thought = params
        .get("thought")
        .and_then(|v| v.as_str())
        .unwrap_or("(no thought recorded)");

    Ok(ExecOutput {
        output: Some(format!("[Thinking] {}", thought)),
        stderr: None,
        exit_code: 0,
    })
}
