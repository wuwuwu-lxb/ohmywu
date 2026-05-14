use std::path::Path;

use crate::tools::ExecOutput;

pub fn execute(params: &serde_json::Value) -> Result<ExecOutput, String> {
    let pattern = params
        .get("pattern")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing 'pattern' parameter for glob".to_string())?;
    let root = params
        .get("path")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            dirs::home_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| ".".to_string())
        });

    let root_path = Path::new(&root);
    if !root_path.exists() {
        return Err(format!("Path does not exist: {}", root));
    }
    if !root_path.is_dir() {
        return Err(format!("Not a directory: {}", root));
    }

    let max_results: usize = 50;
    let mut results: Vec<String> = Vec::new();

    // Use glob pattern matching via walkdir + glob matching
    let glob_pattern = glob::Pattern::new(pattern)
        .map_err(|e| format!("Invalid glob pattern: {}", e))?;

    for entry in walkdir::WalkDir::new(root_path)
        .max_depth(8)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if results.len() >= max_results {
            break;
        }
        let relative = entry
            .path()
            .strip_prefix(root_path)
            .unwrap_or(entry.path());
        if glob_pattern.matches_path(relative) {
            results.push(entry.path().to_string_lossy().to_string());
        }
    }

    if results.is_empty() {
        return Ok(ExecOutput {
            output: Some(format!("No files matching '{}' in {}", pattern, root)),
            stderr: None,
            exit_code: 0,
        });
    }

    let output = format!(
        "Found {} result(s) for '{}' in {}:\n{}",
        results.len(),
        pattern,
        root,
        results.join("\n")
    );

    Ok(ExecOutput {
        output: Some(output),
        stderr: None,
        exit_code: 0,
    })
}
