use serde_json::Value;

use ohmywu_wiki::{GraphData, NoteMeta, WikiEngine, WikiNote};

use super::ExecOutput;

pub fn read(params: &Value, wiki: &WikiEngine) -> Result<ExecOutput, String> {
    let slug = get_param(params, "slug")?;
    let slug = slug
        .chars()
        .filter(|c| c.is_alphanumeric() || *c == '-')
        .collect::<String>();
    if slug.is_empty() {
        return Err("slug is required".into());
    }

    let note: WikiNote = wiki.read_note(&slug)?;
    let md = format_note_markdown(&note);
    Ok(ExecOutput {
        output: Some(md),
        stderr: None,
        exit_code: 0,
    })
}

pub fn write(params: &Value, wiki: &WikiEngine) -> Result<ExecOutput, String> {
    let slug = get_param(params, "slug")?;
    let title = get_param(params, "title")?;
    let body = get_param(params, "body")?;
    let tags: Vec<String> = params
        .get("tags")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        })
        .unwrap_or_default();
    let folder = get_param(params, "folder").unwrap_or_else(|_| "notes".into());

    let note = wiki.write_note(&slug, &title, &body, &tags, &folder)?;
    Ok(ExecOutput {
        output: Some(format!("note '{}' saved — {}", note.slug, note.title)),
        stderr: None,
        exit_code: 0,
    })
}

pub fn search(params: &Value, wiki: &WikiEngine) -> Result<ExecOutput, String> {
    let query = get_param(params, "query")?;
    let results: Vec<NoteMeta> = wiki.search(&query)?;

    let md = if results.is_empty() {
        "no matching notes found.".into()
    } else {
        let mut out = format!("# search results for \"{}\"\n\n", query);
        for r in &results {
            out.push_str(&format!(
                "- **{}** (`{}`) — tags: {}\n",
                r.title,
                r.slug,
                r.tags.join(", ")
            ));
        }
        out
    };

    Ok(ExecOutput {
        output: Some(md),
        stderr: None,
        exit_code: 0,
    })
}

pub fn list(_params: &Value, wiki: &WikiEngine) -> Result<ExecOutput, String> {
    let notes: Vec<NoteMeta> = wiki.list_notes()?;

    let md = if notes.is_empty() {
        "no wiki notes yet.".into()
    } else {
        let mut out = "# wiki notes\n\n".to_string();
        for n in &notes {
            out.push_str(&format!(
                "- **{}** (`{}`) — tags: {}\n",
                n.title, n.slug, n.tags.join(", ")
            ));
        }
        out
    };

    Ok(ExecOutput {
        output: Some(md),
        stderr: None,
        exit_code: 0,
    })
}

pub fn graph(_params: &Value, wiki: &WikiEngine) -> Result<ExecOutput, String> {
    let data: GraphData = wiki.build_graph()?;
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("serialize graph: {}", e))?;
    Ok(ExecOutput {
        output: Some(json),
        stderr: None,
        exit_code: 0,
    })
}

fn get_param(params: &Value, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("missing or empty parameter: '{}'", key))
}

fn format_note_markdown(note: &WikiNote) -> String {
    format!(
        "---\ntitle: {}\ntags: [{}]\ncreated: {}\nupdated: {}\n---\n\n{}",
        note.title,
        note.tags.join(", "),
        note.created,
        note.updated,
        note.body,
    )
}
