use std::path::PathBuf;

pub fn ensure_data_dirs() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or_else(|| "Cannot determine home directory".to_string())?;
    let data_dir = home.join(".ohmywu");
    let dirs = [
        "sessions",
        "actions",
        "capabilities",
        "wiki",
        "runtime/threads",
        "runtime/turns",
        "runtime/events",
        "runtime/checklists",
    ];

    for sub in &dirs {
        let path = data_dir.join(sub);
        std::fs::create_dir_all(&path)
            .map_err(|e| format!("Failed to create {}: {}", path.display(), e))?;
    }

    Ok(data_dir)
}
