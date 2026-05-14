use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let command = params
        .get("command")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'command' parameter for bash".to_string())?;

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .current_dir(dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("/tmp")))
        .output()
        .map_err(|e| format!("Bash execution failed: {}", e))?;

    Ok(ExecOutput {
        output: Some(String::from_utf8_lossy(&output.stdout).into_owned()),
        stderr: Some(String::from_utf8_lossy(&output.stderr).into_owned()),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
