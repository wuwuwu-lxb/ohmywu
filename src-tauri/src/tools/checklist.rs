use crate::runtime::{ChecklistItem, RuntimeStore};

use super::ExecOutput;

pub fn write(params: &serde_json::Value, runtime: &RuntimeStore) -> Result<ExecOutput, String> {
    let session_id = params
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "checklist_write 缺少 session_id".to_string())?;
    let turn_id = params
        .get("turn_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "checklist_write 缺少 turn_id".to_string())?;
    let title = params.get("title").and_then(|v| v.as_str()).map(str::to_string);
    let items = params
        .get("items")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "checklist_write.items 必须是字符串数组".to_string())?;

    let checklist_items = items
        .iter()
        .map(|item| {
            item.as_str()
                .map(|text| ChecklistItem {
                    text: text.to_string(),
                    done: false,
                })
                .ok_or_else(|| "checklist item 必须是字符串".to_string())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let snapshot = runtime.write_checklist(session_id, turn_id, title.clone(), checklist_items)?;
    let label = title.unwrap_or_else(|| "当前计划".into());

    Ok(ExecOutput {
        output: Some(format!("已写入 checklist「{}」，共 {} 项。", label, snapshot.items.len())),
        stderr: None,
        exit_code: 0,
    })
}
