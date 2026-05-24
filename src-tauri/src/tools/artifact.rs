use std::path::{Path, PathBuf};

use crate::tools::ExecOutput;

const DEFAULT_LIMIT: usize = 6000;
const MAX_LIMIT: usize = 8000;

pub fn execute(params: &serde_json::Value, data_dir: &Path) -> Result<ExecOutput, String> {
    let artifact_dir = data_dir.join("runtime").join("artifacts");
    let artifact_path = resolve_artifact_path(params, &artifact_dir)?;
    let artifact_id = artifact_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("unknown");
    let content = std::fs::read_to_string(&artifact_path)
        .map_err(|e| format!("Read artifact failed: {}", e))?;

    let offset = params
        .get("offset")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    let requested_limit = params
        .get("limit")
        .and_then(|value| value.as_u64())
        .unwrap_or(DEFAULT_LIMIT as u64) as usize;
    let limit = requested_limit.clamp(200, MAX_LIMIT);

    let total_chars = content.chars().count();
    let start = offset.min(total_chars);
    let end = start.saturating_add(limit).min(total_chars);
    let excerpt = content
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect::<String>();
    let has_more = end < total_chars;

    Ok(ExecOutput {
        output: Some(format!(
            "Artifact ID: {}\nPath: {}\nTotal chars: {}\nReturned range: {}..{}\nHas more: {}\nNext offset: {}\n\n{}",
            artifact_id,
            artifact_path.display(),
            total_chars,
            start,
            end,
            has_more,
            end,
            excerpt
        )),
        stderr: None,
        exit_code: 0,
    })
}

fn resolve_artifact_path(params: &serde_json::Value, artifact_dir: &Path) -> Result<PathBuf, String> {
    let canonical_dir = std::fs::canonicalize(artifact_dir)
        .map_err(|e| format!("Canonicalize artifact dir failed: {}", e))?;

    if let Some(id) = params.get("artifactId").and_then(|value| value.as_str()) {
        let artifact_id = id.trim();
        if artifact_id.is_empty() {
            return Err("artifactId 不能为空".into());
        }
        let path = canonical_dir.join(format!("{}.txt", artifact_id));
        if !path.exists() {
            return Err(format!("Artifact '{}' 不存在", artifact_id));
        }
        return Ok(path);
    }

    let raw_path = params
        .get("path")
        .and_then(|value| value.as_str())
        .ok_or_else(|| "artifact_read 需要 artifactId 或 path".to_string())?;
    let canonical_path = std::fs::canonicalize(raw_path)
        .map_err(|e| format!("Canonicalize artifact path failed: {}", e))?;
    if !canonical_path.starts_with(&canonical_dir) {
        return Err("只允许读取 runtime/artifacts 目录中的 artifact".into());
    }
    Ok(canonical_path)
}
