use crate::paths;
use colored::Colorize;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Serialize)]
struct DoctorReport {
    version: &'static str,
    os: &'static str,
    arch: &'static str,
    home_dir: Option<String>,
    temp_dir: String,
    temp_writable: bool,
    audit_dir: Option<String>,
    package_managers: Vec<PackageManagerStatus>,
    npm_ignore_scripts: Option<String>,
    known_secret_files_found: usize,
    warnings: Vec<String>,
    limitations: Vec<&'static str>,
}

#[derive(Debug, Serialize)]
struct PackageManagerStatus {
    name: &'static str,
    found: bool,
    path: Option<String>,
    version: Option<String>,
}

pub fn run_doctor(json: bool) -> Result<(), Box<dyn std::error::Error>> {
    let report = build_report();

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_report(&report);
    }

    Ok(())
}

fn build_report() -> DoctorReport {
    let home_dir = home::home_dir();
    let package_managers = ["npm", "pnpm", "yarn", "bun"]
        .into_iter()
        .map(check_package_manager)
        .collect::<Vec<_>>();
    let npm_ignore_scripts = package_managers
        .iter()
        .find(|pm| pm.name == "npm")
        .and_then(|pm| pm.path.as_ref())
        .and_then(|path| command_output(Path::new(path), &["config", "get", "ignore-scripts"]));

    let temp_writable = tempfile::Builder::new()
        .prefix("devguard-doctor-")
        .tempdir()
        .is_ok();
    let known_secret_files_found = home_dir
        .as_deref()
        .map(|home| {
            paths::known_secret_paths_in(home)
                .iter()
                .filter(|secret| secret.path.exists())
                .count()
        })
        .unwrap_or(0);

    let mut warnings = Vec::new();
    if home_dir.is_none() {
        warnings.push("Could not determine HOME directory".to_string());
    }
    if !temp_writable {
        warnings.push("Could not create a temporary sandbox directory".to_string());
    }
    if !package_managers.iter().any(|pm| pm.found) {
        warnings.push("No supported package manager found in PATH".to_string());
    }
    if npm_ignore_scripts.as_deref() == Some("true") {
        warnings.push(
            "npm ignore-scripts=true is set; installing devguard from npm needs --ignore-scripts=false"
                .to_string(),
        );
    }
    if known_secret_files_found > 0 {
        warnings.push(format!(
            "{} known home secret path(s) exist; devguard hides normal HOME lookups but not absolute paths",
            known_secret_files_found
        ));
    }

    DoctorReport {
        version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        home_dir: home_dir.map(display_path),
        temp_dir: display_path(std::env::temp_dir()),
        temp_writable,
        audit_dir: dirs::data_local_dir().map(|dir| display_path(dir.join("devguard"))),
        package_managers,
        npm_ignore_scripts,
        known_secret_files_found,
        warnings,
        limitations: vec![
            "Current isolation is HOME/env redirection, not OS-level filesystem sandboxing.",
            "Absolute paths to real home files can bypass v0.1.x protections.",
            "Files in the project directory remain visible to lifecycle scripts.",
            "Network policy is advisory only until OS-level enforcement exists.",
        ],
    }
}

fn check_package_manager(name: &'static str) -> PackageManagerStatus {
    let candidates: &[&str] = match name {
        "npm" => &["npm.cmd", "npm"],
        "pnpm" => &["pnpm.cmd", "pnpm"],
        "yarn" => &["yarn.cmd", "yarn"],
        "bun" => &["bun.exe", "bun"],
        _ => &[name],
    };

    let path = candidates
        .iter()
        .find_map(|candidate| which::which(candidate).ok());
    let version = path
        .as_deref()
        .and_then(|pm_path| command_output(pm_path, &["--version"]));

    PackageManagerStatus {
        name,
        found: path.is_some(),
        path: path.map(display_path),
        version,
    }
}

fn command_output(command: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(command)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let value = stdout.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn print_report(report: &DoctorReport) {
    println!();
    println!("{}", "devguard doctor".green().bold());
    println!("version: {}", report.version);
    println!("system:  {} {}", report.os, report.arch);
    println!(
        "home:    {}",
        report.home_dir.as_deref().unwrap_or("(unknown)").dimmed()
    );
    println!(
        "temp:    {} {}",
        report.temp_dir.dimmed(),
        status_label(report.temp_writable)
    );
    if let Some(audit_dir) = &report.audit_dir {
        println!("audit:   {}", audit_dir.dimmed());
    }

    println!();
    println!("{}", "package managers".bold());
    for pm in &report.package_managers {
        match (&pm.path, &pm.version) {
            (Some(path), Some(version)) => println!(
                "  {} {:<5} {} ({})",
                "ok".green(),
                pm.name,
                version,
                path.dimmed()
            ),
            (Some(path), None) => println!("  {} {:<5} {}", "ok".green(), pm.name, path.dimmed()),
            (None, _) => println!("  {} {:<5} not found", "miss".yellow(), pm.name),
        }
    }
    if let Some(ignore_scripts) = &report.npm_ignore_scripts {
        println!("  npm ignore-scripts: {}", ignore_scripts);
    }

    println!();
    println!(
        "{} known home secret path(s) found",
        report.known_secret_files_found
    );

    if !report.warnings.is_empty() {
        println!();
        println!("{}", "warnings".yellow().bold());
        for warning in &report.warnings {
            println!("  - {}", warning);
        }
    }

    println!();
    println!("{}", "limitations".bold());
    for limitation in &report.limitations {
        println!("  - {}", limitation);
    }
}

fn status_label(ok: bool) -> String {
    if ok {
        "ok".green().to_string()
    } else {
        "failed".red().to_string()
    }
}

fn display_path(path: PathBuf) -> String {
    path.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doctor_report_has_limitations() {
        let report = build_report();
        assert!(!report.limitations.is_empty());
        assert_eq!(report.version, env!("CARGO_PKG_VERSION"));
    }
}
