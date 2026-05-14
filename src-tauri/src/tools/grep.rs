use std::process::Command;

use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let pattern = params
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'pattern' parameter for grep".to_string())?;
    let path = params
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or(".");
    let include = params.get("include").and_then(|v| v.as_str());

    let mut cmd = Command::new("grep");
    cmd.arg("-rn")  // recursive, line numbers
        .arg("--max-count=30")  // max matches per file
        .arg("-e")
        .arg(pattern)
        .arg(path);

    if let Some(ext) = include {
        cmd.arg("--include").arg(ext);
    }

    let output = cmd.output().map_err(|e| format!("Grep failed: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let exit_code = output.status.code().unwrap_or(-1);

    // grep returns exit code 1 when no match — not an error
    if exit_code == 1 {
        return Ok(ExecOutput {
            output: Some(format!("No matches for '{}' in {}", pattern, path)),
            stderr: None,
            exit_code: 0,
        });
    }

    if exit_code != 0 {
        return Err(format!("Grep failed: {}", stderr));
    }

    // Limit output lines to prevent huge responses
    let mut lines: Vec<&str> = stdout.lines().collect();
    let total = lines.len();
    if total > 50 {
        lines.truncate(50);
    }

    let result = lines.join("\n");
    let mut output = result;
    if total > 50 {
        output.push_str(&format!("\n... [{} more matches]", total - 50));
    }

    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}
