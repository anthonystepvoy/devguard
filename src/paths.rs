use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecretPath {
    pub path: PathBuf,
    pub name: &'static str,
    pub description: &'static str,
    pub severity: Severity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Critical,
    High,
    Medium,
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Severity::Critical => write!(f, "CRITICAL"),
            Severity::High => write!(f, "HIGH"),
            Severity::Medium => write!(f, "MEDIUM"),
        }
    }
}

pub fn known_secret_paths_in(home: &Path) -> Vec<SecretPath> {
    vec![
        SecretPath {
            path: home.join(".npmrc"),
            name: ".npmrc",
            description: "npm auth token",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".ssh"),
            name: ".ssh",
            description: "SSH private keys and config",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".aws").join("credentials"),
            name: ".aws/credentials",
            description: "AWS access keys",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".aws").join("config"),
            name: ".aws/config",
            description: "AWS configuration",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".config").join("gh").join("hosts.yml"),
            name: ".config/gh/hosts.yml",
            description: "GitHub CLI tokens",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".docker").join("config.json"),
            name: ".docker/config.json",
            description: "Docker registry credentials",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".kube").join("config"),
            name: ".kube/config",
            description: "Kubernetes cluster credentials",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".git-credentials"),
            name: ".git-credentials",
            description: "Git HTTP credentials",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".netrc"),
            name: ".netrc",
            description: "Network auto-login tokens",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".bash_history"),
            name: ".bash_history",
            description: "Shell history (may contain tokens)",
            severity: Severity::Medium,
        },
        SecretPath {
            path: home.join(".zsh_history"),
            name: ".zsh_history",
            description: "Shell history (may contain tokens)",
            severity: Severity::Medium,
        },
        SecretPath {
            path: home.join(".env"),
            name: ".env (home)",
            description: "Environment variables at home level",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".config").join("gcloud"),
            name: ".config/gcloud",
            description: "Google Cloud SDK credentials",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".azure"),
            name: ".azure",
            description: "Azure CLI credentials",
            severity: Severity::Critical,
        },
        SecretPath {
            path: home.join(".terraform.d"),
            name: ".terraform.d",
            description: "Terraform credentials and plugins",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".cargo").join("credentials.toml"),
            name: ".cargo/credentials.toml",
            description: "Cargo registry tokens",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".pypirc"),
            name: ".pypirc",
            description: "PyPI upload tokens",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".gem").join("credentials"),
            name: ".gem/credentials",
            description: "RubyGems API keys",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".config").join("pnpm"),
            name: ".config/pnpm",
            description: "pnpm config (may contain auth)",
            severity: Severity::Medium,
        },
        SecretPath {
            path: home.join(".yarnrc"),
            name: ".yarnrc",
            description: "Yarn config (may contain auth)",
            severity: Severity::High,
        },
        SecretPath {
            path: home.join(".yarnrc.yml"),
            name: ".yarnrc.yml",
            description: "Yarn 2+ config (may contain auth)",
            severity: Severity::High,
        },
    ]
}

pub fn sandbox_home_paths() -> Vec<(PathBuf, &'static str)> {
    vec![
        (PathBuf::from(".npm"), "npm cache"),
        (PathBuf::from(".cache"), "general cache"),
        (PathBuf::from(".local"), "local data"),
        (PathBuf::from(".node-gyp"), "node-gyp cache"),
        (PathBuf::from(".electron-gyp"), "electron-gyp cache"),
        (PathBuf::from(".ccache"), "ccache"),
        (PathBuf::from(".sccache"), "sccache"),
        (
            PathBuf::from(".cargo").join("registry"),
            "cargo registry cache",
        ),
        (PathBuf::from(".cargo").join("git"), "cargo git checkouts"),
        (PathBuf::from(".pnpm-store"), "pnpm global store"),
        (PathBuf::from(".yarn"), "yarn cache"),
        (
            PathBuf::from(".bun").join("install").join("cache"),
            "bun cache",
        ),
    ]
}

pub fn secret_env_var_patterns() -> Vec<&'static str> {
    vec![
        "TOKEN",
        "SECRET",
        "PASSWORD",
        "PASSWD",
        "API_KEY",
        "APIKEY",
        "PRIVATE_KEY",
        "ACCESS_KEY",
        "CREDENTIAL",
        "AUTH",
        "NPM_TOKEN",
        "NODE_AUTH_TOKEN",
        "GITHUB_TOKEN",
        "GH_TOKEN",
        "GITLAB_TOKEN",
        "AWS_ACCESS_KEY_ID",
        "AWS_SECRET_ACCESS_KEY",
        "AWS_SESSION_TOKEN",
        "OPENAI_API_KEY",
        "ANTHROPIC_API_KEY",
        "GOOGLE_API_KEY",
        "AZURE",
        "GCLOUD",
        "GOOGLE_APPLICATION_CREDENTIALS",
        "DOCKER_PASSWORD",
        "DOCKER_AUTH",
        "REGISTRY",
        "NPMRC",
        "YARN",
        "PNPM",
        "BUN",
        "CI_JOB_TOKEN",
        "JFROG",
        "ARTIFACTORY",
        "NEXUS",
        "GRAFANA",
        "DATADOG",
        "SENTRY_AUTH_TOKEN",
        "HEROKU",
        "VERCEL",
        "NETLIFY",
        "RENDER",
    ]
}

pub fn auth_pass_through_paths() -> Vec<&'static str> {
    vec![
        ".npmrc",
        ".yarnrc",
        ".yarnrc.yml",
        ".pnpmrc",
        ".bunfig.toml",
    ]
}

#[allow(dead_code)]
pub fn project_git_config() -> Vec<&'static str> {
    vec![".gitconfig"]
}

pub fn is_secret_env_var(key: &str) -> bool {
    let upper = key.to_uppercase();
    secret_env_var_patterns()
        .iter()
        .any(|pattern| upper.contains(pattern))
}
