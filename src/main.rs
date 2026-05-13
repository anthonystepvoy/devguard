mod audit;
mod env;
mod network;
mod paths;
mod sandbox;
mod scanner;

use clap::{Parser, Subcommand};
use colored::*;
use std::process;

#[derive(Parser)]
#[command(name = "devguard", about = "Developer token firewall", version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    #[command(about = "Scan for exposed tokens and secrets")]
    Scan {
        #[arg(short, long, help = "Output as JSON")]
        json: bool,
        #[arg(long, help = "Directory to scan (default: home directory)")]
        dir: Option<String>,
    },
    #[command(about = "Run a sandboxed package install")]
    Install {
        #[arg(help = "Package manager to use (npm, pnpm, yarn, bun)")]
        manager: String,
        #[arg(
            short = 'e',
            long,
            help = "Extra args to pass to the package manager",
            allow_hyphen_values = true
        )]
        args: Vec<String>,
        #[arg(short = 'n', long, help = "Allow full network access")]
        allow_network: bool,
        #[arg(long, help = "Project directory (default: current dir)")]
        cwd: Option<String>,
    },
    #[command(about = "Show recent audit log entries")]
    AuditLog {
        #[arg(short, long, default_value = "20", help = "Number of entries")]
        lines: usize,
    },
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Scan { json, dir } => scanner::run_scan(json, dir),
        Command::Install {
            manager,
            args,
            allow_network,
            cwd,
        } => sandbox::run_install(&manager, &args, allow_network, cwd.as_deref()),
        Command::AuditLog { lines } => audit::show_recent(lines),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "error:".red().bold(), e);
        process::exit(1);
    }
}
