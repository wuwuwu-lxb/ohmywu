use std::fs;

use chrono::{DateTime, Utc};

use ohmywu_command_runner::CommandRunner;
use ohmywu_domain::{
    CleanupExecuteQuery, CleanupItem, CleanupPreviewQuery, CleanupPreviewResult, CleanupPreset,
    CleanupNode, CleanupTreeResult,
    JournalEntry, JournalQuery, ProcessInfo, ServiceInfo, StorageEntry, StorageScanQuery,
    StorageScanResult,
};

#[derive(Debug, Clone)]
pub struct LinuxAdapter {
    pub distro_family: String,
}

impl Default for LinuxAdapter {
    fn default() -> Self {
        Self {
            distro_family: "debian-family".to_string(),
        }
    }
}

impl LinuxAdapter {
    pub fn supports_v0_1_scope(&self) -> bool {
        self.distro_family == "debian-family"
    }

    /// Read process list from /proc filesystem (sync — /proc is fast)
    pub fn list_processes(&self) -> Vec<ProcessInfo> {
        let mut processes = Vec::new();

        let proc_dir = match fs::read_dir("/proc") {
            Ok(d) => d,
            Err(_) => return vec![],
        };

        for entry in proc_dir.flatten() {
            let path = entry.path();
            let filename = match path.file_name().and_then(|s| s.to_str()) {
                Some(f) => f,
                None => continue,
            };
            let pid: u64 = match filename.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };

            let stat_path = path.join("stat");
            let cmdline_path = path.join("cmdline");
            let status_path = path.join("status");

            let content = fs::read_to_string(&stat_path).unwrap_or_default();
            let content = content.trim();
            let open_paren = match content.find('(') {
                Some(p) => p,
                None => continue,
            };
            let close_paren = match content.rfind(')') {
                Some(p) if p > open_paren => p,
                _ => continue,
            };
            let name = content[open_paren + 1..close_paren].to_string();
            let after_paren = &content[close_paren + 1..];
            let parts: Vec<&str> = after_paren.split_whitespace().collect();
            let state = parts.get(0).unwrap_or(&"?").to_string();
            let utime: u64 = parts.get(12).and_then(|s| s.parse().ok()).unwrap_or(0);
            let stime: u64 = parts.get(13).and_then(|s| s.parse().ok()).unwrap_or(0);

            let (user, memory_kb) = {
                let content = fs::read_to_string(&status_path).unwrap_or_default();
                let mut user = String::new();
                let mut memory_kb = 0u64;
                for line in content.lines() {
                    if line.starts_with("Uid:") {
                        let uid: u32 = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                        user = uid_to_name(uid);
                    }
                    if line.starts_with("VmRSS:") {
                        memory_kb = line.split_whitespace().nth(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                    }
                }
                (user, memory_kb)
            };

            let command = {
                let bytes = fs::read(&cmdline_path).unwrap_or_default();
                if bytes.is_empty() {
                    String::new()
                } else {
                    bytes.split(|&b| b == 0).map(|s| String::from_utf8_lossy(s).to_string()).collect::<Vec<_>>().join(" ")
                }
            };

            let cpu_percent = ((utime + stime) as f32) / 100.0;

            processes.push(ProcessInfo {
                pid,
                name,
                state,
                cpu_percent,
                memory_kb,
                user,
                command,
            });
        }

        processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
        processes
    }

    /// List systemd services via systemctl
    pub async fn list_services(&self) -> Vec<ServiceInfo> {
        let runner = CommandRunner::default();
        let output = runner
            .run(
                "systemctl",
                &[
                    "list-units",
                    "--type=service",
                    "--all",
                    "--no-pager",
                    "--plain",
                    "--no-legend",
                ],
                Some(15),
            )
            .await;

        if output.exit_code != 0 {
            return vec![];
        }

        output
            .stdout
            .lines()
            .filter_map(|line| {
                let parts: Vec<&str> = line.splitn(5, "::").collect();
                if parts.len() >= 4 {
                    return Some(ServiceInfo {
                        name: parts[0].to_string(),
                        load_state: parts[1].to_string(),
                        active_state: parts[2].to_string(),
                        sub_state: parts[3].to_string(),
                        description: String::new(),
                        enabled: false,
                    });
                }
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() < 4 {
                    return None;
                }
                Some(ServiceInfo {
                    name: parts[0].to_string(),
                    load_state: parts.get(1).map(|s| s.to_string()).unwrap_or_default(),
                    active_state: parts.get(2).map(|s| s.to_string()).unwrap_or_default(),
                    sub_state: parts.get(3).map(|s| s.to_string()).unwrap_or_default(),
                    description: String::new(),
                    enabled: false,
                })
            })
            .collect()
    }

    /// Scan storage using du
    pub async fn scan_storage(&self, query: StorageScanQuery) -> StorageScanResult {
        let root_path = query.path.as_deref().unwrap_or("/");
        let depth = query.depth.unwrap_or(1);

        let runner = CommandRunner::default();
        let output = runner.run("du", &["-b", "--max-depth", &depth.to_string(), root_path], Some(60)).await;

        let mut entries: Vec<StorageEntry> = output
            .stdout
            .lines()
            .filter_map(|line| {
                let (size_str, path_str) = line.split_once('\t')?;
                let size_bytes: u64 = size_str.parse().ok()?;
                Some(StorageEntry {
                    path: path_str.to_string(),
                    size_bytes,
                    human_size: human_size(size_bytes),
                })
            })
            .collect();

        entries.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        let total_bytes: u64 = entries.iter().map(|e| e.size_bytes).sum();

        StorageScanResult {
            root_path: root_path.to_string(),
            total_bytes,
            entries,
            scanned_at: Utc::now(),
        }
    }

    /// Read journal entries via journalctl
    pub async fn read_journal(&self, query: JournalQuery) -> Vec<JournalEntry> {
        let mut args: Vec<String> = vec![
            "--output=json".to_string(),
            "--no-pager".to_string(),
            "-b".to_string(),
        ];

        if let Some(unit) = &query.unit {
            args.push("--unit".to_string());
            args.push(unit.clone());
        }
        if let Some(priority) = query.priority {
            args.push("-p".to_string());
            args.push(priority.to_string());
        }
        let lines = query.lines.unwrap_or(50);
        args.push("-n".to_string());
        args.push(lines.to_string());

        let runner = CommandRunner::default();
        let output = runner.run("journalctl", &args.iter().map(|s| s.as_str()).collect::<Vec<_>>(), Some(15)).await;

        output
            .stdout
            .lines()
            .filter_map(|line| {
                let json: serde_json::Value = serde_json::from_str(line).ok()?;
                let timestamp = json
                    .get("__REALTIME_TIMESTAMP")
                    .and_then(|v| v.as_str())
                    .and_then(|s| s.parse::<i64>().ok())
                    .map(|ms| DateTime::from_timestamp(ms / 1_000_000, 0).unwrap_or_else(Utc::now))
                    .unwrap_or_else(Utc::now);
                let unit = json
                    .get("_SYSTEMD_UNIT")
                    .or_else(|| json.get("UNIT"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("?")
                    .to_string();
                let priority = json
                    .get("PRIORITY")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(6) as i32;
                let message = json
                    .get("MESSAGE")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                Some(JournalEntry {
                    timestamp,
                    unit,
                    priority,
                    message,
                })
            })
            .collect()
    }

    // ── M3: Write Operations ───────────────────────────────────────────────────

    /// Kill a process by PID using the kill command
    pub fn kill_process(&self, pid: u64) -> Result<(), String> {
        let output = std::process::Command::new("kill")
            .arg(pid.to_string())
            .output()
            .map_err(|e| format!("failed to execute kill: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("kill failed: {}", stderr))
        }
    }

    /// Start a systemd service
    pub async fn start_service(&self, name: &str) -> Result<(), String> {
        let runner = CommandRunner::default();
        let output = runner.run("systemctl", &["start", name], Some(30)).await;
        if output.exit_code == 0 {
            Ok(())
        } else {
            Err(format!("failed to start {}: {}", name, output.stderr))
        }
    }

    /// Stop a systemd service
    pub async fn stop_service(&self, name: &str) -> Result<(), String> {
        let runner = CommandRunner::default();
        let output = runner.run("systemctl", &["stop", name], Some(30)).await;
        if output.exit_code == 0 {
            Ok(())
        } else {
            Err(format!("failed to stop {}: {}", name, output.stderr))
        }
    }

    /// Restart a systemd service
    pub async fn restart_service(&self, name: &str) -> Result<(), String> {
        let runner = CommandRunner::default();
        let output = runner.run("systemctl", &["restart", name], Some(30)).await;
        if output.exit_code == 0 {
            Ok(())
        } else {
            Err(format!("failed to restart {}: {}", name, output.stderr))
        }
    }

    // ── M3: Storage Cleanup ─────────────────────────────────────────────────────

    /// Scan cleanup tree: $HOME + /tmp + /var/log + /var/cache/apt
    pub async fn cleanup_tree(&self) -> CleanupTreeResult {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        let runner = CommandRunner::default();

        let mut root_children = Vec::new();

        // Scan $HOME (depth 3, L1/L2/Unknown only, not L3)
        if let Some(home_node) = self.scan_home_tree(&runner, &home, 2).await {
            root_children.push(home_node);
        }

        // Scan /tmp (single level)
        if let Some(tmp_node) = self.scan_tmp_tree(&runner).await {
            root_children.push(tmp_node);
        }

        // Scan /var/log (L3 system files)
        if let Some(log_node) = self.scan_var_log_tree(&runner).await {
            root_children.push(log_node);
        }

        // Scan /var/cache/apt (L3 system files)
        if let Some(apt_node) = self.scan_apt_cache_tree(&runner).await {
            root_children.push(apt_node);
        }

        let total_bytes: u64 = root_children.iter().map(|n| n.size_bytes).sum();
        let root = CleanupNode {
            path: "/".to_string(),
            name: "可清理空间".to_string(),
            size_bytes: total_bytes,
            human_size: human_size(total_bytes),
            category: "root".to_string(),
            level: "root".to_string(),
            reason: "".to_string(),
            children: root_children,
        };

        CleanupTreeResult {
            root,
            total_bytes,
            human_total: human_size(total_bytes),
        }
    }

    /// Scan children of a specific path (for on-demand deep drilling)
    pub async fn cleanup_scan_path(&self, path: &str) -> CleanupTreeResult {
        let runner = CommandRunner::default();
        let output = runner.run("du", &["-b", "--max-depth=1", path], Some(30)).await;

        let mut children = Vec::new();
        let mut total_bytes = 0u64;

        for line in output.stdout.lines() {
            if let Some((size_str, path_str)) = line.split_once('\t') {
                if let Ok(size) = size_str.parse::<u64>() {
                    if path_str != path && size > 0 {
                        total_bytes += size;
                        let name = path_str.split('/').last().unwrap_or(&path_str).to_string();
                        let (cat, level, reason) = classify(&path_str, size, true);

                        // Check if this child has its own children by doing a depth-1 check
                        let child_output = runner.run("du", &["-b", "--max-depth=0", &path_str], Some(10)).await;
                        let has_children = child_output.stdout.lines().count() > 1
                            || child_output.stdout.lines().next()
                                .and_then(|l| l.split_once('\t'))
                                .and_then(|(s, _)| s.parse::<u64>().ok())
                                .map(|s| s > size)
                                .unwrap_or(false);

                        let grandchildren = if has_children {
                            let sub_output = runner.run("du", &["-b", "--max-depth=1", &path_str], Some(30)).await;
                            let mut sub_children = Vec::new();
                            for sub_line in sub_output.stdout.lines() {
                                if let Some((sub_size_str, sub_path_str)) = sub_line.split_once('\t') {
                                    if let Ok(sub_size) = sub_size_str.parse::<u64>() {
                                        if sub_path_str != path_str && sub_size > 0 {
                                            let sub_name = sub_path_str.split('/').last().unwrap_or(&sub_path_str).to_string();
                                            let (sub_cat, sub_level, sub_reason) = classify(&sub_path_str, sub_size, true);
                                            sub_children.push(CleanupNode {
                                                path: sub_path_str.clone(),
                                                name: sub_name,
                                                size_bytes: sub_size,
                                                human_size: human_size(sub_size),
                                                category: sub_cat.to_string(),
                                                level: sub_level.to_string(),
                                                reason: sub_reason.to_string(),
                                                children: Vec::new(),
                                            });
                                        }
                                    }
                                }
                            }
                            sub_children
                        } else {
                            Vec::new()
                        };

                        children.push(CleanupNode {
                            path: path_str.clone(),
                            name,
                            size_bytes: size,
                            human_size: human_size(size),
                            category: cat.to_string(),
                            level: level.to_string(),
                            reason: reason.to_string(),
                            children: grandchildren,
                        });
                    }
                }
            }
        }

        children.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));
        let name = path.split('/').last().unwrap_or(path).to_string();
        let root = CleanupNode {
            path: path.to_string(),
            name,
            size_bytes: total_bytes,
            human_size: human_size(total_bytes),
            category: "folder".to_string(),
            level: "folder".to_string(),
            reason: "".to_string(),
            children,
        };

        CleanupTreeResult {
            root,
            total_bytes,
            human_total: human_size(total_bytes),
        }
    }

    async fn scan_home_tree(&self, runner: &CommandRunner, home: &str, max_depth: usize) -> Option<CleanupNode> {
        let output = runner.run("du", &["-b", &format!("--max-depth={}", max_depth), home], Some(60)).await;
        let mut paths: Vec<(u64, String)> = Vec::new();

        for line in output.stdout.lines() {
            if let Some((size_str, path_str)) = line.split_once('\t') {
                if let Ok(size) = size_str.parse::<u64>() {
                    if size > 10 * 1024 * 1024 && path_str != home {
                        paths.push((size, path_str.to_string()));
                    }
                }
            }
        }

        if paths.is_empty() {
            return None;
        }

        paths.sort_by(|a, b| b.0.cmp(&a.0));
        let total = paths.iter().map(|p| p.0).sum::<u64>();
        let home_name = home.split('/').last().unwrap_or("home");

        let children = paths.into_iter().map(|(size, path)| {
            let name = path.split('/').last().unwrap_or(&path).to_string();
            let (cat, level, reason) = classify(&path, size, true);
            CleanupNode {
                path: path.clone(),
                name,
                size_bytes: size,
                human_size: human_size(size),
                category: cat.to_string(),
                level: level.to_string(),
                reason: reason.to_string(),
                children: Vec::new(),
            }
        }).collect();

        Some(CleanupNode {
            path: home.to_string(),
            name: home_name.to_string(),
            size_bytes: total,
            human_size: human_size(total),
            category: "other".to_string(),
            level: "folder".to_string(),
            reason: "".to_string(),
            children,
        })
    }

    async fn scan_tmp_tree(&self, runner: &CommandRunner) -> Option<CleanupNode> {
        let output = runner.run("find", &["/tmp", "-type", "f", "-atime", "+7"], Some(20)).await;
        let mut total = 0u64;
        let mut children = Vec::new();

        for path_str in output.stdout.lines().filter(|s| !s.is_empty()) {
            if let Ok(meta) = std::fs::metadata(path_str) {
                let size = meta.len();
                if size > 0 {
                    total += size;
                    let name = path_str.split('/').last().unwrap_or(path_str).to_string();
                    children.push(CleanupNode {
                        path: path_str.to_string(),
                        name,
                        size_bytes: size,
                        human_size: human_size(size),
                        category: "temp".to_string(),
                        level: "L1".to_string(),
                        reason: "7 天未访问的临时文件".to_string(),
                        children: Vec::new(),
                    });
                }
            }
        }

        if total > 0 {
            return Some(CleanupNode {
                path: "/tmp".to_string(),
                name: "tmp".to_string(),
                size_bytes: total,
                human_size: human_size(total),
                category: "temp".to_string(),
                level: "L1".to_string(),
                reason: "".to_string(),
                children,
            });
        }
        None
    }

    async fn scan_var_log_tree(&self, runner: &CommandRunner) -> Option<CleanupNode> {
        let output = runner.run("find", &["/var/log", "-name", "*.gz", "-type", "f"], Some(10)).await;
        let mut total = 0u64;
        let mut children = Vec::new();

        for line in output.stdout.lines() {
            let path = line.trim();
            if path.is_empty() { continue; }
            if let Ok(meta) = std::fs::metadata(path) {
                let size = meta.len();
                total += size;
                let name = path.split('/').last().unwrap_or(path).to_string();
                children.push(CleanupNode {
                    path: path.to_string(),
                    name,
                    size_bytes: size,
                    human_size: human_size(size),
                    category: "log".to_string(),
                    level: "L3".to_string(),
                    reason: "系统日志（需 root 权限）".to_string(),
                    children: Vec::new(),
                });
            }
        }

        if total > 0 {
            return Some(CleanupNode {
                path: "/var/log".to_string(),
                name: "var/log".to_string(),
                size_bytes: total,
                human_size: human_size(total),
                category: "log".to_string(),
                level: "L3".to_string(),
                reason: "".to_string(),
                children,
            });
        }
        None
    }

    async fn scan_apt_cache_tree(&self, runner: &CommandRunner) -> Option<CleanupNode> {
        let output = runner.run("du", &["-b", "-s", "/var/cache/apt"], Some(5)).await;
        if let Some(line) = output.stdout.lines().next() {
            if let Some((size_str, _)) = line.split_once('\t') {
                if let Ok(size) = size_str.parse::<u64>() {
                    if size > 0 {
                        return Some(CleanupNode {
                            path: "/var/cache/apt".to_string(),
                            name: "apt cache".to_string(),
                            size_bytes: size,
                            human_size: human_size(size),
                            category: "apt".to_string(),
                            level: "L3".to_string(),
                            reason: "apt 缓存（需 root 权限）".to_string(),
                            children: Vec::new(),
                        });
                    }
                }
            }
        }
        None
    }

    /// Scan for cleanup candidates. (Legacy stub for backward compat - use cleanup_tree instead)
    pub async fn cleanup_preview(&self, _query: CleanupPreviewQuery) -> CleanupPreviewResult {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        CleanupPreviewResult {
            items: vec![],
            total_bytes: 0,
            human_total: "0 B".to_string(),
            scanned_path: home,
        }
    }


    /// Delete the user-confirmed paths (L1/L2 only, L3 filtered by frontend).
    pub async fn cleanup_execute(&self, query: CleanupExecuteQuery) -> Result<u64, String> {
        let mut freed_bytes: u64 = 0;

        for path_str in &query.paths {
            let path = std::path::Path::new(path_str);
            if !path.exists() {
                continue;
            }
            let size_before = Self::dir_size(path).unwrap_or(0);

            let result = if path.is_file() {
                tokio::fs::remove_file(path).await
            } else {
                tokio::fs::remove_dir_all(path).await
            };

            match result {
                Ok(()) => freed_bytes += size_before,
                Err(e) => return Err(format!("failed to remove {}: {}", path_str, e)),
            }
        }

        Ok(freed_bytes)
    }


    fn dir_size(path: &std::path::Path) -> Option<u64> {
        if path.is_file() {
            return Some(std::fs::metadata(path).ok()?.len());
        }
        let mut total: u64 = 0;
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let p = entry.path();
                total += Self::dir_size(&p).unwrap_or(0);
            }
        }
        Some(total)
    }
}

// L1 white list (absolutely safe)
fn is_l1(path: &str) -> bool {
    path.contains("/.cache/pip/") || path.ends_with("/.cache/pip")
        || path.contains("/.cache/uv/") || path.ends_with("/.cache/uv")
        || path.contains("/.cache/go-build/") || path.ends_with("/.cache/go-build")
        || path.contains("/.cache/thumbnails/") || path.ends_with("/.cache/thumbnails")
        || path.contains("/.cargo/registry/cache/") || path.contains("/.cargo/registry/src/")
        || path.contains("/.npm/_cacache/") || path.ends_with("/.npm/_cacache")
        || path.contains("/.local/share/pnpm/store/")
        || path.contains("/.local/share/Trash/files/")
}

// L2 caution (needs confirmation)
fn is_l2(path: &str) -> bool {
    path.contains("/.cache/chromium/") || path.contains("/.cache/firefox/")
        || path.contains("/.nvm/versions/") || path.contains("/.pyenv/versions/")
        || path.contains("/.rustup/toolchains/")
}

// L3 system files (need root)
fn is_l3(path: &str) -> bool {
    path.starts_with("/var/log/") || path.starts_with("/var/cache/apt/")
        || path.starts_with("/etc/") || path.starts_with("/usr/bin/")
        || path.starts_with("/usr/lib/")
}

fn classify(path: &str, size: u64, _is_dir: bool) -> (&'static str, &'static str, &'static str) {
    if is_l3(path) {
        return ("log", "L3", "系统文件（需 root 权限）");
    }
    if is_l1(path) {
        if path.contains("/.cache/") {
            return ("cache", "L1", "");
        }
        if path.contains("/.cargo/") || path.contains("/.npm/") {
            return ("package_cache", "L1", "");
        }
        return ("other", "L1", "");
    }
    if is_l2(path) {
        if path.contains("/.cache/") {
            return ("cache", "L2", "应用缓存（需确认）");
        }
        return ("toolchain", "L2", "工具链版本（需确认）");
    }
    if size > 500 * 1024 * 1024 {
        return ("unknown", "L2", "体积超 500MB");
    }
    if size > 50 * 1024 * 1024 {
        return ("unknown", "unknown", "未分类大目录");
    }
    ("other", "L1", "")
}


fn human_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn uid_to_name(uid: u32) -> String {
    let output = std::process::Command::new("id")
        .arg("-u")
        .arg(&uid.to_string())
        .output()
        .ok();
    match output {
        Some(o) if o.status.success() => {
            let name = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if name.is_empty() { uid.to_string() } else { name }
        }
        _ => uid.to_string(),
    }
}

fn make_item(
    path: String,
    size_bytes: u64,
    category: &str,
    reason: &str,
    risk: &str,
    risk_reason: &str,
    executable: bool,
    execution_block_reason: &str,
) -> CleanupItem {
    CleanupItem {
        human_size: human_size(size_bytes),
        path,
        size_bytes,
        category: category.to_string(),
        reason: reason.to_string(),
        risk: risk.to_string(),
        risk_reason: risk_reason.to_string(),
        executable,
        execution_block_reason: execution_block_reason.to_string(),
    }
}

fn classify_path(path: &str, size: u64, is_dir: bool) -> (&'static str, &'static str, &'static str, bool, &'static str) {
    // executable=false: only truly dangerous things (toolchain, large_dir)
    // risk=risky: needs user confirmation but CAN be executed
    // risk=safe: no confirmation needed

    if path.contains("/.cache/") || path.ends_with("/.cache") {
        return ("cache", "safe", "", true, "");
    }
    if path.contains("/.npm") || path.contains("/npm/_cacache") {
        return ("package_cache", "safe", "", true, "");
    }
    if path.contains("/.cargo/registry") {
        return ("package_cache", "safe", "", true, "");
    }
    if path.contains("/.nvm/versions") || path.contains("/.pyenv/versions") || path.contains("/.rustup/toolchains") {
        return (
            "toolchain",
            "risky",
            "删除工具链版本可能影响开发环境，请确认不再使用",
            false,
            "工具链目录暂不支持应用内直接清理，请先切换版本或手动卸载",
        );
    }
    if is_dir && size > 100 * 1024 * 1024 {
        return (
            "large_dir",
            "risky",
            "大型目录（超过 100MB），请先确认内容再清理",
            false,
            "大型目录暂不支持应用内直接递归删除，请手动确认后处理",
        );
    }
    if path.starts_with("/tmp") {
        return ("temp", "safe", "", true, "");
    }
    if path.starts_with("/var/log") {
        return ("log", "risky", "系统日志，操作前请确认", true, "");
    }
    // All other paths: executable=true, but risky if it's a dir
    ("
other", "safe", "", !is_dir, if is_dir { "目录操作有风险，请先确认内容" } else { "" })
}
