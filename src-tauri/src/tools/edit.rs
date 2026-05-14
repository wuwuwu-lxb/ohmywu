use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let file_path = params
        .get("file_path")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'file_path' parameter for edit".to_string())?;
    let old_string = params
        .get("old_string")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'old_string' parameter for edit".to_string())?;
    let new_string = params
        .get("new_string")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'new_string' parameter for edit".to_string())?;

    let content = std::fs::read_to_string(file_path)
        .map_err(|e| format!("Read failed: {}", e))?;

    let count = content.matches(old_string).count();
    if count == 0 {
        return Err(
            "No match found. The text may have changed — read the file first to get current content."
                .to_string(),
        );
    }
    if count > 1 {
        return Err(format!(
            "Found {} matches. Provide more context in old_string for a unique match.",
            count
        ));
    }

    let new_content = content.replace(old_string, new_string);
    std::fs::write(file_path, &new_content)
        .map_err(|e| format!("Write failed: {}", e))?;

    // Compute approximate line range of the change
    let old_line = content.lines().position(|l| l.contains(
        &old_string.chars().take(40).collect::<String>()
    )).map(|l| l + 1).unwrap_or(0);

    Ok(ExecOutput {
        output: Some(format!(
            "Applied edit at {} around line {}. {} bytes written.",
            file_path, old_line, new_content.len()
        )),
        stderr: None,
        exit_code: 0,
    })
}
