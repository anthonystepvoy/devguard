use crate::paths::{self, Severity};
use colored::*;
use regex::Regex;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub exposed_secrets: Vec<SecretFinding>,
    pub readable_tokens: Vec<TokenMatch>,
    pub risk_score: u32,
}

#[derive(Debug, Serialize)]
pub struct SecretFinding {
    pub path: String,
    pub name: String,
    pub description: String,
    pub severity: String,
    pub exists: bool,
    pub size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct TokenMatch {
    pub file: String,
    pub line_number: usize,
    pub line_snippet: String,
    pub token_type: String,
}

pub fn run_scan(json: bool, dir: Option<String>) -> Result<(), Box<dyn std::error::Error>> {
    let scan_dir = match dir {
        Some(d) => PathBuf::from(d),
        None => home::home_dir().ok_or("Cannot determine home directory")?,
    };

    let mut result = ScanResult {
        exposed_secrets: Vec::new(),
        readable_tokens: Vec::new(),
        risk_score: 0,
    };

    if !json {
        println!(
            "{} Scanning {} for exposed secrets...\n",
            "[*]".cyan().bold(),
            scan_dir.display().to_string().dimmed()
        );
    }

    scan_known_paths(&scan_dir, &mut result);
    scan_env_files(&scan_dir, &mut result);
    scan_shell_profiles(&scan_dir, &mut result);

    calculate_risk(&mut result);

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        print_results(&result);
    }

    Ok(())
}

fn scan_known_paths(scan_dir: &PathBuf, result: &mut ScanResult) {
    for secret in paths::known_secret_paths() {
        let rel = secret.path.strip_prefix(scan_dir).unwrap_or(&secret.path);
        let exists = secret.path.exists();
        let size = if exists {
            fs::metadata(&secret.path).map(|m| m.len()).unwrap_or(0)
        } else {
            0
        };

        if exists {
            result.risk_score += match secret.severity {
                Severity::Critical => 10,
                Severity::High => 5,
                Severity::Medium => 2,
                Severity::Low => 1,
            };
        }

        result.exposed_secrets.push(SecretFinding {
            path: rel.display().to_string(),
            name: secret.name.to_string(),
            description: secret.description.to_string(),
            severity: secret.severity.to_string(),
            exists,
            size_bytes: size,
        });
    }
}

fn scan_env_files(scan_dir: &PathBuf, result: &mut ScanResult) {
    let env_files = [
        ".env",
        ".env.local",
        ".env.development",
        ".env.production",
        ".env.test",
    ];

    for file in &env_files {
        let path = scan_dir.join(file);
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                scan_content_for_tokens(&path, &contents, result);
            }
        }
    }
}

fn scan_shell_profiles(scan_dir: &PathBuf, result: &mut ScanResult) {
    let profiles = [
        ".bashrc", ".bash_profile", ".bash_aliases",
        ".zshrc", ".zprofile", ".zshenv",
        "profile.ps1",
        "config.fish",
    ];

    for profile in &profiles {
        let path = scan_dir.join(profile);
        if path.exists() {
            if let Ok(contents) = fs::read_to_string(&path) {
                scan_content_for_tokens(&path, &contents, result);
            }
        }
    }
}

fn scan_content_for_tokens(path: &PathBuf, contents: &str, result: &mut ScanResult) {
    let patterns: &[(&str, &str)] = &[
        (r#"(?i)TOKEN\s*=\s*['"][^'"]{8,}['"]"#, "TOKEN in quotes"),
        (r#"(?i)SECRET\s*=\s*['"][^'"]{8,}['"]"#, "SECRET in quotes"),
        (r#"(?i)PASSWORD\s*=\s*['"][^'"]{8,}['"]"#, "PASSWORD in quotes"),
        (r#"(?i)API_KEY\s*=\s*['"][^'"]{8,}['"]"#, "API_KEY in quotes"),
        (r#"_authToken\s*=\s*\S{8,}"#, "npm _authToken"),
        (r"//registry\.npmjs\.org/:_authToken\s*=\s*(\S+)", "npm registry token"),
        (r"ghp_[a-zA-Z0-9]{36}", "GitHub personal access token"),
        (r"github_pat_[a-zA-Z0-9_]{22,}", "GitHub fine-grained token"),
        (r"gho_[a-zA-Z0-9]{36}", "GitHub OAuth token"),
        (r"ghu_[a-zA-Z0-9]{36}", "GitHub user token"),
        (r"ghs_[a-zA-Z0-9]{36}", "GitHub server token"),
        (r"ghr_[a-zA-Z0-9]{36}", "GitHub refresh token"),
        (r"AKIA[0-9A-Z]{16}", "AWS Access Key ID"),
        (r"sk-ant-[a-zA-Z0-9]{20,}", "Anthropic API key"),
        (r"sk-(?!ant-)[a-zA-Z0-9]{20,}", "OpenAI API key"),
        (r"AIza[0-9A-Za-z\-_]{35}", "Google API key"),
    ];

    let compiled: Vec<(Regex, &str)> = patterns
        .iter()
        .filter_map(|(p, t)| Regex::new(p).ok().map(|re| (re, *t)))
        .collect();

    for (re, token_type) in &compiled {
        for (line_number, line) in contents.lines().enumerate() {
            if re.is_match(line) {
                let snippet = if line.len() > 120 {
                    format!("{}...", &line[..117])
                } else {
                    line.to_string()
                };

                result.readable_tokens.push(TokenMatch {
                    file: path.display().to_string(),
                    line_number: line_number + 1,
                    line_snippet: snippet,
                    token_type: token_type.to_string(),
                });
            }
        }
    }
}

fn calculate_risk(result: &mut ScanResult) {
    result.risk_score += (result.readable_tokens.len() as u32) * 3;
    if result.risk_score > 100 {
        result.risk_score = 100;
    }
}

fn print_results(result: &ScanResult) {
    let risk_label = match result.risk_score {
        0..=10 => "LOW".green(),
        11..=30 => "MEDIUM".yellow(),
        31..=60 => "HIGH".red(),
        _ => "CRITICAL".red().bold(),
    };

    println!("{} Risk score: {}/100", "→".bold(), risk_label);
    println!();

    println!("{} Known secret paths:", "---".bold());
    let existing: Vec<_> = result.exposed_secrets.iter().filter(|s| s.exists).collect();
    if existing.is_empty() {
        println!("  {} (no exposed secret files found)", "none".dimmed());
    } else {
        for finding in &existing {
            let icon = match finding.severity.as_str() {
                "CRITICAL" => "!".red().bold(),
                "HIGH" => "⚠".yellow(),
                "MEDIUM" => "•".dimmed(),
                _ => "·".dimmed(),
            };
            println!(
                "  {} {} — {} ({})",
                icon,
                finding.path,
                finding.description,
                format_size(finding.size_bytes)
            );
        }
    }

    println!();
    println!("{} Token patterns found in files:", "---".bold());
    if result.readable_tokens.is_empty() {
        println!("  {} (no token patterns detected)", "none".dimmed());
    } else {
        for token in &result.readable_tokens {
            println!(
                "  {} {}:{} — {}",
                "→".yellow(),
                token.file,
                token.line_number,
                token.token_type
            );
        }
    }
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
