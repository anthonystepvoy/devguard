use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub timestamp: String,
    pub action: String,
    pub package_manager: String,
    pub project_dir: String,
    pub packages_installed: Option<usize>,
    pub scripts_run: Option<usize>,
    pub network_requests: Option<usize>,
    pub blocked_requests: Option<usize>,
    pub secrets_denied: Option<usize>,
    pub exit_code: Option<i32>,
    pub duration_ms: u64,
    pub verdict: String,
}

pub fn log_entry(entry: &AuditEntry) -> io::Result<()> {
    let log_dir = log_dir();
    fs::create_dir_all(&log_dir)?;

    let log_path = log_dir.join("audit.jsonl");
    let line = serde_json::to_string(entry)? + "\n";
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)?
        .metadata()?;
    std::fs::write(&log_path, line)?;

    Ok(())
}

pub fn show_recent(lines: usize) -> Result<(), Box<dyn std::error::Error>> {
    let log_path = log_dir().join("audit.jsonl");
    if !log_path.exists() {
        println!("No audit log entries yet.");
        return Ok(());
    }

    let contents = fs::read_to_string(&log_path)?;
    let entries: Vec<&str> = contents.lines().collect();
    let start = if entries.len() > lines {
        entries.len() - lines
    } else {
        0
    };

    for line in &entries[start..] {
        if let Ok(entry) = serde_json::from_str::<AuditEntry>(line) {
            println!(
                "{}\n  action:     {}\n  manager:    {}\n  project:    {}\n  duration:   {}ms\n  verdict:    {}\n  secrets:    denied {}\n  network:    {} reqs / {} blocked\n  scripts:    {}\n  exit:       {}\n",
                "─".repeat(50),
                entry.action,
                entry.package_manager,
                entry.project_dir,
                entry.duration_ms,
                entry.verdict,
                entry.secrets_denied.unwrap_or(0),
                entry.network_requests.unwrap_or(0),
                entry.blocked_requests.unwrap_or(0),
                entry.scripts_run.unwrap_or(0),
                entry.exit_code.map(|e| e.to_string()).unwrap_or_else(|| "N/A".to_string()),
            );
        }
    }

    Ok(())
}

fn log_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("devguard")
}

pub fn format_duration(ms: u64) -> String {
    if ms < 1000 {
        format!("{}ms", ms)
    } else if ms < 60_000 {
        format!("{:.1}s", ms as f64 / 1000.0)
    } else {
        format!("{}m {}s", ms / 60_000, (ms % 60_000) / 1000)
    }
}
