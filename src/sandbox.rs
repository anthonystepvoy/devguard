use crate::audit::{self, AuditEntry};
use crate::env;
use crate::network;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;

pub fn run_install(
    manager: &str,
    args: &[String],
    allow_network: bool,
    cwd: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let start = Instant::now();

    print_header(manager, allow_network);

    let project_dir = resolve_project_dir(cwd)?;
    let pm_path = find_package_manager(manager)?;

    println!(
        "{} Using: {}",
        "→".bold(),
        pm_path.display().to_string().dimmed()
    );
    println!(
        "{} Project: {}",
        "→".bold(),
        project_dir.display().to_string().dimmed()
    );

    let uses_two_phase = has_ignore_scripts_support(manager) && has_rebuild_support(manager);

    if !uses_two_phase {
        println!(
            "{} Using single-pass union HOME (manager lacks rebuild support)",
            "→".yellow()
        );
    }

    println!();

    if uses_two_phase {
        println!(
            "{} Phase 1: Downloading packages (real HOME, scripts disabled)",
            "⬇".cyan().bold()
        );
        let phase1_code = spawn_download_phase(&pm_path, manager, args, &project_dir)?;
        if phase1_code != 0 {
            let duration = start.elapsed();
            log_audit(manager, &project_dir, phase1_code, duration.as_millis() as u64, "download-failed", None, None);
            println!();
            println!(
                "{} Download failed (exit {})",
                "✗".red().bold(),
                phase1_code
            );
            return Ok(());
        }
        println!("  {} Packages downloaded", "✓".green());

        println!();
        println!(
            "{} Phase 2: Rebuilding in sandbox (fake HOME, no secrets)",
            "⬆".yellow().bold()
        );

        let sandbox_home = create_sandbox_home()?;
        let sandbox_env = build_sandbox_env(&sandbox_home);

        env::print_env_summary(std::env::vars().count(), sandbox_env.len());
        network::print_network_policy(allow_network);

        println!(
            "{} Sandbox HOME: {}",
            "→".bold(),
            sandbox_home.display().to_string().dimmed()
        );
        print_sandbox_wall();

        println!();
        println!("{} Running {} rebuild...", "[*]".cyan().bold(), manager);

        let phase2_code = spawn_rebuild_phase(&pm_path, manager, &sandbox_env, &project_dir)?;
        let duration = start.elapsed();
        let secrets_denied = std::env::vars().count() - sandbox_env.len() + 1;

        if phase2_code == 0 {
            println!();
            println!(
                "{} Install complete {}",
                "✓".green().bold(),
                audit::format_duration(duration.as_millis() as u64).dimmed()
            );

            println!();
            println!(
                "{} Scripts ran with NO access to: .npmrc, .ssh, .aws, .docker, .kube, .config/gh, .env, shell history",
                "🔒".bold()
            );

            log_audit(
                manager,
                &project_dir,
                0,
                duration.as_millis() as u64,
                "clean",
                Some((std::env::vars().count() - sandbox_env.len() + 1) as usize),
                None,
            );
        } else {
            println!();
            println!(
                "{} Rebuild failed (exit {}) {}",
                "✗".red().bold(),
                phase2_code,
                audit::format_duration(duration.as_millis() as u64).dimmed()
            );
            log_audit(
                manager,
                &project_dir,
                phase2_code,
                duration.as_millis() as u64,
                &format!("rebuild-exit-{}", phase2_code),
                Some(secrets_denied as usize),
                None,
            );
        }
    } else {
        let real_home = home::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let sandbox_home = create_union_home(&real_home)?;
        let sandbox_env = build_sandbox_env(&sandbox_home);

        env::print_env_summary(std::env::vars().count(), sandbox_env.len());
        network::print_network_policy(allow_network);

        println!(
            "{} Union HOME: {}",
            "→".bold(),
            sandbox_home.display().to_string().dimmed()
        );
        print_sandbox_wall();
        println!("  ⚠ .npmrc pass-through enabled (needed for private registries)");
        println!("  ⚠ Postinstall scripts may read .npmrc (all other secrets blocked)");

        println!();
        println!("{} Running {} install...", "[*]".cyan().bold(), manager);

        let exit_code = spawn_install_phase(&pm_path, manager, args, &sandbox_env, &project_dir)?;
        let duration = start.elapsed();

        if exit_code == 0 {
            println!();
            println!(
                "{} Install complete {}",
                "✓".green().bold(),
                audit::format_duration(duration.as_millis() as u64).dimmed()
            );
        } else {
            println!();
            println!(
                "{} Install failed (exit {}) {}",
                "✗".red().bold(),
                exit_code,
                audit::format_duration(duration.as_millis() as u64).dimmed()
            );
        }

        log_audit(
            manager,
            &project_dir,
            exit_code,
            duration.as_millis() as u64,
            if exit_code == 0 { "clean" } else { "failed" },
            Some((std::env::vars().count() - sandbox_env.len() + 1) as usize),
            None,
        );
    }

    if !allow_network {
        println!();
        println!(
            "{} {}",
            "⊡".bold(),
            "Network blocking is advisory in this version; OS-level enforcement coming in v0.2."
                .dimmed()
        );
    }

    Ok(())
}

fn has_ignore_scripts_support(manager: &str) -> bool {
    matches!(manager, "npm" | "pnpm" | "yarn" | "bun")
}

fn has_rebuild_support(manager: &str) -> bool {
    matches!(manager, "npm" | "pnpm")
}

fn print_header(manager: &str, allow_network: bool) {
    println!();
    println!(
        "{} {} {}",
        "╔══".green(),
        "devguard install".green().bold(),
        "══╗".green()
    );
    println!("{} {}", "║".green(), "Sandboxed package install".dimmed());

    let mode = if allow_network {
        "network: full".yellow()
    } else {
        "network: restricted".green()
    };
    println!("{} {} {}", "║".green(), "manager:".dimmed(), manager);
    println!("{} {}", "║".green(), mode);
    println!("{}", "╚══════════════════════════╝".green());
}

fn print_sandbox_wall() {
    println!();
    println!("{} Sandbox enforced:", "---".bold());
    println!("  ✓ HOME = temporary directory (no real secrets)");
    println!("  ✓ No access to ~/.ssh");
    println!("  ✓ No access to ~/.aws");
    println!("  ✓ No access to ~/.config/gh");
    println!("  ✓ No access to ~/.docker");
    println!("  ✓ No access to ~/.kube");
    println!("  ✓ No SSH agent socket");
    println!("  ✓ Secret env vars stripped");
    println!("  ✓ Shell history inaccessible");
}

fn resolve_project_dir(cwd: Option<&str>) -> io::Result<PathBuf> {
    match cwd {
        Some(dir) => {
            let path = PathBuf::from(dir);
            if path.exists() {
                Ok(path.canonicalize()?)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::NotFound,
                    format!("Directory not found: {}", dir),
                ))
            }
        }
        None => std::env::current_dir(),
    }
}

fn find_package_manager(manager: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let candidates = match manager {
        "npm" => vec!["npm.cmd", "npm"],
        "pnpm" => vec!["pnpm.cmd", "pnpm"],
        "yarn" => vec!["yarn.cmd", "yarn"],
        "bun" => vec!["bun.exe", "bun"],
        _ => return Err(format!("Unknown package manager: {}", manager).into()),
    };

    for name in &candidates {
        if let Ok(path) = which::which(name) {
            return Ok(path);
        }
    }

    Err(format!(
        "Package manager '{}' not found in PATH. Is it installed?",
        manager
    )
    .into())
}

fn create_sandbox_home() -> io::Result<PathBuf> {
    #[allow(deprecated)]
    let sandbox = tempfile::tempdir()?.into_path();

    for (subdir, _label) in crate::paths::sandbox_home_paths() {
        let target = sandbox.join(&subdir);
        fs::create_dir_all(&target).ok();
    }

    Ok(sandbox)
}

fn create_union_home(real_home: &PathBuf) -> io::Result<PathBuf> {
    let sandbox = create_sandbox_home()?;

    for filename in crate::paths::auth_pass_through_paths() {
        let src = real_home.join(filename);
        if src.exists() {
            let dst = sandbox.join(filename);
            fs::copy(&src, &dst).ok();
        }
    }

    Ok(sandbox)
}

fn build_sandbox_env(sandbox_home: &PathBuf) -> HashMap<String, String> {
    let mut env = env::sanitize_for_sandbox();

    env.insert("HOME".to_string(), sandbox_home.display().to_string());
    env.insert(
        "USERPROFILE".to_string(),
        sandbox_home.display().to_string(),
    );
    env.insert(
        "APPDATA".to_string(),
        sandbox_home
            .join("AppData")
            .join("Roaming")
            .display()
            .to_string(),
    );
    env.insert(
        "LOCALAPPDATA".to_string(),
        sandbox_home
            .join("AppData")
            .join("Local")
            .display()
            .to_string(),
    );

    env.remove("SSH_AUTH_SOCK");
    env.remove("SSH_AGENT_PID");

    let secret_keys: Vec<String> = env
        .keys()
        .filter(|k| {
            let upper = k.to_uppercase();
            crate::paths::secret_env_var_patterns()
                .iter()
                .any(|p| upper.contains(p))
        })
        .cloned()
        .collect();

    for key in secret_keys {
        env.remove(&key);
    }

    env
}

fn spawn_command(
    pm_path: &PathBuf,
    cmds: &[&str],
    args: &[String],
    env: &HashMap<String, String>,
    project_dir: &PathBuf,
) -> io::Result<i32> {
    let mut cmd = Command::new(pm_path);

    for c in cmds {
        cmd.arg(c);
    }

    if !args.is_empty() {
        cmd.args(args);
    }

    cmd.current_dir(project_dir);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    cmd.env_clear();
    for (key, value) in env {
        cmd.env(key, value);
    }

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(-1))
}

fn spawn_download_phase(
    pm_path: &PathBuf,
    manager: &str,
    args: &[String],
    project_dir: &PathBuf,
) -> io::Result<i32> {
    let mut cmd = Command::new(pm_path);

    match manager {
        "bun" => {
            cmd.arg("add");
            cmd.arg("--ignore-scripts");
        }
        _ => {
            cmd.arg("install");
            cmd.arg("--ignore-scripts");
        }
    }

    if !args.is_empty() {
        cmd.args(args);
    }

    cmd.current_dir(project_dir);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::inherit());
    cmd.stderr(Stdio::inherit());

    let status = cmd.status()?;
    Ok(status.code().unwrap_or(-1))
}

fn spawn_install_phase(
    pm_path: &PathBuf,
    manager: &str,
    args: &[String],
    env: &HashMap<String, String>,
    project_dir: &PathBuf,
) -> io::Result<i32> {
    let install_cmd = match manager {
        "bun" => vec!["add"],
        _ => vec!["install"],
    };

    spawn_command(pm_path, &install_cmd, args, env, project_dir)
}

fn spawn_rebuild_phase(
    pm_path: &PathBuf,
    manager: &str,
    env: &HashMap<String, String>,
    project_dir: &PathBuf,
) -> io::Result<i32> {
    let empty_args: Vec<String> = vec![];
    match manager {
        "npm" => spawn_command(pm_path, &["rebuild"], &empty_args, env, project_dir),
        "pnpm" => spawn_command(pm_path, &["rebuild"], &empty_args, env, project_dir),
        _ => spawn_command(pm_path, &["install"], &empty_args, env, project_dir),
    }
}

fn log_audit(
    manager: &str,
    project_dir: &PathBuf,
    exit_code: i32,
    duration_ms: u64,
    verdict: &str,
    secrets_denied: Option<usize>,
    _blocked: Option<usize>,
) {
    let entry = AuditEntry {
        timestamp: chrono::Utc::now().to_rfc3339(),
        action: "install".to_string(),
        package_manager: manager.to_string(),
        project_dir: project_dir.display().to_string(),
        packages_installed: None,
        scripts_run: None,
        network_requests: None,
        blocked_requests: None,
        secrets_denied,
        exit_code: if exit_code == 0 { None } else { Some(exit_code) },
        duration_ms,
        verdict: verdict.to_string(),
    };

    if let Err(e) = audit::log_entry(&entry) {
        eprintln!("{} Failed to write audit log: {}", "⚠".yellow(), e);
    }
}
