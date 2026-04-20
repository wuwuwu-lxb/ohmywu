use std::fs;

use chrono::{DateTime, Utc};

use ohmywu_command_runner::CommandRunner;
use ohmywu_domain::{
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
