use crate::paths;
use colored::Colorize;
use std::collections::HashMap;

pub fn sanitize_for_sandbox() -> HashMap<String, String> {
    let mut env: HashMap<String, String> = HashMap::new();

    for (key, value) in std::env::vars() {
        if is_allowed_sandbox_env(&key) && !paths::is_secret_env_var(&key) {
            env.insert(key, value);
        }
    }

    env
}

pub fn print_env_summary(passed_count: usize, withheld_count: usize) {
    println!(
        "{} Environment: {} vars passed, {} vars withheld",
        "→".bold(),
        passed_count,
        withheld_count.to_string().yellow()
    );
}

fn is_allowed_sandbox_env(key: &str) -> bool {
    let upper = key.to_uppercase();
    matches!(
        upper.as_str(),
        "PATH"
            | "PATHEXT"
            | "SYSTEMROOT"
            | "WINDIR"
            | "COMSPEC"
            | "TEMP"
            | "TMP"
            | "TMPDIR"
            | "USER"
            | "USERNAME"
            | "LOGNAME"
            | "LANG"
            | "LC_ALL"
            | "LC_CTYPE"
            | "TERM"
            | "SHELL"
            | "COLORTERM"
            | "NO_COLOR"
            | "FORCE_COLOR"
            | "CI"
            | "OS"
            | "PROCESSOR_ARCHITECTURE"
            | "PROCESSOR_IDENTIFIER"
            | "NUMBER_OF_PROCESSORS"
            | "PROGRAMFILES"
            | "PROGRAMFILES(X86)"
            | "PROGRAMW6432"
    ) || upper.starts_with("RUST_")
}

#[cfg(test)]
mod tests {
    use super::is_allowed_sandbox_env;
    use crate::paths::is_secret_env_var;

    #[test]
    fn common_runtime_vars_are_allowed() {
        assert!(is_allowed_sandbox_env("PATH"));
        assert!(is_allowed_sandbox_env("SystemRoot"));
        assert!(is_allowed_sandbox_env("TMP"));
    }

    #[test]
    fn common_secret_names_are_detected() {
        assert!(is_secret_env_var("OPENAI_API_KEY"));
        assert!(is_secret_env_var("ANTHROPIC_API_KEY"));
        assert!(is_secret_env_var("GOOGLE_API_KEY"));
        assert!(is_secret_env_var("MY_SERVICE_TOKEN"));
        assert!(is_secret_env_var("AWS_SECRET_ACCESS_KEY"));
    }
}
