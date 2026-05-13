use crate::audit::{self, AuditEntry};
use crate::env;
use crate::network;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;

struct SandboxEnv {
    vars: HashMap<String, String>,
    withheld_count: usize,
}

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
            log_audit(
                manager,
                &project_dir,
                phase1_code,
                duration.as_millis() as u64,
                "download-failed",
                None,
            );
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

        env::print_env_summary(sandbox_env.vars.len(), sandbox_env.withheld_count);
        network::print_network_policy(allow_network);

        println!(
            "{} Sandbox HOME: {}",
            "→".bold(),
            sandbox_home.display().to_string().dimmed()
        );
        print_sandbox_wall();

        println!();
        println!("{} Running {} rebuild...", "[*]".cyan().bold(), manager);

        let phase2_code = spawn_rebuild_phase(&pm_path, manager, &sandbox_env.vars, &project_dir)?;
        let duration = start.elapsed();
        let env_vars_withheld = sandbox_env.withheld_count;

        if phase2_code == 0 {
            println!();
            println!(
                "{} Install complete {}",
                "✓".green().bold(),
                audit::format_duration(duration.as_millis() as u64).dimmed()
            );

            println!();
            println!("{} Scripts ran with HOME/env isolation", "🔒".bold());

            log_audit(
                manager,
                &project_dir,
                0,
                duration.as_millis() as u64,
                "clean",
                Some(env_vars_withheld),
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
                Some(env_vars_withheld),
            );
        }
    } else {
        let real_home = home::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        let sandbox_home = create_union_home(&real_home)?;
        let sandbox_env = build_sandbox_env(&sandbox_home);

        env::print_env_summary(sandbox_env.vars.len(), sandbox_env.withheld_count);
        network::print_network_policy(allow_network);

        println!(
            "{} Union HOME: {}",
            "→".bold(),
            sandbox_home.display().to_string().dimmed()
        );
        print_sandbox_wall_union();

        println!();
        println!("{} Running {} install...", "[*]".cyan().bold(), manager);

        let exit_code =
            spawn_install_phase(&pm_path, manager, args, &sandbox_env.vars, &project_dir)?;
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
            Some(sandbox_env.withheld_count),
        );
    }

    if !allow_network {
        println!();
        println!(
            "{} {}",
            "⊡".bold(),
            "Network policy is advisory in this version; OS-level enforcement coming in v0.2."
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
        "network: advisory".yellow()
    };
    println!("{} {} {}", "║".green(), "manager:".dimmed(), manager);
    println!("{} {}", "║".green(), mode);
    println!("{}", "╚══════════════════════════╝".green());
}

fn print_sandbox_wall() {
    println!();
    println!("{} Isolation layer:", "---".bold());
    println!("  ✓ HOME → temp directory (shell path resolution redirected)");
    println!("  ✓ ~/.npmrc hidden from HOME-based lookups");
    println!("  ✓ ~/.ssh hidden via HOME redirect");
    println!("  ✓ ~/.aws hidden via HOME redirect");
    println!("  ✓ ~/.config/gh hidden via HOME redirect");
    println!("  ✓ ~/.docker hidden via HOME redirect");
    println!("  ✓ ~/.kube hidden via HOME redirect");
    println!("  ✓ SSH agent disconnected");
    println!("  ✓ Most environment variables withheld by default");
    println!("  ⚠ Absolute-path bypass possible for all home files (OS enforcement in v0.2)");
}

fn print_sandbox_wall_union() {
    println!();
    println!("{} Isolation layer:", "---".bold());
    println!("  ✓ HOME → temp directory (shell path resolution redirected)");
    println!("  ✓ ~/.ssh hidden via HOME redirect");
    println!("  ✓ ~/.aws hidden via HOME redirect");
    println!("  ✓ ~/.config/gh hidden via HOME redirect");
    println!("  ✓ ~/.docker hidden via HOME redirect");
    println!("  ✓ ~/.kube hidden via HOME redirect");
    println!("  ✓ SSH agent disconnected");
    println!("  ✓ Most environment variables withheld by default");
    println!("  ⚠ .npmrc pass-through enabled (needed for private registries)");
    println!("  ⚠ Postinstall scripts may read .npmrc (all other secrets hidden)");
    println!("  ⚠ Absolute-path bypass possible (OS enforcement in v0.2)");
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
    clean_old_sandboxes();

    let sandbox = tempfile::Builder::new()
        .prefix("devguard-")
        .tempdir()?
        .keep();

    for (subdir, _label) in crate::paths::sandbox_home_paths() {
        let target = sandbox.join(&subdir);
        fs::create_dir_all(&target).ok();
    }

    Ok(sandbox)
}

fn clean_old_sandboxes() {
    let temp = std::env::temp_dir();
    if let Ok(entries) = fs::read_dir(&temp) {
        let now = std::time::SystemTime::now();
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("devguard-")
                && let Ok(meta) = entry.metadata()
                && let Ok(age) = now.duration_since(meta.modified().unwrap_or(now))
                && age.as_secs() > 3600
            {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
}

fn create_union_home(real_home: &Path) -> io::Result<PathBuf> {
    let sandbox = create_sandbox_home()?;

    for filename in crate::paths::auth_pass_through_paths() {
        let src = real_home.join(filename);
        if src.exists() {
            let dst = sandbox.join(filename);
            if let Some(parent) = dst.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&src, &dst).ok();
        }
    }

    Ok(sandbox)
}

fn build_sandbox_env(sandbox_home: &Path) -> SandboxEnv {
    let original_count = std::env::vars().count();
    let mut env = env::sanitize_for_sandbox();
    let withheld_count = original_count.saturating_sub(env.len());

    let home_str = sandbox_home.display().to_string();
    env.insert("HOME".to_string(), home_str.clone());
    env.insert("USERPROFILE".to_string(), home_str.clone());

    if cfg!(target_os = "windows") {
        if let Some(drive) = home_str.chars().next() {
            env.insert("HOMEDRIVE".to_string(), format!("{}:", drive));
        }
        let home_relative = home_str
            .strip_prefix(&format!("{}:", home_str.chars().next().unwrap_or('C')))
            .unwrap_or(&home_str)
            .to_string();
        env.insert("HOMEPATH".to_string(), home_relative);
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
    }

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

    SandboxEnv {
        vars: env,
        withheld_count,
    }
}

fn spawn_command(
    pm_path: &Path,
    cmds: &[&str],
    args: &[String],
    env: &HashMap<String, String>,
    project_dir: &Path,
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
    pm_path: &Path,
    manager: &str,
    args: &[String],
    project_dir: &Path,
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
    pm_path: &Path,
    manager: &str,
    args: &[String],
    env: &HashMap<String, String>,
    project_dir: &Path,
) -> io::Result<i32> {
    let install_cmd = match manager {
        "bun" => vec!["add"],
        _ => vec!["install"],
    };

    spawn_command(pm_path, &install_cmd, args, env, project_dir)
}

fn spawn_rebuild_phase(
    pm_path: &Path,
    manager: &str,
    env: &HashMap<String, String>,
    project_dir: &Path,
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
    project_dir: &Path,
    exit_code: i32,
    duration_ms: u64,
    verdict: &str,
    env_vars_withheld: Option<usize>,
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
        env_vars_withheld,
        exit_code: if exit_code == 0 {
            None
        } else {
            Some(exit_code)
        },
        duration_ms,
        verdict: verdict.to_string(),
    };

    if let Err(e) = audit::log_entry(&entry) {
        eprintln!("{} Failed to write audit log: {}", "⚠".yellow(), e);
    }
}
